//! Files and folders: the tree's expansion and selection, and creating,
//! renaming and deleting items.

use nsh_base::alert_dialog;

use super::*;

pub(crate) fn expanded() -> Vec<SharedString> {
    get::<Expanded>().0
}

pub(crate) fn selected() -> Option<SharedString> {
    get::<Selected>().0.map(SharedString::from)
}

pub(crate) fn renaming() -> Option<String> {
    get::<Renaming>().0
}

pub(crate) fn set_expanded(expanded: &[SharedString]) {
    set(Expanded(expanded.to_vec()));
}

/// The folder new items go in: the selected folder, or the selected
/// file's folder.
fn target_dir() -> String {
    let Some(path) = untracked(|| get::<Selected>().0) else {
        return String::new();
    };
    match kind_of(&path) {
        Some(EntryKind::Folder) => path,
        _ => path.rsplit_once('/').map_or("", |(dir, _)| dir).to_owned(),
    }
}

/// Expands `dir` and the folders above it.
fn reveal(dir: &str) {
    global::<Expanded>().update(|expanded| {
        let mut path = String::new();
        for part in dir.split('/').filter(|p| !p.is_empty()) {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
            let id = SharedString::from(path.clone());
            if !expanded.0.contains(&id) {
                expanded.0.push(id);
            }
        }
    });
}

pub(crate) fn new_note() {
    new_note_in(&target_dir());
}

pub(crate) fn new_note_in(dir: &str) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    match vault.create_note(dir) {
        Ok(path) => {
            reveal(dir);
            refresh();
            select(&path.into());
        }
        Err(e) => notify(e),
    }
}

pub(crate) fn new_folder() {
    new_folder_in(&target_dir());
}

/// Creates a folder in `dir` and starts renaming it.
pub(crate) fn new_folder_in(dir: &str) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    match vault.create_folder(dir) {
        Ok(path) => {
            reveal(dir);
            refresh();
            set(Selected(Some(path.clone())));
            set(Renaming(Some(path)));
        }
        Err(e) => notify(e),
    }
}

pub(crate) fn start_rename(path: String) {
    set(Renaming(Some(path)));
}

pub(crate) fn cancel_rename() {
    set(Renaming(None));
}

/// Renames the item of the rename dialog to `name`.
pub(crate) fn rename(name: &str) {
    if let Some(old) = untracked(renaming) {
        set(Renaming(None));
        rename_item(old, name);
    }
}

/// Renames the open note to `name` (its title edited).
pub(crate) fn rename_note(name: &str) {
    if let Some(path) = untracked(note_path) {
        rename_item(path, name);
    }
}

/// Renames the item at `old` to `name`; the open note and the selection
/// follow it.
fn rename_item(old: String, name: &str) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    if name.trim().is_empty() {
        return;
    }
    save_or_copy();
    match vault.rename(&old, name) {
        Ok(new) => {
            let moved = |path: &str| match path == old || path.starts_with(&format!("{old}/")) {
                true => format!("{new}{}", &path[old.len()..]),
                false => path.to_owned(),
            };
            untracked(|| {
                set(NotePath(note_path().map(|p| moved(&p))));
                set(Selected(get::<Selected>().0.map(|p| moved(&p))));
            });
            refresh();
        }
        Err(e) => notify(e),
    }
}

/// Asks, then moves the item at `path` to the trash: the system's on the
/// desktops, the vault's `.trash/` on the phones.
pub(crate) fn delete(path: String) {
    spawn_task(async move {
        let choice = alert_dialog()
            .title(strings::DELETE_TITLE)
            .description(strings::DELETE_DESCRIPTION)
            .cancel("cancel", strings::CANCEL)
            .destructive("delete", strings::DELETE)
            .show()
            .await;
        if !matches!(choice, Ok(r) if r.action.as_deref() == Some("delete")) {
            return;
        }
        let Some(vault) = untracked(vault) else {
            return;
        };
        // Close the note first, so that autosave cannot write it back.
        let inside = |p: &String| *p == path || p.starts_with(&format!("{path}/"));
        if untracked(note_path).is_some_and(|p| inside(&p)) {
            set(NotePath(None));
            set(Conflict(false));
        }
        let trashed = match is_desktop() {
            true => move_to_trash(vault.abs(&path)).await.map(|_| ()),
            false => vault.trash(&path).map_err(|e| e.to_string()),
        };
        match trashed {
            Ok(()) => {
                set(Selected(None));
                refresh();
            }
            Err(e) => notify(e),
        }
    });
}
