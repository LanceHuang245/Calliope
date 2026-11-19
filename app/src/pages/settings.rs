//! Settings: the appearance, the vault, sync, and about the app.

use nsh_core::prelude::*;

use super::sync::sync_settings;
use crate::{
    components::{action, muted, page_bar, section, segmented},
    store::{self, Route, Theme},
    strings,
};

pub(crate) fn settings_page() -> impl IntoElement {
    let back = match is_desktop() {
        true => Route::Note,
        false => Route::Home,
    };
    let content = view()
        .p_6()
        .gap_2()
        .items_start()
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
        // The desktops open the trash from the sidebar.
        .child((!is_desktop()).then(|| action(strings::TRASH, || store::set_route(Route::Trash))))
        .child(sync_settings())
        .child(section(strings::ABOUT))
        .child(muted(strings::version()))
        .child(muted(strings::LICENSE))
        .child(action(strings::EXPORT_LOGS, store::export_logs));
    view()
        .flex_1()
        .child(page_bar(strings::SETTINGS, move || store::set_route(back)))
        .child(scroll().flex_1().child(content))
}
