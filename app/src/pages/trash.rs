//! The trash (MVP phase 4): what sync, and the phones, deleted, each with
//! where it was and the day it was deleted; restored there or deleted for
//! good, one by one or all at once.

use calliope_core::trash::Trashed;
use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    components::{ButtonVariant, action, button, muted, scroll_page},
    store, strings,
};

pub(crate) fn trash_page() -> impl IntoElement {
    store::load_trash();
    // On the desktops only what sync deleted: the user's own go to the
    // system's trash.
    let keeps = match is_desktop() {
        true => strings::TRASH_KEEPS_DESKTOP,
        false => strings::TRASH_KEEPS,
    };
    let content = view()
        .p_6()
        .gap_2()
        .items_start()
        .child(muted(keeps))
        .child(dynamic(|| {
            let items = store::trashed();
            untracked(|| match items.is_empty() {
                true => muted(strings::TRASH_IS_EMPTY).into_element(),
                false => list(items).into_element(),
            })
        }));
    scroll_page(strings::TRASH, store::back_to_note, content)
}

fn list(items: Vec<Trashed>) -> impl IntoElement {
    view()
        .w_full()
        .max_w(px(560.0))
        .pt_2()
        .child(items.into_iter().map(row))
        .child(view().flex_row().pt_4().child(button(
            ButtonVariant::Secondary,
            strings::EMPTY_TRASH,
            store::clear_trash,
        )))
}

/// A file: its name, where it was and when it was deleted; restoring it or
/// deleting it for good.
fn row(item: Trashed) -> impl IntoElement {
    let name = item.origin.rsplit('/').next().unwrap_or(&item.origin);
    let detail = strings::deleted_from(&item.origin, &item.deleted.to_string());
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
        .child(action(strings::RESTORE, move || {
            store::restore(restored.clone())
        }))
        .child(action(strings::DELETE, move || {
            store::delete_forever(item.clone())
        }))
}
