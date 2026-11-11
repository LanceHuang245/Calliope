//! The open vault: opening one, its files as the tree shows them, and
//! other apps' changes to them.

use std::time::Duration;

use calliope_core::note;
use nsh_base::TreeItem;
use nsh_core::{file_dialog::OpenDialog, platform::Lifecycle};

use super::*;

pub(crate) fn vault() -> Option<Vault> {
    get::<OpenVault>().0
}

pub(crate) fn vault_name() -> String {
    global::<OpenVault>().with(|v| v.0.as_ref().map(Vault::name).unwrap_or_default())
}

pub(crate) fn vault_path() -> String {
    global::<OpenVault>().with(|v| {
        v.0.as_ref()
            .map(|v| v.root().display().to_string())
            .unwrap_or_default()
    })
}

pub(crate) fn tree_items() -> Vec<TreeItem> {
    fn item(entry: &Entry) -> TreeItem {
        let label = match entry.kind {
            EntryKind::Note => note::title(&entry.name),
            _ => &entry.name,
        };
        TreeItem::new(entry.path.clone(), label.to_owned())
            .children(entry.children.iter().map(item))
    }
    global::<Entries>().with(|e| e.tree.iter().map(item).collect())
}

pub(crate) fn has_files() -> bool {
    global::<Entries>().with(|e| !e.tree.is_empty())
}

/// The kind of the file or folder at `path`, read untracked.
pub(crate) fn kind_of(path: &str) -> Option<EntryKind> {
    untracked(|| global::<Entries>().with(|e| e.kinds.get(path).copied()))
}

fn entries(tree: Vec<Entry>) -> Entries {
    fn add(entries: &[Entry], kinds: &mut HashMap<String, EntryKind>) {
        for e in entries {
            kinds.insert(e.path.clone(), e.kind);
            add(&e.children, kinds);
        }
    }
    let mut kinds = HashMap::new();
    add(&tree, &mut kinds);
    Entries { tree, kinds }
}

/// Opens (or creates) the vault at `root`.
pub(crate) fn open_vault(root: PathBuf) {
    save_or_copy();
    match Vault::open(&root) {
        Ok(vault) => {
            let last = match is_desktop() {
                true => root.to_string_lossy().into_owned(),
                false => vault.name(),
            };
            let _ = Storage::set_item(LAST_VAULT, last);
            watcher::watch(&vault);
            log::info!("opened the vault {}", root.display());
            set(OpenVault(Some(vault)));
            set(NotePath(None));
            set(Selected(None));
            set(Expanded(Vec::new()));
            set(Route::Home);
            reset_links();
            refresh();
        }
        Err(e) => notify(format!("{}: {e}", root.display())),
    }
}

/// Creates the phone's vault named `name` in the app's sandbox and opens
/// it. An empty name would open the sandbox itself.
pub(crate) fn create_vault(name: &str) {
    if name.trim().is_empty() {
        return;
    }
    if let Some(root) = paths::sandbox_vault(name.trim()) {
        open_vault(root);
    }
}

/// Shows the system's folder dialog (`new`: the save dialog, for a new
/// folder) and opens the vault picked. Desktops only.
pub(crate) fn pick_vault(new: bool) {
    if new {
        let picked = pick_save_path(SaveDialog::new().file_name("Notes"));
        spawn_task(async move {
            match picked.await {
                Ok(Some(root)) => open_vault(root),
                Ok(None) => {}
                Err(e) => notify(e),
            }
        });
    } else {
        let picked = pick_paths(OpenDialog::folders());
        spawn_task(async move {
            match picked.await {
                Ok(Some(roots)) => roots.into_iter().take(1).for_each(open_vault),
                Ok(None) => {}
                Err(e) => notify(e),
            }
        });
    }
}

/// Scans the vault again and loads the open note again when another app
/// changed it: at once when it has no unsaved changes, else it is in
/// conflict and neither side is overwritten until the user chooses.
pub(crate) fn refresh() {
    let Some(vault) = untracked(vault) else {
        return;
    };
    match vault.scan() {
        Ok(tree) => set(entries(tree)),
        Err(e) => notify(e),
    }
    reindex();
    let Some(path) = untracked(note_path) else {
        return;
    };
    // A note deleted or moved by another app stays open; a change writes
    // it back.
    let Ok(disk) = vault.read(&path) else {
        return;
    };
    untracked(|| {
        let saved = get::<Saved>().0;
        if disk == saved {
            return;
        }
        if *text() == *saved {
            load(disk);
        } else if !conflict() {
            log::warn!("{path} changed on disk with unsaved changes");
            set(Conflict(true));
        }
    });
}

/// Other apps' changes: the desktops watch the vault, the phones scan it
/// again when the app comes back to the foreground.
pub(super) fn watch_other_apps() {
    set_interval(Duration::from_millis(500), || {
        if watcher::take_changed() {
            refresh();
        }
    });
    effect(|| {
        if lifecycle() == Lifecycle::Active && !is_desktop() {
            refresh();
        }
    });
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod watcher {
    use std::cell::RefCell;

    use calliope_core::{vault::Vault, watch::Watcher};

    thread_local! {
        static WATCHER: RefCell<Option<Watcher>> = const { RefCell::new(None) };
    }

    pub(super) fn watch(vault: &Vault) {
        let watcher = Watcher::new(vault.root())
            .inspect_err(|e| log::warn!("cannot watch {}: {e}", vault.root().display()))
            .ok();
        WATCHER.with(|w| *w.borrow_mut() = watcher);
    }

    pub(super) fn take_changed() -> bool {
        WATCHER.with(|w| w.borrow().as_ref().is_some_and(Watcher::take_changed))
    }
}

/// The phones watch nothing: they scan when the app comes back.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod watcher {
    pub(super) fn watch(_: &calliope_core::vault::Vault) {}

    pub(super) fn take_changed() -> bool {
        false
    }
}
