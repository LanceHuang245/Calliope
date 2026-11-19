//! Where the app keeps its own files: its data directory (logs) and, on
//! the phones, the vault in the app's sandbox.

use std::{path::PathBuf, sync::OnceLock};

/// The app's data directory, kept by [`init`].
static DATA_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Keeps the app's data directory for every thread (the secrets are read
/// in the background): nsh's, which only the engine's thread can ask for,
/// beside its `storage.json`. Application Support on Apple platforms,
/// `%APPDATA%` on Windows, `$XDG_DATA_HOME` (`~/.local/share`) on Linux,
/// each with the app's id; on Android, in the files directory the system
/// gives this user (`files/nsh/<app id>`), so a work profile or another
/// user of the phone has its own.
pub(crate) fn init() {
    DATA_DIR.get_or_init(nsh_core::prelude::data_dir);
}

/// The app's data directory ([`init`]); `None` where the engine keeps no
/// files (tests).
pub(crate) fn data_dir() -> Option<PathBuf> {
    DATA_DIR.get().cloned().flatten()
}

/// The phones' vault named `name`, in the app's sandbox (the data
/// directory, or `CALLIOPE_SANDBOX` for tests and development).
pub(crate) fn sandbox_vault(name: &str) -> Option<PathBuf> {
    std::env::var_os("CALLIOPE_SANDBOX")
        .map(PathBuf::from)
        .or_else(data_dir)
        .map(|dir| dir.join(name))
}
