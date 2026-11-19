//! The layout of a page: its top bar on the desktops, its page of the
//! navigation stack on the phones, and the phones' floating toolbar.

use nsh_base::{nav_page, theme, toolbar};
use nsh_core::{GlassEffect, element::AnyElement, platform::Os, prelude::*};

use super::{icon_button, icons};
use crate::strings;

/// A page: on the desktops `content` under its top bar ([`page_bar`], the
/// `actions` after the title); on the phones a page of the navigation
/// stack, whose system header shows the `title` and the `actions`, and
/// whose back (the header's, a swipe, the system's) pops it and goes
/// `back`.
pub(crate) fn page(
    title: &'static str,
    back: impl Fn() + 'static,
    actions: Option<AnyElement>,
    content: impl IntoElement,
) -> AnyElement {
    if is_desktop() {
        let bar = page_bar(title, back).child(actions);
        return view().flex_1().child(bar).child(content).into_element();
    }
    let content = view().flex_1().pb_safe_area().child(content);
    let page = nav_page().title(title).on_dismiss(back).child(content);
    match actions {
        Some(actions) => page.header_right(actions).into_element(),
        None => page.into_element(),
    }
}

/// A page's top bar, as the note's: the button that goes `back`, then the
/// `title` taking the room left; a page adds its own buttons after it.
fn page_bar(title: &'static str, back: impl Fn() + 'static) -> View {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(icon_button(icons::CHEVRON_LEFT, strings::BACK, back))
        .child(
            text(title)
                .flex_1()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
}

/// The room the [`floating_toolbar`] takes over the bottom of a phone's
/// page: its bar (52 points) 16 points over the edge, and 8 to spare. What
/// scrolls under it ends with as much, to scroll clear of it.
pub(crate) const TOOLBAR_ROOM: f32 = 76.0;

/// A toolbar floating over the bottom of a phone's page, in each system's
/// look: Liquid Glass on iOS 26 (a blur of what is behind it before),
/// Material 3's floating toolbar on Android (a raised, opaque surface).
pub(crate) fn floating_toolbar(buttons: impl IntoElement) -> impl IntoElement {
    let bar = toolbar()
        .flex_row()
        .items_center()
        .gap_1()
        .p_1()
        .rounded_full()
        .shadow_lg()
        .child(buttons);
    let bar = match os() {
        Os::Ios => bar
            .glass_effect(GlassEffect::Regular)
            .backdrop_blur(px(20.0)),
        _ => bar
            .bg(move || theme().popover)
            .border_1()
            .border_color(move || theme().border),
    };
    view()
        .absolute()
        .left_0()
        .right_0()
        .bottom(px(16.0))
        .items_center()
        .child(bar)
}
