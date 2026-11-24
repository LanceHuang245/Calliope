//! The home page, where no note is open: a new note in its middle (the
//! download's progress while an empty vault syncs) and, on a phone, the
//! menu at the top and the floating toolbar at the bottom.

use calliope_sync::Provider;
use nsh_base::{progress, theme};
use nsh_core::{platform::Os, prelude::*};

use crate::{
    components::{
        ButtonVariant, button, desktop_layout, fab, floating_toolbar, icon_button, icons, muted,
        toolbar_toggle,
    },
    store::{self, Route, SyncStatus},
    strings,
};

pub(crate) fn home_page() -> impl IntoElement {
    view()
        .flex_1()
        .child((!desktop_layout()).then(|| {
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
                .child(when(downloading, download_progress).otherwise(|| {
                    view()
                        .items_center()
                        .gap_4()
                        .child(text(strings::NO_NOTE).text_color(move || theme().muted_foreground))
                        .child(button(
                            ButtonVariant::Primary,
                            strings::NEW_NOTE,
                            store::new_note,
                        ))
                })),
        )
        .child((!desktop_layout()).then(phone_toolbar))
}

/// Whether the vault is empty while it syncs: its first download (a
/// phone's vault made from sync), which shows its progress.
fn downloading() -> bool {
    !store::has_files() && matches!(store::sync_status(), SyncStatus::Running(..))
}

/// The files synced of how many, and a bar of them; moving with no end
/// while the run finds out how many (it lists the server first).
fn download_progress() -> impl IntoElement {
    let counts = || match store::sync_status() {
        SyncStatus::Running(done, total) => (done, total),
        _ => (0, 0),
    };
    let share = move || {
        let (done, total) = counts();
        match total {
            0 => 0.0,
            _ => done as f32 * 100.0 / total as f32,
        }
    };
    view()
        .w(px(240.0))
        .gap_2()
        .child(muted(move || {
            let (done, total) = counts();
            strings::syncing(done, total)
        }))
        .child(
            progress()
                .value(share)
                .indeterminate(move || counts().1 == 0)
                .track_style(|s| s.h(px(6.0)).rounded_full().overflow_hidden())
                .indicator_style(|s| s.h_full().rounded_full())
                .track_class("bg-muted")
                .indicator_class("bg-primary"),
        )
}

fn sync_enabled() -> bool {
    store::sync_config().provider != Provider::Off
}

/// Shows the desktop shortcut while sync is enabled, dimmed during a run.
pub(crate) fn sync_button() -> impl IntoElement {
    when(sync_enabled, || {
        let running = || matches!(store::sync_status(), SyncStatus::Running(..));
        view()
            .opacity(move || if running() { 0.5_f32 } else { 1.0 })
            .child(icon_button(icons::REFRESH, strings::SYNC, store::sync_now))
    })
}

/// The phone's menu button: the files slide in from the left.
pub(crate) fn menu_button() -> impl IntoElement {
    icon_button(icons::MENU, strings::MENU, || store::set_menu_open(true))
}

/// iOS's new note (MVP 5.4): a `+` at the end of the top bar, where its
/// navigation bars have it (Android's is the [`phone_toolbar`]'s FAB; the
/// desktops' layout has its own over the files).
pub(crate) fn ios_new_note() -> Option<impl IntoElement> {
    (os() == Os::Ios && !desktop_layout())
        .then(|| icon_button(icons::PLUS, strings::NEW_NOTE, store::new_note))
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
                .child(when(sync_enabled, move || {
                    toolbar_toggle(icons::REFRESH, strings::SYNC, syncing, store::sync_now)
                }))
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
