//! Every text of the UI, in one place (English only for now), so other
//! languages can be added later.

pub(crate) const APP_NAME: &str = "Calliope";

// Files.
pub(crate) const NEW_NOTE: &str = "New note";
pub(crate) const NEW_FOLDER: &str = "New folder";
pub(crate) const RENAME: &str = "Rename";
pub(crate) const NAME: &str = "Name";
pub(crate) const DELETE: &str = "Delete";
pub(crate) const SETTINGS: &str = "Settings";
pub(crate) const FILES: &str = "Files";
pub(crate) const NO_FILES: &str = "No notes yet.";
pub(crate) const DELETE_TITLE: &str = "Delete this item?";
pub(crate) const DELETE_DESCRIPTION: &str = "It will be moved to the trash.";
pub(crate) const CANCEL: &str = "Cancel";

// Vault.
pub(crate) const NO_VAULT: &str = "Open a folder of Markdown notes, or create a new vault.";
pub(crate) const OPEN_FOLDER: &str = "Open folder…";
pub(crate) const NEW_VAULT: &str = "New vault…";
pub(crate) const NAME_VAULT: &str = "Name a vault to keep your notes in.";
pub(crate) const VAULT_NAME: &str = "Vault name";
pub(crate) const CREATE: &str = "Create";

// Note.
pub(crate) const NO_NOTE: &str = "Select a note, or create one.";
pub(crate) const BACK: &str = "Back";
pub(crate) const MENU: &str = "Menu";
pub(crate) const EDIT: &str = "Edit";
pub(crate) const SPLIT: &str = "Split";
pub(crate) const READ: &str = "Read";
pub(crate) const TITLE: &str = "Title";
pub(crate) const SOURCE: &str = "Markdown source";
pub(crate) const FIND: &str = "Find";
pub(crate) const REPLACE: &str = "Replace";
pub(crate) const REPLACE_ALL: &str = "All";
pub(crate) const CLOSE: &str = "Close";
pub(crate) const PREVIOUS_MATCH: &str = "Previous match";
pub(crate) const NEXT_MATCH: &str = "Next match";
pub(crate) const PROPERTIES: &str = "Properties";
pub(crate) const CONFLICT: &str = "This note was changed by another app.";
pub(crate) const KEEP_MINE: &str = "Keep mine";
pub(crate) const USE_DISK: &str = "Use theirs";

// Settings.
pub(crate) const APPEARANCE: &str = "Appearance";
pub(crate) const SYSTEM: &str = "System";
pub(crate) const LIGHT: &str = "Light";
pub(crate) const DARK: &str = "Dark";
pub(crate) const VAULT: &str = "Vault";
pub(crate) const ABOUT: &str = "About";
pub(crate) const LICENSE: &str = "Calliope is licensed under the Apache License 2.0. It is built on \
     Native Shell (Apache-2.0) and other open-source crates under their own licenses \
     (MIT, Apache-2.0, BSD); formulas use the KaTeX fonts (SIL Open Font License), \
     icons are Lucide's (ISC).";
pub(crate) const EXPORT_LOGS: &str = "Export logs…";
pub(crate) const ON: &str = "On";
pub(crate) const OFF: &str = "Off";
pub(crate) const SAVE: &str = "Save";
pub(crate) const LOGS_COPIED: &str = "The logs were copied to the clipboard.";

// Links.
pub(crate) const BACKLINKS: &str = "Backlinks";
pub(crate) const NO_BACKLINKS: &str = "No other note links here.";

pub(crate) const NO_DIAGRAM: &str = "This is not a Mermaid diagram.";
pub(crate) const DIAGRAM_FAILED: &str = "This diagram could not be drawn.";
pub(crate) const GRAPH: &str = "Graph";
pub(crate) const ORPHANS: &str = "Orphans";
pub(crate) const UNCREATED: &str = "Uncreated notes";
pub(crate) const SEARCH: &str = "Search";
pub(crate) const FIND_NOTE: &str = "Find or create a note";

// Sync.
pub(crate) const SYNC: &str = "Sync";
pub(crate) const WEBDAV: &str = "WebDAV";
pub(crate) const S3: &str = "S3";
pub(crate) const SERVER_ADDRESS: &str = "Server address";
pub(crate) const USER_NAME: &str = "User name";
pub(crate) const PASSWORD: &str = "Password";
pub(crate) const REMOTE_FOLDER: &str = "Remote folder";
pub(crate) const ENDPOINT: &str = "Endpoint";
pub(crate) const REGION: &str = "Region";
pub(crate) const BUCKET: &str = "Bucket";
pub(crate) const ACCESS_KEY: &str = "Access key ID";
pub(crate) const SECRET_KEY: &str = "Secret access key";
pub(crate) const PATH_PREFIX: &str = "Path prefix";
pub(crate) const PATH_STYLE: &str = "Path-style";
pub(crate) const VIRTUAL_HOST: &str = "Virtual-hosted";
pub(crate) const DEVICE_NAME: &str = "Device name";
pub(crate) const AUTO_SYNC: &str = "Sync automatically";
pub(crate) const SYNC_MINUTES: &str = "Every how many minutes";
pub(crate) const SECRET_KEPT: &str = "Leave it empty to keep the saved one.";
pub(crate) const TEST_CONNECTION: &str = "Test connection";
pub(crate) const SYNC_NOW: &str = "Sync now";
pub(crate) const SYNC_SAVED: &str = "The sync settings were saved.";
pub(crate) const SYNC_WORKS: &str = "The connection works.";
pub(crate) const NOT_SYNCED: &str = "Not synced yet";
pub(crate) const SYNC_FAILED: &str = "Sync failed";
pub(crate) const SECRET_IN_FILE: &str = "No keyring was found, so the password is kept in a \
     file only Calliope reads. Install GNOME Keyring or KWallet, then save the password \
     again to keep it in the keyring.";
pub(crate) const DELETE_FILES: &str = "Delete these files?";

pub(crate) fn deleting(count: usize) -> String {
    format!(
        "This sync would delete {count} files, here or on the server. Files deleted here go \
         to the vault's trash."
    )
}

pub(crate) fn syncing(done: usize, total: usize) -> String {
    format!("Syncing… {done} of {total}")
}

pub(crate) fn synced_at(time: &str) -> String {
    format!("Synced at {time}")
}

pub(crate) fn sync_failed(error: &dyn std::fmt::Display) -> String {
    format!("Sync failed: {error}.")
}

pub(crate) fn sync_conflicts(copies: &[String]) -> String {
    format!(
        "Changed here and elsewhere, so both versions were kept: {}",
        copies.join(", ")
    )
}

pub(crate) fn create_note(name: &str) -> String {
    format!("Create the note “{name}”")
}

pub(crate) fn version() -> String {
    format!("Version {}", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn saved_copy(path: &str) -> String {
    format!("Your changes were saved to {path}.")
}
