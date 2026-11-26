//! The layout of a page: its top bar on the desktops, its page of the
//! navigation stack on the phones, and the phones' floating toolbar.

use nsh_base::{nav_page, theme, toolbar};
use nsh_core::{
    GlassEffect,
    element::AnyElement,
    platform::{Idiom, Os},
    prelude::*,
};

use super::{icon_button, icons, ios_glass_press};
use crate::i18n::t;

/// How wide a tablet takes the desktops' layout (MVP phase 4).
const TABLET_DESKTOP_WIDTH: f32 = 900.0;

/// Whether the desktops' layout shows (sidebar, cards, Split): on the
/// desktops, and on a tablet at least [`TABLET_DESKTOP_WIDTH`] wide; a
/// phone keeps its own, turned or not. It follows a tablet turned, or put
/// beside another app.
pub(crate) fn desktop_layout() -> bool {
    is_desktop() || (idiom() == Idiom::Pad && window().width >= px(TABLET_DESKTOP_WIDTH))
}

/// A page: on the desktops `content` under its top bar ([`page_bar`], the
/// `actions` after the title); on the phones a page of the navigation
/// stack, whose system header shows the `title` and the `actions`, and
/// whose back (the header's, a swipe, the system's) pops it and goes
/// `back`.
pub(crate) fn page(
    title: impl Into<SharedString>,
    back: impl Fn() + 'static,
    actions: Option<AnyElement>,
    content: impl IntoElement,
) -> AnyElement {
    let title: SharedString = title.into();
    if desktop_layout() {
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

/// A [`page`] whose `content` scrolls: on iOS its title is large and
/// shrinks into the header as the content scrolls (MVP 5.4), which takes
/// the scroll view straight in the page. Only iOS: nsh lays a large
/// title's content out under the header everywhere, and only UIKit moves
/// it clear.
pub(crate) fn scroll_page(
    title: impl Into<SharedString>,
    back: impl Fn() + 'static,
    content: impl IntoElement,
) -> AnyElement {
    let title: SharedString = title.into();
    let content = scroll().flex_1().child(content);
    if desktop_layout() {
        return page(title, back, None, content);
    }
    (nav_page().title(title).header_large_title(os() == Os::Ios))
        .pb_safe_area()
        .on_dismiss(back)
        .child(content)
        .into_element()
}

/// A page's top bar, as the note's: the button that goes `back`, then the
/// `title` taking the room left; a page adds its own buttons after it.
fn page_bar(title: SharedString, back: impl Fn() + 'static) -> View {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child(icon_button(icons::CHEVRON_LEFT, t("back"), back))
        .child(
            text(title)
                .flex_1()
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
}

/// The room the [`floating_toolbar`] takes over the bottom of a phone's
/// page: its bar ([`BAR_HEIGHT`]) 16 points over the edge, and 8 to spare.
/// What scrolls under it ends with as much, to scroll clear of it.
pub(crate) const TOOLBAR_ROOM: f32 = 76.0;

/// The [`floating_toolbar`]'s height: its buttons (44 points) and its
/// padding; the [`fab`] beside it is as tall.
pub(crate) const BAR_HEIGHT: f32 = 52.0;

/// The surface of what floats over a page (the floating toolbar, the
/// notices), in each system's look: Liquid Glass on iOS 26 (a blur of what
/// is behind it before), an opaque raised surface elsewhere (Material's on
/// Android).
pub(crate) fn floating_surface<E: Styled>(element: E) -> E {
    match os() {
        Os::Ios => element
            .glass_effect(GlassEffect::Regular)
            .backdrop_blur(px(20.0)),
        _ => element
            .bg(move || theme().popover)
            .border_1()
            .border_color(move || theme().border),
    }
}

/// A toolbar floating over the bottom of a phone's page (Material 3's
/// floating toolbar on Android), with the [`fab`] beside it on the right.
pub(crate) fn floating_toolbar(
    buttons: impl IntoElement,
    beside: impl IntoElement,
    native_glass: bool,
) -> impl IntoElement {
    let bar = toolbar()
        .flex_row()
        .items_center()
        .gap_1()
        .rounded_full()
        .shadow_lg()
        .child(buttons);
    let bar = if native_glass {
        bar
    } else {
        floating_surface(bar.p_1())
    };
    view()
        .absolute()
        .left_0()
        .right_0()
        .bottom(px(16.0))
        .flex_row()
        .items_center()
        .justify_center()
        .gap_3()
        .child(bar)
        .child(beside)
}

/// A separate round action as tall as the toolbar, using each platform's surface.
pub(crate) fn fab(
    paths: &'static str,
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
    let button = nsh_base::button()
        .w(px(BAR_HEIGHT))
        .h(px(BAR_HEIGHT))
        .items_center()
        .justify_center()
        .rounded_full()
        .shadow_lg()
        .accessibility_label(label)
        .on_press(on_press)
        .child(
            icons::icon(paths)
                .w(px(24.0))
                .h(px(24.0))
                .text_color(move || match os() {
                    Os::Ios => theme().foreground,
                    _ => theme().primary_foreground,
                }),
        );
    match os() {
        Os::Ios => floating_surface(ios_glass_press(button)),
        _ => button
            .bg(move || theme().primary)
            .active(|s| s.opacity(0.8)),
    }
}
