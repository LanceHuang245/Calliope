//! The app shell: the layout of each device idiom, with no page logic.
//!
//! - Desktop (Windows, macOS, Linux): a sidebar (files, search, backlinks)
//!   beside the workspace of open notes.
//! - Phone and pad (iOS, Android): the system tab bar (`nsh/mobile`'s
//!   `TabBar`: Liquid Glass on iOS 26, Material 3 on Android) over
//!   navigation stacks of pages.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::pages::note;

pub(crate) fn shell() -> impl IntoElement {
    if is_desktop() {
        desktop().into_element()
    } else {
        mobile().into_element()
    }
}

fn desktop() -> impl IntoElement {
    view()
        .flex_1()
        .flex_row()
        .child(sidebar())
        .child(view().flex_1().child(note::note_page()))
}

fn sidebar() -> impl IntoElement {
    view()
        .w(px(260.0))
        .p_4()
        .gap_2()
        .bg(move || theme().sidebar)
        .border_r_1()
        .border_color(move || theme().sidebar_border)
        .child(
            text("Calliope")
                .text_size(px(17.0))
                .text_color(move || theme().sidebar_foreground),
        )
        .child(
            text("Vault")
                .text_sm()
                .text_color(move || theme().muted_foreground),
        )
}

fn mobile() -> impl IntoElement {
    view().flex_1().p_safe_area().child(note::note_page())
}
