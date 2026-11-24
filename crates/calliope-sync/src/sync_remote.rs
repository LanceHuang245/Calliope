//! Sync sees plaintext paths and bytes. Only this boundary uses ciphertext
//! paths and payloads, while versions come from the stored objects.

use std::collections::HashMap;

use opendal::{ErrorKind, Operator};

use crate::{
    Config, Error, Provider, Result,
    crypt::Crypt,
    excluded,
    remote::{self, Version, retry},
};

// rclone skips this non-base32 name. Its nonempty authenticated payload
// verifies the passphrase even after the last synced file has been deleted.
pub(crate) const VERIFIER: &str = ".calliope-crypt";
const STAGING_PREFIX: &str = ".calliope-crypt-setup-";
const PROOF: &[u8] = b"Calliope encrypted remote v1";

pub(crate) struct Remote {
    op: Operator,
    crypt: Option<Crypt>,
}

impl Remote {
    pub(crate) async fn connect(
        config: &Config,
        secret: &str,
        passphrase: &str,
        allow_new: bool,
        initialize: bool,
    ) -> Result<Self> {
        let op = remote::operator(config, secret).await?;
        if !config.encrypted {
            return Ok(Self::plain(op));
        }
        let crypt = Crypt::new(passphrase)?;
        if !verify(&op, &crypt).await? {
            if !allow_new {
                return Err(Error::UninitializedRemote);
            }
            if remote::entries(&op)
                .await?
                .iter()
                .any(|e| !e.path().starts_with(STAGING_PREFIX))
            {
                return Err(Error::RemoteNotEmpty);
            }
            if initialize {
                let data = crypt.seal(PROOF)?;
                if config.provider == Provider::WebDav {
                    publish_webdav_verifier(&op, config, secret, data).await?;
                } else {
                    // If another device initialized this path, verify its
                    // verifier below and proceed only with the same passphrase.
                    match retry(|| op.write_with(VERIFIER, data.clone()).if_not_exists(true)).await
                    {
                        Ok(_) => {}
                        Err(e)
                            if matches!(
                                e.kind(),
                                ErrorKind::AlreadyExists | ErrorKind::ConditionNotMatch
                            ) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
                if !verify(&op, &crypt).await? {
                    return Err(Error::UninitializedRemote);
                }
            }
        }
        Ok(Self {
            op,
            crypt: Some(crypt),
        })
    }

    #[cfg(test)]
    pub(crate) fn identity(&self) -> String {
        format!("{} {}", self.op.info().scheme(), self.op.info().root())
    }

    pub(crate) fn plain(op: Operator) -> Self {
        Self { op, crypt: None }
    }

    #[cfg(test)]
    pub(crate) async fn encrypted_fs(op: Operator, passphrase: &str) -> Result<Self> {
        let crypt = Crypt::new(passphrase)?;
        op.write(VERIFIER, crypt.seal(PROOF)?).await?;
        Ok(Self {
            op,
            crypt: Some(crypt),
        })
    }

    pub(crate) async fn list(&self) -> Result<HashMap<String, Version>> {
        let mut files = HashMap::new();
        for entry in remote::entries(&self.op).await? {
            if entry.path() == VERIFIER {
                if self.crypt.is_none() {
                    return Err(Error::EncryptedRemote);
                }
                continue;
            }
            if !entry.metadata().is_file() || entry.path().starts_with(STAGING_PREFIX) {
                continue;
            }
            let path = match &self.crypt {
                Some(crypt) => crypt.decode_path(entry.path())?,
                None => entry.path().to_owned(),
            };
            if !excluded(&path) && files.insert(path, Version::of(entry.metadata())).is_some() {
                return Err(Error::InvalidCiphertext);
            }
        }
        Ok(files)
    }

    fn stored_path(&self, path: &str) -> Result<String> {
        match &self.crypt {
            Some(crypt) => crypt.encode_path(path),
            None => Ok(path.to_owned()),
        }
    }

    pub(crate) async fn read(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.stored_path(path)?;
        let data = retry(|| self.op.read(&path)).await?.to_vec();
        match &self.crypt {
            Some(crypt) => crypt.open(&data),
            None => Ok(data),
        }
    }

    pub(crate) async fn write(&self, path: &str, data: Vec<u8>) -> Result<Version> {
        let path = self.stored_path(path)?;
        let data = match &self.crypt {
            Some(crypt) => crypt.seal(&data)?,
            None => data,
        };
        retry(|| self.op.write(&path, data.clone())).await?;
        Ok(Version::of(&retry(|| self.op.stat(&path)).await?))
    }

    pub(crate) async fn version(&self, path: &str) -> Result<Version> {
        let path = self.stored_path(path)?;
        Ok(Version::of(&retry(|| self.op.stat(&path)).await?))
    }

    pub(crate) async fn delete(&self, path: &str) -> Result<()> {
        let path = self.stored_path(path)?;
        Ok(retry(|| self.op.delete(&path)).await?)
    }
}

async fn verify(op: &Operator, crypt: &Crypt) -> Result<bool> {
    match retry(|| op.read(VERIFIER)).await {
        Ok(data) if crypt.open(&data.to_vec()).is_ok_and(|bytes| bytes == PROOF) => Ok(true),
        Ok(_) => Err(Error::WrongPassphrase),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// WebDAV servers may ignore conditional PUT. Publish the verifier using
/// MOVE with Overwrite: F so a concurrent setup cannot replace it.
async fn publish_webdav_verifier(
    op: &Operator,
    config: &Config,
    secret: &str,
    data: Vec<u8>,
) -> Result<()> {
    let staged = format!(
        "{STAGING_PREFIX}{}",
        data_encoding::HEXLOWER.encode(&data[8..32])
    );
    let mut root = reqwest::Url::parse(config.endpoint.trim()).map_err(std::io::Error::other)?;
    root.path_segments_mut()
        .map_err(|_| std::io::Error::other("invalid WebDAV endpoint"))?
        .pop_if_empty()
        .extend(op.info().root().split('/').filter(|part| !part.is_empty()))
        .push("");
    let source = root.join(&staged).map_err(std::io::Error::other)?;
    let target = root.join(VERIFIER).map_err(std::io::Error::other)?;
    let client = remote::client(config)?;

    retry(|| op.write(&staged, data.clone())).await?;
    let response = client
        .request(
            reqwest::Method::from_bytes(b"MOVE").expect("WebDAV method"),
            source,
        )
        .basic_auth(&config.user, Some(secret))
        .header("Destination", target.as_str())
        .header("Overwrite", "F")
        .send()
        .await;
    if let Err(e) = retry(|| op.delete(&staged)).await {
        log::warn!("could not remove temporary encryption verifier: {e}");
    }
    let response = response.map_err(std::io::Error::other)?;
    if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
        return Ok(());
    }
    response.error_for_status().map_err(std::io::Error::other)?;
    Ok(())
}
