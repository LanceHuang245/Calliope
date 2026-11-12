//! Sync passwords and secret keys, one per account (a vault): kept in the
//! system's secure storage (Windows' Credential Manager, the Keychain on
//! macOS and iOS, the Secret Service on Linux). Where there is none, in
//! `secrets.json` in the app's data directory, which only the app reads:
//! on Android (nsh does not let an app reach its Keystore yet) and on a
//! Linux desktop without GNOME Keyring or KWallet.

use std::{collections::HashMap, fs, io};

use crate::paths;

const SERVICE: &str = "Calliope";

/// Keeps `secret` for `account`: in secure storage, else in the file.
pub(crate) fn set(account: &str, secret: &str) -> io::Result<()> {
    let mut file = read_file();
    let in_file = !system::set(account, secret);
    // Never left in the file once secure storage holds it.
    match in_file {
        true => file.insert(account.to_owned(), secret.to_owned()),
        false => file.remove(account),
    };
    write_file(&file)
}

pub(crate) fn get(account: &str) -> Option<String> {
    system::get(account).or_else(|| read_file().remove(account))
}

/// Whether the secret of `account` is in the file.
pub(crate) fn in_file(account: &str) -> bool {
    read_file().contains_key(account)
}

fn read_file() -> HashMap<String, String> {
    (paths::data_dir().and_then(|dir| fs::read(dir.join("secrets.json")).ok()))
        .and_then(|json| serde_json::from_slice(&json).ok())
        .unwrap_or_default()
}

fn write_file(secrets: &HashMap<String, String>) -> io::Result<()> {
    let dir = paths::data_dir().ok_or_else(|| io::Error::other("no data directory"))?;
    fs::create_dir_all(&dir)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    // Only its user may read it.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let json = serde_json::to_vec(secrets).map_err(io::Error::other)?;
    io::Write::write_all(&mut options.open(dir.join("secrets.json"))?, &json)
}

#[cfg(not(target_os = "android"))]
mod system {
    use std::sync::{Arc, OnceLock};

    use keyring_core::{CredentialStore, Entry};

    use super::SERVICE;

    /// Whether `secret` is now in secure storage.
    pub(super) fn set(account: &str, secret: &str) -> bool {
        entry(account)
            .and_then(|e| {
                e.set_password(secret)
                    .inspect_err(|e| log::warn!("keyring: {e}"))
                    .ok()
            })
            .is_some()
    }

    pub(super) fn get(account: &str) -> Option<String> {
        entry(account)?.get_password().ok()
    }

    fn entry(account: &str) -> Option<Entry> {
        static STORE: OnceLock<Option<Arc<CredentialStore>>> = OnceLock::new();
        let store = STORE.get_or_init(|| {
            native()
                .inspect_err(|e| log::warn!("no secure storage: {e}"))
                .ok()
        });
        store.as_ref()?.build(SERVICE, account, None).ok()
    }

    fn native() -> keyring_core::Result<Arc<CredentialStore>> {
        #[cfg(target_os = "linux")]
        let store: Arc<CredentialStore> = zbus_secret_service_keyring_store::Store::new()?;
        #[cfg(target_os = "windows")]
        let store: Arc<CredentialStore> = windows_native_keyring_store::Store::new()?;
        #[cfg(target_os = "macos")]
        let store: Arc<CredentialStore> = apple_native_keyring_store::keychain::Store::new()?;
        #[cfg(target_os = "ios")]
        let store: Arc<CredentialStore> = apple_native_keyring_store::protected::Store::new()?;
        Ok(store)
    }
}

#[cfg(target_os = "android")]
mod system {
    pub(super) fn set(_: &str, _: &str) -> bool {
        false
    }

    pub(super) fn get(_: &str) -> Option<String> {
        None
    }
}
