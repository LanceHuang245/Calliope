//! The desktops' status bar (MVP 5.2), floating in the window's bottom
//! right corner as Obsidian's does: how sync goes, while it is on; a press
//! syncs.

use calliope_sync::Provider;
use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::{
    components::icons::{self, icon},
    store::{self, SyncStatus},
    strings,
};

pub(crate) fn status_bar() -> impl IntoElement {
    dynamic(|| (store::sync_config().provider != Provider::Off).then(|| untracked(bar)))
}

fn bar() -> impl IntoElement {
    button()
        .absolute()
        .bottom_0()
        .right_0()
        .flex_row()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .bg(move || theme().sidebar)
        .border_t_1()
        .border_l_1()
        .border_color(move || theme().border)
        .rounded_tl(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .accessibility_label(strings::SYNC_NOW)
        .on_press(store::sync_now)
        .child(
            icon(icons::REFRESH)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            text(move || match store::sync_status() {
                SyncStatus::Idle => strings::NOT_SYNCED.to_owned(),
                SyncStatus::Running(done, total) => strings::syncing(done, total),
                SyncStatus::Done(time) => strings::synced_at(&time),
                SyncStatus::Failed(_) => strings::SYNC_FAILED.to_owned(),
            })
            .text_xs()
            .text_color(move || theme().muted_foreground),
        )
}
