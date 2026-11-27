//! The desktops' status bar (MVP 5.2), floating in the window's bottom
//! right corner: the grammar issue count and the sync state when enabled.
//! The sync button also starts a sync.

use calliope_sync::Provider;
use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::{
    components::icons::{self, icon},
    i18n,
    store::{self, SyncStatus},
};

pub(crate) fn status_bar() -> impl IntoElement {
    let syncing = || store::sync_config().provider != Provider::Off;
    #[cfg(any(target_os = "windows", target_os = "android", target_os = "ios"))]
    let grammar_count = {
        let count = computed(store::grammar::count);
        move || count.get()
    };
    #[cfg(not(any(target_os = "windows", target_os = "android", target_os = "ios")))]
    let grammar_count = || None::<usize>;
    dynamic(move || {
        let sync = syncing();
        let count = grammar_count();
        (sync || count.is_some()).then(|| {
            view()
                .absolute()
                .bottom_0()
                .right_0()
                .flex_row()
                .items_center()
                .bg(move || theme().sidebar)
                .border_t_1()
                .border_l_1()
                .border_color(move || theme().border)
                .rounded_tl(move || theme().radius)
                .overflow_hidden()
                .child(count.map(|count| {
                    text(i18n::grammar_issues(count))
                        .px_2()
                        .py_1()
                        .text_xs()
                        .text_color(move || theme().muted_foreground)
                }))
                .child(sync.then(sync_button))
        })
    })
}

fn sync_button() -> impl IntoElement {
    button()
        .flex_row()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .hover(|s| s.bg(theme().accent))
        .accessibility_label(t("sync_now"))
        .on_press(store::sync_now)
        .child(
            icon(icons::REFRESH)
                .w(px(12.0))
                .h(px(12.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(
            text(move || match store::sync_status() {
                SyncStatus::Idle => t("not_synced").to_owned(),
                SyncStatus::Running(done, total) => i18n::syncing(done, total),
                SyncStatus::Done(time, 0) => i18n::synced_at(&time),
                SyncStatus::Done(time, n) => i18n::synced_conflicts(&time, n),
                SyncStatus::Failed(_) => t("sync_failed").to_owned(),
            })
            .text_xs()
            .text_color(move || match store::sync_status() {
                SyncStatus::Done(_, 1..) | SyncStatus::Failed(_) => theme().destructive,
                _ => theme().muted_foreground,
            }),
        )
}
