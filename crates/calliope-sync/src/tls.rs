//! Whom sync trusts over HTTPS: the certificates the system trusts (on
//! Android Mozilla's, as the system's need a Java part nsh cannot add), and
//! the one certificate the user pinned by its SHA-256 fingerprint, for a
//! server with a certificate of its own (a NAS's self-signed one).

use std::sync::Arc;

use rustls::{
    ClientConfig, DigitallySignedStruct, Error, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};

/// TLS on ring, trusting the system's certificates and the one of
/// `fingerprint` (any case, with or without colons; empty for none).
pub(crate) fn config(fingerprint: &str) -> Result<ClientConfig, Error> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = Pinned {
        system: system(provider.clone())?,
        pin: plain(fingerprint),
    };
    Ok(ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

#[cfg(not(target_os = "android"))]
fn system(provider: Arc<CryptoProvider>) -> Result<Arc<dyn ServerCertVerifier>, Error> {
    Ok(Arc::new(rustls_platform_verifier::Verifier::new(provider)?))
}

#[cfg(target_os = "android")]
fn system(provider: Arc<CryptoProvider>) -> Result<Arc<dyn ServerCertVerifier>, Error> {
    let roots = Arc::new(rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    });
    let verifier = rustls::client::WebPkiServerVerifier::builder_with_provider(roots, provider)
        .build()
        .map_err(|e| Error::General(e.to_string()))?;
    Ok(verifier)
}

/// A certificate's SHA-256 fingerprint as browsers show it (`AB:CD:…`).
fn fingerprint(cert: &[u8]) -> String {
    let hex: Vec<String> = Sha256::digest(cert)
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    hex.join(":")
}

/// Only the hex digits of `fingerprint`, upper case.
fn plain(fingerprint: &str) -> String {
    (fingerprint.chars())
        .filter(char::is_ascii_hexdigit)
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// The system's verifier, but for the pinned certificate; a certificate it
/// refuses gets its fingerprint in the error, for the user to check and pin.
#[derive(Debug)]
struct Pinned {
    system: Arc<dyn ServerCertVerifier>,
    pin: String,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let fingerprint = fingerprint(end_entity);
        if !self.pin.is_empty() && plain(&fingerprint) == self.pin {
            return Ok(ServerCertVerified::assertion());
        }
        (self.system)
            .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
            .map_err(|e| {
                log::warn!("{server_name:?}: {e}");
                Error::General(format!(
                    "the server's certificate is not trusted; its SHA-256 fingerprint is \
                     {fingerprint}"
                ))
            })
    }

    // The handshake's signatures are checked as the system's verifier does,
    // pinned certificate or not.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.system.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        self.system.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.system.supported_verify_schemes()
    }
}
