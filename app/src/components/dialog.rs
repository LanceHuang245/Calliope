//! Calliope's dialogs: the question before what cannot be undone (or what
//! makes something), and the one that only tells the user something. Both
//! draw the same card over the dimmed page.

use nsh_base::{dialog, theme};
use nsh_core::{element::AnyElement, prelude::*};

use super::{ButtonVariant, button, card, muted};
use crate::i18n::t;

/// A dialog asking before something is done, as the Rename dialog looks:
/// `title`, what it means (`description`, if any), Cancel and `action`.
/// `answer` hears whether the user goes on; Escape and the backdrop cancel.
pub(crate) fn confirm_dialog(
    title: impl Into<SharedString>,
    description: Option<SharedString>,
    action: impl Into<SharedString>,
    answer: impl Fn(bool) + Copy + 'static,
) -> impl IntoElement {
    let cancel = move || answer(false);
    let buttons = view()
        .flex_row()
        .justify_end()
        .gap_2()
        .child(button(ButtonVariant::Secondary, t("cancel"), cancel))
        .child(button(ButtonVariant::Primary, action, move || answer(true)));
    let description = description.map(|description| muted(description).into_element());
    modal(cancel, dialog_card(title, description, buttons))
}

/// A dialog telling the user something: `title`, what it means under it
/// and one button (`action`) closing it. Escape and the backdrop close it.
pub(crate) fn notice_dialog(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    action: impl Into<SharedString>,
    close: impl Fn() + Copy + 'static,
) -> impl IntoElement {
    let buttons =
        view()
            .flex_row()
            .justify_end()
            .child(button(ButtonVariant::Primary, action, close));
    let description: SharedString = description.into();
    let body = Some(muted(description).into_element());
    modal(close, dialog_card(title, body, buttons))
}

/// The layer a dialog is drawn in: the page behind it dimmed and closed by
/// Escape or a press on it, `content` over the dimming.
pub(crate) fn modal(
    close: impl Fn() + Copy + 'static,
    content: impl IntoElement,
) -> impl IntoElement {
    dialog()
        .animation(ModalAnimation::Fade)
        .on_cancel(close)
        .flex_1()
        .child(view().absolute().inset_0().bg(overlay).on_press(close))
        .child(content)
}

/// The card a dialog draws: `title`, what it means or the field it asks
/// for (`body`, if any) under it, and `buttons` at the end.
pub(crate) fn dialog_card(
    title: impl Into<SharedString>,
    body: Option<AnyElement>,
    buttons: impl IntoElement,
) -> impl IntoElement {
    let title: SharedString = title.into();
    card()
        .m(auto())
        .w_80()
        .gap_3()
        .p_5()
        .shadow_lg()
        .child(
            text(title)
                .text_lg()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(body)
        .child(buttons)
}

/// The dimming behind dialogs and menu sheets: the theme's `overlay`
/// (`theme.json`), the foreground faded where a theme has none.
pub(crate) fn overlay() -> AdaptiveColor {
    let theme = theme();
    theme
        .color("overlay")
        .unwrap_or_else(|| theme.foreground.opacity(0.4))
}
