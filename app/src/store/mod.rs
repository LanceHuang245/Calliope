//! The UI state: its signals stay private, pages read and change it through
//! these functions. A global is per surface, so each test has its own.
//!
//! The vault lives in `calliope-core`; this module keeps what the UI shows
//! of it: the files, the open note and its text, the view mode, the route.
//! Each part is a global of its own, so a keystroke re-runs only what
//! shows the text.
//!
//! - [`vault`]: the open vault, its files and other apps' changes;
//! - [`note`]: the open note, autosave and conflicts;
//! - [`files`]: creating, renaming and deleting files and folders;
//! - here: the view (mode, route, theme), notices and the start.

mod files;
mod note;
mod vault;

use std::{fs, path::PathBuf};

use calliope_core::vault::{Entry, Vault};
use nsh_core::{file_dialog::SaveDialog, prelude::*, storage::Storage};

pub(crate) use self::{
    files::{
        cancel_rename, delete, expanded, new_folder, new_folder_in, new_note, new_note_in, rename,
        rename_note, renaming, selected, set_expanded, start_rename,
    },
    note::{
        conflict, edit, image_src, note_path, note_title, opened, preview, resolve_conflict,
        select, text,
    },
    vault::{
        has_files, kind_of, open_vault, pick_vault, refresh, tree_items, vault, vault_name,
        vault_path,
    },
};
use self::{
    note::{load, save_or_copy, start_autosave},
    vault::watch_other_apps,
};
use crate::{logging, paths, strings};

const LAST_VAULT: &str = "vault";
const THEME: &str = "theme";

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Mode {
    #[default]
    Edit,
    Split,
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

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

fn spawn_task(task: impl Future<Output = ()> + 'static) {
    let owner = untracked(|| get::<Root>().0).unwrap_or_else(Owner::current);
    owner.run(|| {
        spawn(task);
    });
}

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

    start_autosave();
    watch_other_apps();
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
