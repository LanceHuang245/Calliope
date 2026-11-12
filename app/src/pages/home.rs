//! The home page, where no note is open: a new note in its middle and, on a
//! phone, a top bar with the menu, sync, the quick switcher and the graph.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    components::{icon_button, icons, primary},
    store::{self, Route, SyncStatus},
    strings,
};

pub(crate) fn home_page() -> impl IntoElement {
    view()
        .flex_1()
        .child((!is_desktop()).then(|| {
            view()
                .flex_row()
                .p_2()
                .child(menu_button())
                .child(view().flex_1())
                .child(phone_actions())
        }))
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_4()
                .child(text(strings::NO_NOTE).text_color(move || theme().muted_foreground))
                .child(primary(strings::NEW_NOTE, store::new_note)),
        )
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

/// The phone's top bar buttons: sync, the quick switcher and the graph
/// (the desktops have them under the files).
pub(crate) fn phone_actions() -> impl IntoElement {
    view()
        .flex_row()
        .child(sync_button())
        .child(icon_button(icons::SEARCH, strings::SEARCH, || {
            store::set_switching(true)
        }))
        .child(icon_button(icons::NETWORK, strings::GRAPH, || {
            store::set_route(Route::Graph)
        }))
}
