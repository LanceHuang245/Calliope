//! Calliope's own styled controls, built on `nsh-base`'s unstyled behavior
//! (buttons, inputs, menus, dialogs, tree rows, tabs), drawn with the
//! theme's tokens (`theme.json`). Pages use these instead of styling
//! `nsh-base` themselves.

pub(crate) mod icons;
mod page;

use nsh_base::{ButtonElement, dialog, input, menu_item, tab, tabs, theme};
use nsh_core::{
    GlassEffect,
    element::{AnyElement, StyleRefinement},
    platform::Os,
    prelude::*,
};

use self::icons::icon;
pub(crate) use self::page::{
    BAR_HEIGHT, TOOLBAR_ROOM, desktop_layout, fab, floating_surface, floating_toolbar, page,
    scroll_page,
};
use crate::strings;

/// A quiet text button: no surface until hovered or pressed.
pub(crate) fn action(
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    nsh_base::button()
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

/// How a [`button`] stands out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonVariant {
    /// The main action of a page or a dialog, in the theme's `primary`.
    Primary,
    /// An action beside the main one (Cancel, Test connection), on the
    /// theme's `muted`.
    Secondary,
}

/// A button showing `label`, in its middle when the button is stretched;
/// every variant is the same size, so a row of them lines up.
pub(crate) fn button(
    variant: ButtonVariant,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let fill = move || match variant {
        ButtonVariant::Primary => theme().primary,
        ButtonVariant::Secondary => theme().muted,
    };
    let ink = move || match variant {
        ButtonVariant::Primary => theme().primary_foreground,
        ButtonVariant::Secondary => theme().foreground,
    };
    nsh_base::button()
        .items_center()
        .px_4()
        .py_2()
        .rounded(move || theme().radius)
        .bg(fill)
        .active(|s| s.opacity(0.8))
        .on_press(on_press)
        .child(text(label).text_sm().text_color(ink))
}

/// A quiet icon button, named by `label` for screen readers and in a
/// tooltip on the desktops.
pub(crate) fn icon_button(
    paths: &'static str,
    label: &'static str,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    icon_toggle(paths, label, || false, on_press)
}

/// An [`icon_button`] that turns something on and off: filled, its icon
/// in the foreground, and selected to screen readers while `on`.
pub(crate) fn icon_toggle(
    paths: &'static str,
    label: &'static str,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    view().tooltip(tooltip(label)).child(
        toggle(paths, label, on, on_press, 16.0)
            .active(|s| s.bg(theme().accent))
            .p(px(6.0))
            .rounded(move || theme().radius),
    )
}

/// A button of a [`floating_toolbar`]: an [`icon_toggle`] a finger hits
/// (44 points, the least both systems ask for), round.
pub(crate) fn toolbar_toggle(
    paths: &'static str,
    label: &'static str,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let button = toggle(paths, label, on, on_press, 20.0)
        .p(px(12.0))
        .rounded_full();
    if os() == Os::Ios {
        ios_glass_press(button)
    } else {
        button.active(|s| s.bg(theme().accent))
    }
}

// nsh's glass is a backdrop. Highlight the pressed control and animate
// its scale as the finger presses and releases it.
fn ios_glass_press(button: ButtonElement) -> ButtonElement {
    button
        .transition_transform()
        .duration(160.0)
        .ease_out()
        .active(|s| {
            s.scale(0.92)
                .glass_effect(GlassEffect::Regular)
                .glass_tint_color(theme().accent)
        })
}

/// The icon toggles' button: its icon `size` points square, filled while
/// `on`.
fn toggle(
    paths: &'static str,
    label: &'static str,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
    size: f32,
) -> ButtonElement {
    nsh_base::button()
        .hover(|s| s.bg(theme().accent))
        .style_when(on, |s| s.bg(theme().accent))
        .selected(on)
        .accessibility_label(label)
        .on_press(on_press)
        .child(
            icon(paths)
                .w(px(size))
                .h(px(size))
                .text_color(move || match on() {
                    true => theme().foreground,
                    false => theme().muted_foreground,
                }),
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

/// The surface of a menu: the popup on the desktops, the sheet on phones,
/// whose last item stays above the system's bars at the bottom (Android's
/// gesture bar, the iPhone's home indicator): nsh leaves that to the app.
pub(crate) fn menu_popup(s: StyleRefinement) -> StyleRefinement {
    let s = s
        .min_w(px(176.0))
        .p_1()
        .rounded(theme().radius)
        .border_1()
        .border_color(theme().border)
        .bg(theme().popover)
        .shadow_lg();
    match is_desktop() {
        true => s,
        false => s.pb_safe_area_plus(px(4.0)),
    }
}

/// A row of options, the current one highlighted (the view mode, the
/// theme).
pub(crate) fn segmented<T: Copy + PartialEq + 'static>(
    options: Vec<(T, &'static str)>,
    current: impl Fn() -> T + Copy + 'static,
    choose: impl Fn(T) + Copy + 'static,
) -> impl IntoElement {
    // The selected option's highlight is the tab list's indicator, which
    // slides to the next option natively (a spring). In a row of its own,
    // so it is as wide as its options, not the column it is in.
    let tabs = tabs()
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
        }));
    view().flex_row().child(tabs)
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

/// A dialog asking before something is done, as the Rename dialog looks:
/// `title`, what it means (`description`, if any), Cancel and `action`.
/// `answer` hears whether the user goes on; Escape and the backdrop cancel.
pub(crate) fn confirm_dialog(
    title: impl Into<SharedString>,
    description: Option<SharedString>,
    action: &'static str,
    answer: impl Fn(bool) + Copy + 'static,
) -> impl IntoElement {
    let cancel = move || answer(false);
    dialog()
        .animation(ModalAnimation::Fade)
        .on_cancel(cancel)
        .flex_1()
        .child(view().absolute().inset_0().bg(overlay).on_press(cancel))
        .child(
            card()
                .m(auto())
                .w_80()
                .gap_3()
                .p_5()
                .shadow_lg()
                .child(
                    text(title.into())
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(description.map(muted))
                .child(
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(button(ButtonVariant::Secondary, strings::CANCEL, cancel))
                        .child(button(ButtonVariant::Primary, action, move || answer(true))),
                ),
        )
}

/// The dimming behind dialogs and menu sheets: the theme's `overlay`
/// (`theme.json`), the foreground faded where a theme has none.
pub(crate) fn overlay() -> AdaptiveColor {
    let theme = theme();
    theme
        .color("overlay")
        .unwrap_or_else(|| theme.foreground.opacity(0.4))
}

/// Quiet text: a hint, a state, a field's label.
pub(crate) fn muted(content: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(content)
        .text_sm()
        .text_color(move || theme().muted_foreground)
}

/// A form's field: `content` (an input, a switch) under its `label`, which
/// a red `*` comes before when the field is `required`.
pub(crate) fn labeled(label: &'static str, required: bool, content: impl IntoElement) -> View {
    let star = text("*").text_sm().text_color(move || theme().destructive);
    view()
        .w_full()
        .max_w(px(420.0))
        .gap_1()
        .child(
            view()
                .flex_row()
                .gap_1()
                .child(required.then_some(star))
                .child(muted(label)),
        )
        .child(content)
}

pub(crate) fn section(title: &'static str) -> impl IntoElement {
    text(title)
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().muted_foreground)
        .mt_4()
}
