//! A note: reading, editing, or both side by side (the view mode).
//!
//! Reading draws the Markdown with `nsh-base`'s `TextView` (formulas
//! typeset natively). Editing will use its `Editor` (source) or
//! `RichEditor` (live preview).

use nsh_base::{text_view, theme};
use nsh_core::prelude::*;

use crate::store;

pub(crate) fn note_page() -> impl IntoElement {
    scroll().flex_1().child(
        text_view()
            .markdown(store::current_markdown)
            .p_6()
            .text_color(move || theme().foreground),
    )
}
