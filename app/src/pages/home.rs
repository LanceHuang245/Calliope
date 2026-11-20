//! The home page, where no note is open: a new note in its middle and, on a
//! phone, the menu at the top and the floating toolbar at the bottom.

use nsh_base::theme;
use nsh_core::{platform::Os, prelude::*};

use crate::{
    components::{
        ButtonVariant, button, fab, floating_toolbar, icon_button, icons, toolbar_toggle,
    },
    store::{self, Route, SyncStatus},
    strings,
};

pub(crate) fn home_page() -> impl IntoElement {
    view()
        .flex_1()
        .child((!is_desktop()).then(|| {
            view()
                .flex_row()
                .justify_between()
                .p_2()
                .child(menu_button())
                .child(ios_new_note())
        }))
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_4()
                .child(text(strings::NO_NOTE).text_color(move || theme().muted_foreground))
                .child(button(
                    ButtonVariant::Primary,
                    strings::NEW_NOTE,
                    store::new_note,
                )),
        )
        .child((!is_desktop()).then(phone_toolbar))
}

/// Syncs the vault; dimmed while it syncs.
pub(crate) fn sync_button() -> impl IntoElement {
    let running = || matches!(store::sync_status(), SyncStatus::Running(..));
    view()
        .opacity(move || if running() { 0.5_f32 } else { 1.0 })
        .child(icon_button(icons::REFRESH, strings::SYNC, store::sync_now))
}

/// The phone's menu button: the files slide in from the left.
pub(crate) fn menu_button() -> impl IntoElement {
    icon_button(icons::MENU, strings::MENU, || store::set_menu_open(true))
}

/// iOS's new note (MVP 5.4): a `+` at the end of the top bar, where its
/// navigation bars have it (Android's is the [`phone_toolbar`]'s FAB).
pub(crate) fn ios_new_note() -> Option<impl IntoElement> {
    (os() == Os::Ios).then(|| icon_button(icons::PLUS, strings::NEW_NOTE, store::new_note))
}

/// The phones' floating toolbar, over the home page and the note (the
/// desktops have these under the files): sync (filled while it runs), the
/// quick switcher, the graph, the note's backlinks (with a note open) and
/// the trash; on Android with the FAB making a note beside it. Gone while
/// the keyboard is up, the text taking the room.
pub(crate) fn phone_toolbar() -> impl IntoElement {
    let syncing = || matches!(store::sync_status(), SyncStatus::Running(..));
    let graph = || store::route() == Route::Graph;
    let trash = || store::route() == Route::Trash;
    when(
        || keyboard_height() <= px(0.0),
        move || {
            let buttons = view()
                .flex_row()
                .gap_1()
                .child(toolbar_toggle(
                    icons::REFRESH,
                    strings::SYNC,
                    syncing,
                    store::sync_now,
                ))
                .child(toolbar_toggle(
                    icons::SEARCH,
                    strings::SEARCH,
                    store::switching,
                    || store::set_switching(true),
                ))
                .child(toolbar_toggle(
                    icons::NETWORK,
                    strings::GRAPH,
                    graph,
                    store::toggle_graph,
                ))
                .child(when(
                    || store::opened().is_some(),
                    || {
                        toolbar_toggle(
                            icons::LINK,
                            strings::BACKLINKS,
                            store::backlinks_shown,
                            store::toggle_backlinks,
                        )
                    },
                ))
                .child(toolbar_toggle(
                    icons::TRASH,
                    strings::TRASH,
                    trash,
                    store::toggle_trash,
                ));
            let new_note = (os() != Os::Ios)
                .then(|| fab(icons::PLUS, strings::NEW_NOTE, store::new_note).into_element());
            floating_toolbar(buttons, new_note)
        },
    )
}
