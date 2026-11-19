//! The remote side: an OpenDAL operator for a [`Config`], and the files it
//! holds with what identifies each one's version.

use std::{
    collections::HashMap,
    future::IntoFuture,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use opendal::{
    ErrorKind, HttpTransporter, Metadata, OperationContext, Operator,
    services::{S3, Webdav},
};
use opendal_http_transport_reqwest::ReqwestTransport;
use serde::{Deserialize, Serialize};

use crate::{Config, Provider, Result, excluded};

/// A remote file's version: its ETag when the server gives one, else its
/// modification time (in nanoseconds) and size.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Version {
    pub etag: Option<String>,
    pub modified: Option<i64>,
    pub size: u64,
}

impl Version {
    pub(crate) fn of(meta: &Metadata) -> Self {
        // `W/"abc"` and `"abc"` name the same version.
        let etag = meta
            .etag()
            .map(|e| e.trim_start_matches("W/").trim_matches('"'));
        let modified = (meta.last_modified())
            .and_then(|t| SystemTime::from(t).duration_since(UNIX_EPOCH).ok());
        Self {
            etag: etag.map(str::to_owned),
            modified: modified.map(|d| d.as_nanos() as i64),
            size: meta.content_length(),
        }
    }

    /// Whether `self` and `other` are the same version of a file.
    pub(crate) fn same(&self, other: &Version) -> bool {
        match (&self.etag, &other.etag) {
            (Some(a), Some(b)) => a == b,
            _ => self.modified == other.modified && self.size == other.size,
        }
    }
}

/// The operator for the storage `config` names, signed in with `secret`
/// (the WebDAV password or the S3 secret key).
pub(crate) fn operator(config: &Config, secret: &str) -> Result<Operator> {
    let op = match config.provider {
        Provider::WebDav => Operator::new(
            Webdav::default()
                .endpoint(&config.endpoint)
                .username(&config.user)
                .password(secret)
                .root(&config.root),
        )?,
        Provider::S3 => {
            // Only what is configured here: no credentials from the
            // environment, the AWS profile or the EC2 metadata service.
            let s3 = S3::default()
                .endpoint(&config.endpoint)
                .region(&config.region)
                .bucket(&config.bucket)
                .access_key_id(&config.user)
                .secret_access_key(secret)
                .root(&config.root)
                .disable_config_load()
                .disable_ec2_metadata();
            Operator::new(match config.path_style {
                true => s3,
                false => s3.enable_virtual_host_style(),
            })?
        }
        Provider::Off => {
            return Err(opendal::Error::new(ErrorKind::ConfigInvalid, "sync is off").into());
        }
    };
    let http = HttpTransporter::new(ReqwestTransport::new(client(config)?));
    Ok(op.with_context(OperationContext::new().with_http_transport(http)))
}

/// The HTTP client: TLS as [`crate::tls`] trusts, and timeouts so a dead
/// network fails the run.
fn client(config: &Config) -> Result<reqwest::Client> {
    let unexpected =
        |e: &dyn std::fmt::Display| opendal::Error::new(ErrorKind::Unexpected, e.to_string());
    let tls = crate::tls::config(&config.certificate).map_err(|e| unexpected(&e))?;
    reqwest::Client::builder()
        .tls_backend_preconfigured(tls)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| unexpected(&e).into())
}

/// Retries rate-limited (HTTP 429) or temporary remote errors with exponential
/// backoff (1s, 2s, 4s), so WebDAV providers like Jianguoyun do not abort a run.
pub(crate) async fn retry<T, F, Fut>(mut op: F) -> opendal::Result<T>
where
    F: FnMut() -> Fut,
    Fut: IntoFuture<Output = opendal::Result<T>>,
{
    let mut delay = Duration::from_secs(1);
    for attempt in 0..4 {
        match op().await {
            Ok(value) => return Ok(value),
            Err(e) if attempt < 3 && is_retryable(&e) => {
                log::warn!("sync: transient remote error, retrying in {delay:?}: {e}");
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

fn is_retryable(e: &opendal::Error) -> bool {
    e.kind() == ErrorKind::RateLimited || e.is_temporary() || e.to_string().contains("status: 429")
}

/// Every remote file that syncs, by path; none while the vault's folder is
/// not there yet.
pub(crate) async fn list(op: &Operator) -> Result<HashMap<String, Version>> {
    let entries = match retry(|| op.list_with("").recursive(true)).await {
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
        entries => entries?,
    };
    Ok((entries.into_iter())
        .filter(|e| e.metadata().is_file() && !excluded(e.path()))
        .map(|e| (e.path().to_owned(), Version::of(e.metadata())))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_rate_limits_and_temporary_errors_only() {
        let rate_limited = opendal::Error::new(ErrorKind::RateLimited, "too many requests");
        let temp = opendal::Error::new(ErrorKind::Unexpected, "timeout").set_temporary();
        let http_429 = opendal::Error::new(ErrorKind::Unexpected, "status: 429");
        let not_found = opendal::Error::new(ErrorKind::NotFound, "missing");
        let denied = opendal::Error::new(ErrorKind::PermissionDenied, "status: 403");

        assert!(is_retryable(&rate_limited));
        assert!(is_retryable(&temp));
        assert!(is_retryable(&http_429));
        assert!(!is_retryable(&not_found));
        assert!(!is_retryable(&denied));
    }
}
