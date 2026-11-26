//! The trash (MVP phase 4): what sync, and the phones, deleted, each with
//! where it was and the day it was deleted; restored there or deleted for
//! good, one by one or all at once.

use calliope_core::trash::Trashed;
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::{
    components::{ButtonVariant, action, button, muted, scroll_page},
    i18n, store,
};

pub(crate) fn trash_page() -> impl IntoElement {
    store::load_trash();
    // On the desktops only what sync deleted: the user's own go to the
    // system's trash.
    let keeps = match is_desktop() {
        true => i18n::trash_keeps_desktop(),
        false => t("trash_keeps"),
    };
    let content = view()
        .p_6()
        .gap_2()
        .items_start()
        .child(muted(keeps))
        .child(dynamic(|| {
            let items = store::trashed();
            untracked(|| match items.is_empty() {
                true => muted(t("trash_is_empty")).into_element(),
                false => list(items).into_element(),
            })
        }));
    scroll_page(t("trash"), store::back_to_note, content)
}

fn list(items: Vec<Trashed>) -> impl IntoElement {
    view()
        .w_full()
        .max_w(px(560.0))
        .pt_2()
        .child(items.into_iter().map(row))
        .child(view().flex_row().pt_4().child(button(
            ButtonVariant::Secondary,
            t("empty_trash"),
            store::clear_trash,
        )))
}

/// A file: its name, where it was and when it was deleted; restoring it or
/// deleting it for good.
fn row(item: Trashed) -> impl IntoElement {
    let name = item.origin.rsplit('/').next().unwrap_or(&item.origin);
    let detail = i18n::deleted_from(&item.origin, &item.deleted.to_string());
    let restored = item.clone();
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .py_2()
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            view()
                .flex_1()
                .gap(px(2.0))
                .child(
                    text(name.to_owned())
                        .text_sm()
                        .text_color(move || theme().foreground),
                )
                .child(muted(detail)),
        )
        .child(action(t("restore"), move || {
            store::restore(restored.clone())
        }))
        .child(action(t("delete"), move || {
            store::delete_forever(item.clone())
        }))
}
