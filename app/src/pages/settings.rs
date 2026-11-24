//! Settings categories share their contents across desktop and mobile.

use nsh_base::theme;
use nsh_core::{element::AnyElement, prelude::*};

use super::sync::sync_settings;
use crate::{
    components::{action, icons, labeled, muted, page, scroll_page, section, segmented},
    store::{self, Route, SettingsCategory, Theme},
    strings,
};

pub(crate) const CATEGORIES: [SettingsCategory; 3] = [
    SettingsCategory::Appearance,
    SettingsCategory::Sync,
    SettingsCategory::About,
];

impl SettingsCategory {
    fn title(self) -> &'static str {
        match self {
            Self::Appearance => strings::APPEARANCE,
            Self::Sync => strings::SYNC,
            Self::About => strings::ABOUT,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Appearance => icons::SUN,
            Self::Sync => icons::REFRESH,
            Self::About => icons::INFO,
        }
    }
}

fn selected_category() -> SettingsCategory {
    match store::route() {
        Route::SettingsDetail(category) => category,
        _ => SettingsCategory::Appearance,
    }
}

pub(crate) fn desktop_settings() -> impl IntoElement {
    // Only the right pane changes when a category is selected.
    let body = view()
        .flex_1()
        .flex_row()
        .child(
            view()
                .w(px(160.0))
                .flex_shrink_0()
                .border_r_1()
                .border_color(move || theme().border)
                .p_2()
                .gap_1()
                .child(CATEGORIES.map(|category| category_button(category, true))),
        )
        .child(view().flex_1().min_w(px(0.0)).child(dynamic(move || {
            let category = selected_category();
            untracked(|| scroll().flex_1().child(category_content(category)))
        })));
    page(strings::SETTINGS, store::back_to_note, None, body)
}

pub(crate) fn settings_categories() -> impl IntoElement {
    scroll_page(
        strings::SETTINGS,
        store::back_to_note,
        view()
            .p_6()
            .gap_1()
            .child(CATEGORIES.map(|category| category_button(category, false))),
    )
}

pub(crate) fn settings_detail(category: SettingsCategory) -> impl IntoElement {
    scroll_page(
        category.title(),
        || store::set_route(Route::Settings),
        category_content(category),
    )
}

fn category_button(category: SettingsCategory, desktop: bool) -> impl IntoElement {
    let active = move || desktop && selected_category() == category;
    nsh_base::button()
        .w_full()
        .flex_row()
        .items_center()
        .gap_2()
        .px_3()
        .py_3()
        .rounded(move || theme().radius)
        .accessibility_label(category.title())
        .selected(active)
        .hover(|s| s.bg(theme().accent))
        .active(|s| s.bg(theme().accent))
        .style_when(active, |s| s.bg(theme().accent))
        .on_press(move || store::set_route(Route::SettingsDetail(category)))
        .child(icons::icon(category.icon()).text_color(move || theme().foreground))
        .child(
            text(category.title())
                .flex_1()
                .text_color(move || theme().foreground),
        )
        .child((!desktop).then(|| {
            icons::icon(icons::CHEVRON_RIGHT).text_color(move || theme().muted_foreground)
        }))
}

fn category_content(category: SettingsCategory) -> impl IntoElement {
    let content: AnyElement = match category {
        SettingsCategory::Appearance => labeled(
            strings::THEME,
            false,
            segmented(
                vec![
                    (Theme::System, strings::SYSTEM),
                    (Theme::Light, strings::LIGHT),
                    (Theme::Dark, strings::DARK),
                ],
                store::theme,
                store::set_theme,
            ),
        )
        .into_element(),
        SettingsCategory::Sync => view()
            .w_full()
            .gap_2()
            .child(section(strings::VAULT))
            .child(muted(store::vault_path))
            // Phones keep their vault in the app's sandbox.
            .child(is_desktop().then(|| {
                view()
                    .flex_row()
                    .child(action(strings::OPEN_FOLDER, || store::pick_vault(false)))
                    .child(action(strings::NEW_VAULT, || store::pick_vault(true)))
            }))
            .child(sync_settings())
            .into_element(),
        SettingsCategory::About => view()
            .gap_2()
            .items_start()
            .child(muted(strings::version()))
            .child(muted(strings::LICENSE))
            .child(action(strings::EXPORT_LOGS, store::export_logs))
            .into_element(),
    };
    view().p_6().gap_2().items_start().child(content)
}
