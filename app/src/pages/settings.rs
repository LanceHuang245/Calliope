//! Settings: the appearance, the vault, and about the app.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    components::{action, icon_button, icons, section, segmented},
    store::{self, Route, Theme},
    strings,
};

pub(crate) fn settings_page() -> impl IntoElement {
    let back = match is_desktop() {
        true => Route::Note,
        false => Route::Files,
    };
    scroll().flex_1().child(
        view()
            .p_6()
            .gap_2()
            .items_start()
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(icon_button(icons::CHEVRON_LEFT, strings::BACK, move || {
                        store::set_route(back)
                    }))
                    .child(
                        text(strings::SETTINGS)
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(move || theme().foreground),
                    ),
            )
            .child(section(strings::APPEARANCE))
            .child(segmented(
                vec![
                    (Theme::System, strings::SYSTEM),
                    (Theme::Light, strings::LIGHT),
                    (Theme::Dark, strings::DARK),
                ],
                store::theme,
                store::set_theme,
            ))
            .child(section(strings::VAULT))
            .child(muted(store::vault_path))
            // The phones keep their vault in the app's sandbox.
            .child(is_desktop().then(|| {
                view()
                    .flex_row()
                    .child(action(strings::OPEN_FOLDER, || store::pick_vault(false)))
                    .child(action(strings::NEW_VAULT, || store::pick_vault(true)))
            }))
            .child(section(strings::ABOUT))
            .child(muted(strings::version()))
            .child(muted(strings::LICENSE))
            .child(action(strings::EXPORT_LOGS, store::export_logs)),
    )
}

/// Secondary text.
fn muted(content: impl IntoReactive<SharedString>) -> impl IntoElement {
    text(content)
        .text_sm()
        .text_color(move || theme().muted_foreground)
}
