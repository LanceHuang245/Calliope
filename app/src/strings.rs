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

// Note.
pub(crate) const NO_NOTE: &str = "Select a note, or create one.";
pub(crate) const BACK: &str = "Back";
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
pub(crate) const LOGS_COPIED: &str = "The logs were copied to the clipboard.";

pub(crate) fn version() -> String {
    format!("Version {}", env!("CARGO_PKG_VERSION"))
}

pub(crate) fn saved_copy(path: &str) -> String {
    format!("Your changes were saved to {path}.")
}
