//! Calliope's own styled controls, built on `nsh-base`'s unstyled behavior
//! (buttons, inputs, menus, dialogs, tree rows, tabs), drawn with the
//! theme's tokens (`theme.json`). Pages use these instead of styling
//! `nsh-base` themselves.

pub(crate) mod icons;

use nsh_base::{button, input, menu_item, tab, tabs, theme};
use nsh_core::{element::StyleRefinement, prelude::*};

use self::icons::icon;

/// A quiet text button: no surface until hovered or pressed.
pub(crate) fn action(
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .active(|s| s.bg(theme().accent))
        .on_press(on_press)
        .child(
            text(label.into())
                .text_sm()
                .text_color(move || theme().foreground),
        )
}

/// The main action of a page or a dialog, in the theme's `primary`.
pub(crate) fn primary(label: &'static str, on_press: impl Fn() + 'static) -> impl IntoElement {
    button()
        .px_4()
        .py_2()
        .rounded(move || theme().radius)
        .bg(move || theme().primary)
        .active(|s| s.opacity(0.8))
        .on_press(on_press)
        .child(
            text(label)
                .text_sm()
                .text_color(move || theme().primary_foreground),
        )
}

/// A quiet icon button, named by `label` for screen readers and in a
/// tooltip on the desktops.
pub(crate) fn icon_button(
    paths: &'static str,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view().tooltip(tooltip(label)).child(
        button()
            .p(px(6.0))
            .rounded(move || theme().radius)
            .hover(|s| s.bg(theme().accent))
            .active(|s| s.bg(theme().accent))
            .accessibility_label(label)
            .on_press(on_press)
            .child(icon(paths).text_color(move || theme().muted_foreground)),
    )
}

fn tooltip(label: &'static str) -> AnyElement {
    view()
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .bg(move || theme().popover)
        .border_1()
        .border_color(move || theme().border)
        .child(
            text(label)
                .text_xs()
                .text_color(move || theme().popover_foreground),
        )
        .into_element()
}

/// An item of a menu: an icon and a label; `destructive` in the theme's
/// color for it.
pub(crate) fn menu_entry(
    paths: &'static str,
    label: &'static str,
    destructive: bool,
    on_select: impl Fn() + 'static,
) -> impl IntoElement {
    let color = move || match destructive {
        true => theme().destructive,
        false => theme().popover_foreground,
    };
    menu_item()
        .on_select(on_select)
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .py(px(6.0))
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .child(icon(paths).text_color(color))
        .child(text(label).text_sm().text_color(color))
}

/// The surface of a menu: the popup on the desktops, the sheet on phones.
pub(crate) fn menu_popup(s: StyleRefinement) -> StyleRefinement {
    s.min_w(px(176.0))
        .p_1()
        .rounded(theme().radius)
        .border_1()
        .border_color(theme().border)
        .bg(theme().popover)
        .shadow_lg()
}

/// A row of options, the current one highlighted (the view mode, the
/// theme).
pub(crate) fn segmented<T: Copy + PartialEq + 'static>(
    options: Vec<(T, &'static str)>,
    current: impl Fn() -> T + Copy + 'static,
    choose: impl Fn(T) + Copy + 'static,
) -> impl IntoElement {
    // The selected option's highlight is the tab list's indicator, which
    // slides to the next option natively (a spring).
    tabs()
        .flex_row()
        .p(px(2.0))
        .gap(px(2.0))
        .rounded(move || theme().radius)
        .bg(move || theme().muted)
        .indicator_style(move |s| {
            s.top(px(2.0))
                .bottom(px(2.0))
                .rounded(theme().radius)
                .bg(theme().background)
                .shadow_sm()
        })
        .child(options.into_iter().map(move |(value, label)| {
            let on = move || current() == value;
            tab()
                .selected(on)
                .px_3()
                .py_1()
                .on_press(move |_: &()| choose(value))
                .child(text(label).text_sm().text_color(move || match on() {
                    true => theme().foreground,
                    false => theme().muted_foreground,
                }))
        }))
}

pub(crate) fn field(placeholder: &'static str) -> nsh_base::Input {
    input()
        .placeholder(placeholder)
        .text_sm()
        .px_2()
        .py_1()
        .border_1()
        .border_color(move || theme().input)
        .rounded(move || theme().radius)
        .text_color(move || theme().foreground)
}

pub(crate) fn banner(message: impl IntoReactive<SharedString>) -> View {
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .bg(move || theme().muted)
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            text(message)
                .flex_1()
                .text_sm()
                .text_color(move || theme().foreground),
        )
}

/// A raised surface that sets a group apart from what is around it: the
/// app's background and a light shadow, no border.
pub(crate) fn card() -> View {
    view()
        .bg(move || theme().background)
        .rounded(move || theme().radius)
        .shadow_sm()
}

/// The dimming behind dialogs and menu sheets: the theme's `overlay`
/// (`theme.json`), the foreground faded where a theme has none.
pub(crate) fn overlay() -> AdaptiveColor {
    let theme = theme();
    theme
        .color("overlay")
        .unwrap_or_else(|| theme.foreground.opacity(0.4))
}

pub(crate) fn section(title: &'static str) -> impl IntoElement {
    text(title)
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().muted_foreground)
        .mt_4()
}
