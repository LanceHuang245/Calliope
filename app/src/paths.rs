//! Where the app keeps its own files: its data directory (logs) and, on
//! the phones, the vault in the app's sandbox.

use std::path::PathBuf;

/// The app's id (`nsh.json` `"id"`).
const APP_ID: &str = "io.github.lancehuang245.calliope";

/// The app's data directory: Application Support on Apple platforms,
/// `%APPDATA%` on Windows, `$XDG_DATA_HOME` (`~/.local/share`) on Linux,
/// the app's files directory on Android.
pub(crate) fn data_dir() -> Option<PathBuf> {
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    let base = if cfg!(target_os = "android") {
        return Some(PathBuf::from("/data/data").join(APP_ID).join("files"));
    } else if cfg!(any(target_os = "macos", target_os = "ios")) {
        env("HOME").map(|home| home.join("Library/Application Support"))
    } else if cfg!(target_os = "windows") {
        env("APPDATA")
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|home| home.join(".local/share")))
    };
    base.map(|dir| dir.join(APP_ID))
}

/// The vault opened at start on the phones (in the app's sandbox).
/// `CALLIOPE_VAULT` names another folder, for tests and development.
pub(crate) fn sandbox_vault() -> Option<PathBuf> {
    std::env::var_os("CALLIOPE_VAULT")
        .map(PathBuf::from)
        .or_else(|| data_dir().map(|dir| dir.join("Vault")))
}
