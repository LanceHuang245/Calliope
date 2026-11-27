//! The app's commands (MVP phase 6): one table the command palette lists
//! and the desktops take their keys from, so a command is added here once.
//! A plugin's command goes into the same table (phase 7).

use super::*;

/// A command of the palette.
pub(crate) struct Command {
    /// What the palette's rows and `on_select` name it by.
    pub id: &'static str,
    /// Its key among the texts (`t!`): what its row says.
    pub title: &'static str,
    /// The key that runs it too on the desktops, as nsh's keymap names it
    /// (`mod+n`); `None` for the ones only the palette runs.
    pub keys: Option<&'static str>,
    pub run: fn(),
}

/// Every command, in the order the palette lists them.
pub(crate) const COMMANDS: &[Command] = &[
    #[cfg(target_os = "windows")]
    Command {
        id: "check-grammar",
        title: "check_current_note",
        keys: None,
        run: grammar::check_current,
    },
    Command {
        id: "new-note",
        title: "new_note",
        keys: Some("mod+n"),
        run: new_note,
    },
    Command {
        id: "toggle-read",
        title: "toggle_read",
        keys: Some("mod+e"),
        run: toggle_read,
    },
    Command {
        id: "quick-switch",
        title: "quick_switch",
        keys: Some("mod+o"),
        run: quick_switch,
    },
    Command {
        id: "graph",
        title: "graph",
        keys: Some("mod+g"),
        run: toggle_graph,
    },
    Command {
        id: "sidebar",
        title: "toggle_sidebar",
        keys: Some("mod+\\"),
        run: toggle_sidebar,
    },
    Command {
        id: "edit-mode",
        title: "edit",
        keys: None,
        run: edit_mode,
    },
    Command {
        id: "split-mode",
        title: "split",
        keys: None,
        run: split_mode,
    },
    Command {
        id: "read-mode",
        title: "read",
        keys: None,
        run: read_mode,
    },
    Command {
        id: "save-copy",
        title: "save_copy",
        keys: None,
        run: save_as_copy,
    },
    Command {
        id: "sync",
        title: "sync_now",
        keys: None,
        run: sync_now,
    },
    Command {
        id: "trash",
        title: "trash",
        keys: None,
        run: toggle_trash,
    },
    Command {
        id: "settings",
        title: "settings",
        keys: None,
        run: settings,
    },
    Command {
        id: "theme",
        title: "cycle_theme",
        keys: None,
        run: cycle_theme,
    },
];

/// The commands the palette lists.
pub(crate) fn commands() -> &'static [Command] {
    COMMANDS
}

pub(crate) fn command_available(command: &Command) -> bool {
    #[cfg(target_os = "windows")]
    if command.id == "check-grammar" {
        return grammar::available();
    }
    let _ = command;
    true
}

/// Runs the command `id` names; an id no command has does nothing.
pub(crate) fn run_command(id: &str) {
    if let Some(command) = COMMANDS.iter().find(|command| command.id == id) {
        (command.run)();
    }
}

/// The command palette is open.
#[derive(Clone, Default)]
struct Palette(bool);

pub(crate) fn palette() -> bool {
    get::<Palette>().0
}

/// Opens the palette, closing the menu over the page.
pub(crate) fn set_palette(open: bool) {
    set(MenuOpen(false));
    set(Palette(open));
}

// What the commands that take an argument run: the palette names commands
// by id, and a `fn()` takes none.

fn quick_switch() {
    set_switching(true);
}

fn edit_mode() {
    set_mode(Mode::Edit);
}

fn split_mode() {
    set_mode(Mode::Split);
}

fn read_mode() {
    set_mode(Mode::Read);
}

fn settings() {
    set_route(Route::Settings);
}
