//! The UI state: its signals stay private, pages read and change it through
//! these functions. A global is per surface, so each test has its own.
//!
//! The vault lives in `calliope-core`; this module keeps what the UI shows
//! of it: the files, the open note and its text, the view mode, the route.
//! Each part is a global of its own, so a keystroke re-runs only what
//! shows the text.

use std::{fs, path::PathBuf, time::Duration};

use calliope_core::{
    note,
    vault::{Entry, EntryKind, Vault},
};
use nsh_base::{TreeItem, alert_dialog};
use nsh_core::{
    file_dialog::{OpenDialog, SaveDialog},
    platform::Lifecycle,
    prelude::*,
    storage::Storage,
};

use crate::{logging, paths, strings};

/// Storage key of the last vault opened (desktops).
const LAST_VAULT: &str = "vault";
/// Storage key of the theme.
const THEME: &str = "theme";

/// How a note is shown.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Mode {
    /// The Markdown source.
    #[default]
    Edit,
    /// The source beside the rendered note (desktops).
    Split,
    /// The rendered note.
    Read,
}

/// What the main area shows: on a phone the whole screen, on a desktop the
/// workspace beside the sidebar (`Files` and `Note` both show the note).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Route {
    #[default]
    Files,
    Note,
    Settings,
}

/// The appearance the user chose.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

// The state, a global per part.
#[derive(Clone, Default)]
struct OpenVault(Option<Vault>);
#[derive(Clone, Default)]
struct Entries(Vec<Entry>);
#[derive(Clone, Default)]
struct Expanded(Vec<SharedString>);
#[derive(Clone, Default)]
struct Selected(Option<String>);
#[derive(Clone, Default)]
struct NotePath(Option<String>);
/// Counts the notes opened: the note page is made again for each, not
/// when the open one is renamed.
#[derive(Clone, Default)]
struct Opened(u64);
#[derive(Clone, Default)]
struct NoteText(SharedString);
#[derive(Clone, Default)]
struct Preview(SharedString);
/// What the note's file holds, as last read or written.
#[derive(Clone, Default)]
struct Saved(String);
/// The file changed on disk while the note had unsaved changes.
#[derive(Clone, Default)]
struct Conflict(bool);
#[derive(Clone, Default)]
struct Renaming(Option<String>);
#[derive(Clone, Default)]
struct Notice(Option<String>);
#[derive(Clone, Default)]
struct SidebarHidden(bool);
/// The root's owner: tasks run under it outlive the view that started
/// them (a menu closes as its item is chosen).
#[derive(Clone, Default)]
struct Root(Option<Owner>);

fn get<T: Clone + Default + 'static>() -> T {
    global::<T>().get()
}

fn set<T: Default + 'static>(value: T) {
    global::<T>().set(value);
}

/// Spawns `task` under the root's owner.
fn spawn_task(task: impl Future<Output = ()> + 'static) {
    let owner = untracked(|| get::<Root>().0).unwrap_or_else(Owner::current);
    owner.run(|| {
        spawn(task);
    });
}

// --- Start ------------------------------------------------------------------

/// Restores the theme and the vault, and starts the timers: autosave, the
/// preview and other apps' changes. Called once by the root.
pub(crate) fn init() {
    set(Root(Some(Owner::current())));
    let theme = match Storage::get_item(THEME).as_deref() {
        Some("light") => Theme::Light,
        Some("dark") => Theme::Dark,
        _ => Theme::System,
    };
    set_theme(theme);

    // The desktops reopen the last vault; the phones always use the one in
    // the app's sandbox, made on first start.
    let start = match is_desktop() {
        true => Storage::get_item(LAST_VAULT)
            .map(PathBuf::from)
            .filter(|root| root.is_dir()),
        false => paths::sandbox_vault(),
    };
    if let Some(root) = start {
        open_vault(root);
    }

    // Autosave: a change reaches the disk within a second, while typing too.
    set_interval(Duration::from_secs(1), save);

    // The preview follows the text, at most 150 ms behind.
    set_interval(Duration::from_millis(150), || {
        let text = get::<NoteText>().0;
        if get::<Preview>().0 != text {
            set(Preview(text));
        }
    });

    // Other apps' changes: the desktops watch the vault, the phones scan
    // it again when the app comes back to the foreground.
    set_interval(Duration::from_millis(500), || {
        if watcher::take_changed() {
            refresh();
        }
    });
    effect(|| {
        let state = lifecycle();
        untracked(|| match state {
            Lifecycle::Active if !is_desktop() => refresh(),
            Lifecycle::Active => {}
            // Leaving the foreground (or the window losing focus) saves.
            _ => save(),
        });
    });
}

