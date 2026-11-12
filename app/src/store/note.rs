//! The open note: its text as edited, the preview, autosave, and the
//! conflicts with other apps' changes.

use std::time::Duration;

use calliope_core::note;
use nsh_core::platform::Lifecycle;

use super::*;

pub(crate) fn note_path() -> Option<String> {
    get::<NotePath>().0
}

/// Which note is open: a new value for each note opened, `None` without
/// one; a rename keeps it.
pub(crate) fn opened() -> Option<u64> {
    global::<NotePath>()
        .with(|p| p.0.is_some())
        .then(|| get::<Opened>().0)
}

pub(crate) fn note_title() -> String {
    note_path()
        .map(|p| note::title(&p).to_owned())
        .unwrap_or_default()
}

pub(crate) fn text() -> SharedString {
    get::<NoteText>().0
}

pub(crate) fn preview() -> SharedString {
    get::<Preview>().0
}

pub(crate) fn conflict() -> bool {
    get::<Conflict>().0
}

/// Where an image `src` of the open note loads from: a file of the vault
/// (relative to the note's folder) as an absolute path, a URL as it is.
pub(crate) fn image_src(src: &str) -> String {
    untracked(|| {
        let vault = vault()?;
        let path = note_path()?;
        note::resolve_file(&vault.abs(&path), src)
    })
    .map(|path| path.to_string_lossy().into_owned())
    .unwrap_or_else(|| src.to_owned())
}

/// Selects the item at `path` in the files; a note opens.
pub(crate) fn select(path: &SharedString) {
    let path = path.to_string();
    set(Selected(Some(path.clone())));
    if kind_of(&path) == Some(EntryKind::Note) {
        open_note(path);
    }
}

/// Opens the note at `path`, saving the one open before.
fn open_note(path: String) {
    save_or_copy();
    let Some(vault) = untracked(vault) else {
        return;
    };
    match vault.read(&path) {
        Ok(text) => {
            load(text);
            set(NotePath(Some(path)));
            global::<Opened>().update(|o| o.0 += 1);
            set(Conflict(false));
            set_route(Route::Note);
        }
        Err(e) => notify(format!("{path}: {e}")),
    }
}

pub(super) fn load(text: String) {
    let shared = SharedString::from(text.clone());
    set(NoteText(shared.clone()));
    set(Preview(shared));
    set(Saved(text));
}

pub(crate) fn edit(text: &SharedString) {
    set(NoteText(text.clone()));
}

/// Autosave, the preview and saving when the app leaves the foreground.
pub(super) fn start_autosave() {
    // A change reaches the disk within a second, while typing too.
    set_interval(Duration::from_secs(1), save);

    // The preview follows the text, at most 150 ms behind.
    set_interval(Duration::from_millis(150), || {
        let text = get::<NoteText>().0;
        if get::<Preview>().0 != text {
            set(Preview(text));
        }
    });

    // Leaving the foreground (or the window losing focus) saves.
    effect(|| {
        if lifecycle() != Lifecycle::Active {
            save();
        }
    });
}

/// Writes the open note when it has unsaved changes (never while in
/// conflict). A file another app changed since is not overwritten: the
/// note is in conflict instead.
pub(super) fn save() {
    untracked(|| {
        let saved = get::<Saved>().0;
        if conflict() || *text() == *saved {
            return;
        }
        let (Some(vault), Some(path)) = (vault(), note_path()) else {
            return;
        };
        if vault.read(&path).is_ok_and(|disk| disk != saved) {
            log::warn!("{path} changed on disk with unsaved changes");
            set(Conflict(true));
            return;
        }
        write_note();
    });
}

fn write_note() {
    let (Some(vault), Some(path)) = (vault(), note_path()) else {
        return;
    };
    let text = text();
    match vault.write(&path, &text) {
        Ok(()) => {
            set(Saved(text.to_string()));
            reindex();
            edited();
        }
        Err(e) => notify(format!("{path}: {e}")),
    }
}

/// Saves the open note before leaving it: in conflict, the user's text
/// goes to a copy, so neither side is lost.
pub(super) fn save_or_copy() {
    untracked(|| {
        if !conflict() {
            return save();
        }
        set(Conflict(false));
        let (Some(vault), Some(path)) = (vault(), note_path()) else {
            return;
        };
        match vault.save_copy(&path, "conflict", text().as_bytes()) {
            Ok(copy) => notify(strings::saved_copy(&copy)),
            Err(e) => notify(format!("{path}: {e}")),
        }
    });
}

/// Settles a conflict: `keep_mine` writes the user's text over the file,
/// else the file's text replaces it.
pub(crate) fn resolve_conflict(keep_mine: bool) {
    set(Conflict(false));
    if keep_mine {
        write_note();
    } else if let (Some(vault), Some(path)) = (vault(), note_path()) {
        match vault.read(&path) {
            Ok(disk) => load(disk),
            Err(e) => notify(format!("{path}: {e}")),
        }
    }
}
