//! Calliope's own styled controls, built on `nsh-base`'s unstyled behavior
//! (buttons, inputs, menus, dialogs, tree rows, tabs), drawn with the
//! theme's tokens (`theme.json`). Pages use these instead of styling
//! `nsh-base` themselves.

mod dialog;
pub(crate) mod icons;
mod page;

use nsh_base::{ButtonElement, input, menu_item, tab, tabs, theme};
use nsh_core::{
    GlassEffect,
    element::{AnyElement, StyleRefinement},
    platform::Os,
    prelude::*,
};

pub(crate) use self::dialog::{confirm_dialog, dialog_card, modal, notice_dialog, overlay};
use self::icons::icon;
pub(crate) use self::page::{
    BAR_HEIGHT, TOOLBAR_ROOM, desktop_layout, fab, floating_surface, floating_toolbar, page,
    scroll_page,
};

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
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
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
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    icon_toggle(paths, label, || false, on_press)
}

/// An [`icon_button`] that turns something on and off: filled, its icon
/// in the foreground, and selected to screen readers while `on`.
pub(crate) fn icon_toggle(
    paths: &'static str,
    label: impl Into<SharedString>,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
    view().tooltip(tooltip(label.clone())).child(
        toggle(paths, label, on, on_press, 16.0)
            .active(|s| s.bg(theme().accent))
            .p(px(6.0))
            .rounded(move || theme().radius),
    )
}

/// The icon in a [`toolbar_toggle`], inside 12 points of padding a side.
const TOOLBAR_ICON: f32 = 20.0;

/// How wide and tall a [`toolbar_toggle`] is: the least both systems ask a
/// finger to hit. The [`floating_toolbar`] counts on it to fit a phone.
pub(crate) const TOOLBAR_BUTTON: f32 = TOOLBAR_ICON + 24.0;

/// A button of a [`floating_toolbar`]: an [`icon_toggle`] a finger hits
/// ([`TOOLBAR_BUTTON`] points), round.
pub(crate) fn toolbar_toggle(
    paths: &'static str,
    label: impl Into<SharedString>,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let button = toggle(paths, label, on, on_press, TOOLBAR_ICON)
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
    label: impl Into<SharedString>,
    on: impl Fn() -> bool + Copy + 'static,
    on_press: impl Fn() + 'static,
    size: f32,
) -> ButtonElement {
    let label: SharedString = label.into();
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

fn tooltip(label: impl Into<SharedString>) -> AnyElement {
    let label: SharedString = label.into();
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
    label: impl Into<SharedString>,
    destructive: bool,
    on_select: impl Fn() + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
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
    options: Vec<(T, String)>,
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

pub(crate) fn field(placeholder: impl Into<SharedString>) -> nsh_base::Input {
    let placeholder = placeholder.into().to_string();
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

/// How a palette's (the quick switcher's, the command palette's) search
/// field and rows look.
pub(crate) const PALETTE_SEARCH: &str = "px-3 py-2 text-sm";
pub(crate) const PALETTE_ITEM: &str = "px-3 py-2 rounded-md text-sm focus:bg-accent";
/// How tall a palette's rows get before they scroll: more than the window
/// shows of a long list, and only the rows in view are made.
pub(crate) const PALETTE_LIST: &str = "max-h-[320px]";

/// The panel a palette opens in, over the dimmed page: both palettes look
/// and sit alike, and leave 16 points on both sides when the phone's
/// viewport is narrow.
pub(crate) fn palette_panel() -> View {
    let panel = card()
        .w_96()
        .mx(auto())
        .mt_20()
        .max_h_96()
        .p_1()
        .shadow_lg()
        // Nothing of the rows is drawn past the panel, scrolled or not.
        .overflow_hidden();
    match desktop_layout() {
        true => panel,
        false => panel.max_w(move || px((window().width.0 - 32.0).max(0.0))),
    }
}

/// A raised surface that sets a group apart from what is around it: the
/// app's background and a light shadow, no border.
pub(crate) fn card() -> View {
    view()
        .bg(move || theme().background)
        .rounded(move || theme().radius)
        .shadow_sm()
}

/// Quiet text: a hint, a state, a field's label.
pub(crate) fn muted(content: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(content)
        .text_sm()
        .text_color(move || theme().muted_foreground)
}

/// A form's field: `content` (an input, a switch) under its `label`, which
/// a red `*` comes before when the field is `required`.
pub(crate) fn labeled(
    label: impl Into<SharedString>,
    required: bool,
    content: impl IntoElement,
) -> View {
    let label: SharedString = label.into();
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

pub(crate) fn section(title: impl Into<SharedString>) -> impl IntoElement {
    let title: SharedString = title.into();
    text(title)
        .text_sm()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().muted_foreground)
        .mt_4()
}