// --- Reading ----------------------------------------------------------------

pub(crate) fn vault() -> Option<Vault> {
    get::<OpenVault>().0
}

/// The open vault's name, empty without one.
pub(crate) fn vault_name() -> String {
    global::<OpenVault>().with(|v| v.0.as_ref().map(Vault::name).unwrap_or_default())
}

/// The vault's folder, empty without one.
pub(crate) fn vault_path() -> String {
    global::<OpenVault>().with(|v| {
        v.0.as_ref()
            .map(|v| v.root().display().to_string())
            .unwrap_or_default()
    })
}

/// The files as tree items: notes by their title, others by file name.
pub(crate) fn tree_items() -> Vec<TreeItem> {
    fn item(entry: &Entry) -> TreeItem {
        let label = match entry.kind {
            EntryKind::Note => note::title(&entry.name),
            _ => &entry.name,
        };
        TreeItem::new(entry.path.clone(), label.to_owned())
            .children(entry.children.iter().map(item))
    }
    global::<Entries>().with(|e| e.0.iter().map(item).collect())
}

pub(crate) fn has_files() -> bool {
    global::<Entries>().with(|e| !e.0.is_empty())
}

/// The kind of the file or folder at `path`, read untracked.
pub(crate) fn kind_of(path: &str) -> Option<EntryKind> {
    fn find(entries: &[Entry], path: &str) -> Option<EntryKind> {
        entries.iter().find_map(|e| match e.path == path {
            true => Some(e.kind),
            false => find(&e.children, path),
        })
    }
    untracked(|| global::<Entries>().with(|e| find(&e.0, path)))
}

pub(crate) fn expanded() -> Vec<SharedString> {
    get::<Expanded>().0
}

pub(crate) fn selected() -> Option<SharedString> {
    get::<Selected>().0.map(SharedString::from)
}

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

/// The open note's title.
pub(crate) fn note_title() -> String {
    note_path()
        .map(|p| note::title(&p).to_owned())
        .unwrap_or_default()
}

/// The open note's text, as edited.
pub(crate) fn text() -> SharedString {
    get::<NoteText>().0
}

/// The text the preview shows: the note's, a moment behind typing.
pub(crate) fn preview() -> SharedString {
    get::<Preview>().0
}

pub(crate) fn conflict() -> bool {
    get::<Conflict>().0
}

pub(crate) fn renaming() -> Option<String> {
    get::<Renaming>().0
}

pub(crate) fn notice() -> Option<String> {
    get::<Notice>().0
}

pub(crate) fn mode() -> Mode {
    get::<Mode>()
}

pub(crate) fn route() -> Route {
    get::<Route>()
}

pub(crate) fn theme() -> Theme {
    get::<Theme>()
}

pub(crate) fn sidebar_hidden() -> bool {
    get::<SidebarHidden>().0
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

// --- The vault --------------------------------------------------------------

/// Opens (or creates) the vault at `root`.
pub(crate) fn open_vault(root: PathBuf) {
    save_or_copy();
    match Vault::open(&root) {
        Ok(vault) => {
            if is_desktop() {
                let _ = Storage::set_item(LAST_VAULT, root.to_string_lossy());
            }
            watcher::watch(&vault);
            log::info!("opened the vault {}", root.display());
            set(OpenVault(Some(vault)));
            set(NotePath(None));
            set(Selected(None));
            set(Expanded(Vec::new()));
            set(Route::Files);
            refresh();
        }
        Err(e) => notify(format!("{}: {e}", root.display())),
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
        Ok(entries) => set(Entries(entries)),
        Err(e) => notify(e),
    }
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

// --- The open note ----------------------------------------------------------

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
            set(Route::Note);
        }
        Err(e) => notify(format!("{path}: {e}")),
    }
}

/// Shows `text` as the open note's, as its file holds it.
fn load(text: String) {
    let shared = SharedString::from(text.clone());
    set(NoteText(shared.clone()));
    set(Preview(shared));
    set(Saved(text));
}

/// The editor's text changed.
pub(crate) fn edit(text: &SharedString) {
    set(NoteText(text.clone()));
}

/// Writes the open note when it has unsaved changes (never while in
/// conflict). A file another app changed since is not overwritten: the
/// note is in conflict instead.
fn save() {
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

/// Writes the open note's text to its file.
fn write_note() {
    let (Some(vault), Some(path)) = (vault(), note_path()) else {
        return;
    };
    let text = text();
    match vault.write(&path, &text) {
        Ok(()) => set(Saved(text.to_string())),
        Err(e) => notify(format!("{path}: {e}")),
    }
}

/// Saves the open note before leaving it: in conflict, the user's text
/// goes to a copy, so neither side is lost.
fn save_or_copy() {
    untracked(|| {
        if !conflict() {
            return save();
        }
        set(Conflict(false));
        let (Some(vault), Some(path)) = (vault(), note_path()) else {
            return;
        };
        match vault.save_copy(&path, &text()) {
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

// --- Files ------------------------------------------------------------------

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

pub(crate) fn set_expanded(expanded: &[SharedString]) {
    set(Expanded(expanded.to_vec()));
}

/// Creates a note in the target folder and opens it.
pub(crate) fn new_note() {
    new_note_in(&target_dir());
}

/// Creates a note in folder `dir` and opens it.
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

/// Creates a folder in the target folder and starts renaming it.
pub(crate) fn new_folder() {
    new_folder_in(&target_dir());
}

/// Creates a folder in folder `dir` and starts renaming it.
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

/// Starts renaming the item at `path`.
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
            // `path` after the rename: itself or what is inside it moved.
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

// --- View -------------------------------------------------------------------

pub(crate) fn set_mode(mode: Mode) {
    set(mode);
}

/// Edit to Read and back (`mod+e`).
pub(crate) fn toggle_read() {
    set_mode(match untracked(self::mode) {
        Mode::Read => Mode::Edit,
        _ => Mode::Read,
    });
}

/// Shows `route`; leaving the note saves it.
pub(crate) fn set_route(route: Route) {
    if route != Route::Note {
        save_or_copy();
    }
    set(route);
}

pub(crate) fn toggle_sidebar() {
    global::<SidebarHidden>().update(|h| h.0 = !h.0);
}

pub(crate) fn set_theme(theme: Theme) {
    let (appearance, name) = match theme {
        Theme::System => (Appearance::System, "system"),
        Theme::Light => (Appearance::Light, "light"),
        Theme::Dark => (Appearance::Dark, "dark"),
    };
    set_color_scheme(appearance);
    let _ = Storage::set_item(THEME, name);
    set(theme);
}

/// Shows `message` to the user (and logs it).
pub(crate) fn notify(message: impl ToString) {
    let message = message.to_string();
    log::warn!("{message}");
    set(Notice(Some(message)));
}

pub(crate) fn dismiss_notice() {
    set(Notice(None));
}

/// Hands the log to the user: saved where they choose on the desktops,
/// copied to the clipboard on the phones.
pub(crate) fn export_logs() {
    let Some(log) = logging::path() else {
        return;
    };
    log::logger().flush();
    if !is_desktop() {
        set_clipboard(fs::read_to_string(&log).unwrap_or_default());
        notify(strings::LOGS_COPIED);
        return;
    }
    let picked = pick_save_path(SaveDialog::new().file_name("calliope.log"));
    spawn_task(async move {
        match picked.await {
            Ok(Some(to)) => {
                if let Err(e) = fs::copy(&log, &to) {
                    notify(e);
                }
            }
            Ok(None) => {}
            Err(e) => notify(e),
        }
    });
}

/// The vault's watcher: other apps' changes on the desktops.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
mod watcher {
    use std::cell::RefCell;

    use calliope_core::{vault::Vault, watch::Watcher};

    thread_local! {
        static WATCHER: RefCell<Option<Watcher>> = const { RefCell::new(None) };
    }

    /// Watches `vault` (in place of the vault watched before).
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
