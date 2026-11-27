//! Settings categories share their contents across desktop and mobile.

use nsh_base::{MenuButtonItem, menu_button, theme};
use nsh_core::{
    element::AnyElement,
    mobile::{SegmentedControlItem, segmented_control},
    platform::Os,
    prelude::*,
};

use super::sync::sync_settings;
use crate::i18n::t;

use crate::{
    components::{action, icons, labeled, muted, page, scroll_page, section, segmented},
    i18n::{self, Language},
    store::{self, Route, SettingsCategory, Theme},
};

pub(crate) const CATEGORIES: [SettingsCategory; 3] = [
    SettingsCategory::Appearance,
    SettingsCategory::Sync,
    SettingsCategory::About,
];

impl SettingsCategory {
    fn title(self) -> String {
        match self {
            Self::Appearance => t("appearance"),
            #[cfg(target_os = "windows")]
            Self::Editor => t("editor_settings"),
            Self::Sync => t("sync"),
            Self::About => t("about"),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Appearance => icons::SUN,
            #[cfg(target_os = "windows")]
            Self::Editor => icons::PENCIL,
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
    // Only the right pane changes when a category is selected. The
    // editor's settings (where grammar checking is) come before Sync.
    let mut categories = vec![SettingsCategory::Appearance];
    #[cfg(target_os = "windows")]
    categories.extend(store::grammar::supported().then_some(SettingsCategory::Editor));
    categories.extend([SettingsCategory::Sync, SettingsCategory::About]);
    let sidebar = view()
        .w(px(160.0))
        .flex_shrink_0()
        .border_r_1()
        .border_color(move || theme().border)
        .p_2()
        .gap_1()
        .child(
            categories
                .into_iter()
                .map(|category| category_button(category, true)),
        );
    let body =
        view()
            .flex_1()
            .flex_row()
            .child(sidebar)
            .child(view().flex_1().min_w(px(0.0)).child(dynamic(move || {
                let category = selected_category();
                untracked(|| scroll().flex_1().child(category_content(category)))
            })));
    page(t("settings"), store::back_to_note, None, body)
}

pub(crate) fn settings_categories() -> impl IntoElement {
    scroll_page(
        t("settings"),
        store::back_to_note,
        view()
            .p_6()
            .px(px(if os() == Os::Ios { 12.0 } else { 24.0 }))
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
        SettingsCategory::Appearance => appearance_settings().into_element(),
        #[cfg(target_os = "windows")]
        SettingsCategory::Editor => nsh_base::switch()
            .accessibility_label(t("live_grammar"))
            .checked(store::grammar::live)
            .on_change(store::grammar::set_live)
            .flex_row()
            .items_center()
            .gap_2()
            .track_style(|s| {
                s.w(px(32.0))
                    .p(px(2.0))
                    .rounded_full()
                    .bg(theme().border)
                    .checked(|s| s.bg(theme().primary))
            })
            .thumb_style(|s| {
                s.size(px(14.0))
                    .rounded_full()
                    .bg(theme().foreground)
                    .checked(|s| s.translate_x(px(14.0)).bg(theme().primary_foreground))
            })
            .child(text(t("live_grammar")))
            .into_element(),
        SettingsCategory::Sync => view()
            .w_full()
            .gap_2()
            .child(section(t("vault")))
            .child(muted(store::vault_path))
            // Phones keep their vault in the app's sandbox.
            .child(is_desktop().then(|| {
                view()
                    .flex_row()
                    .child(action(t("open_folder"), || store::pick_vault(false)))
                    .child(action(t("new_vault"), || store::pick_vault(true)))
            }))
            .child(sync_settings())
            .into_element(),
        SettingsCategory::About => view()
            .gap_2()
            .items_start()
            .child(muted(i18n::version()))
            .child(muted(t("license")))
            .child(action(t("export_logs"), store::export_logs))
            .into_element(),
    };
    view().p_6().gap_2().items_start().child(content)
}

/// The theme, and the interface's language: a pick is kept at once, and
/// speaks from the next start on.
fn appearance_settings() -> impl IntoElement {
    view()
        .w_full()
        .gap_3()
        .items_start()
        .child(labeled(
            t("theme"),
            false,
            settings_segmented(
                vec![
                    (Theme::System, t("system")),
                    (Theme::Light, t("light")),
                    (Theme::Dark, t("dark")),
                ],
                store::theme,
                store::set_theme,
            ),
        ))
        .child(labeled(t("interface_language"), false, language_list()))
}

/// Keeps `language` on this device, and says what shows it: the pages
/// showing were made with the old texts.
fn choose_language(language: Language) {
    if i18n::save_language(language).is_err() {
        store::notify(t("language_save_failed"));
        return;
    }
    store::tell(
        t("language_saved_title"),
        t("language_saved_description"),
        t("ok"),
    );
}

/// The languages, each in its own language and the chosen one marked.
/// UIKit's own list on iOS (a pull-down menu); on Android, as on the
/// desktops, a list unfolding under the row.
fn language_list() -> AnyElement {
    if cfg!(target_os = "ios") && os() == Os::Ios {
        native_language_list().into_element()
    } else {
        unfolding_language_list().into_element()
    }
}

/// The row showing the choice, and the languages under it once it is
/// pressed; a pick is kept and folds the list back.
fn unfolding_language_list() -> impl IntoElement {
    let open = signal(false);
    view()
        .w(px(220.0))
        .gap_1()
        .child(
            nsh_base::button()
                .flex_row()
                .items_center()
                .gap_2()
                .px_2()
                .py_1()
                .border_1()
                .border_color(move || theme().input)
                .rounded(move || theme().radius)
                .accessibility_label(t("interface_language"))
                .on_press(move || open.update(|open| *open = !*open))
                .child(dynamic(|| {
                    text(i18n::saved_language().title())
                        .flex_1()
                        .text_sm()
                        .text_color(move || theme().foreground)
                }))
                .child(
                    icons::icon(icons::CHEVRON_DOWN).text_color(move || theme().muted_foreground),
                ),
        )
        .child(dynamic(move || {
            open.get().then(|| {
                view()
                    .p_1()
                    .gap(px(2.0))
                    .border_1()
                    .border_color(move || theme().border)
                    .rounded(move || theme().radius)
                    .child(Language::ALL.map(|language| language_row(language, open)))
                    .into_element()
            })
        }))
}

/// A language of the unfolding list: its own name, the chosen one on the
/// theme's accent as the settings' categories are.
fn language_row(language: Language, open: Signal<bool>) -> impl IntoElement {
    let on = move || i18n::saved_language() == language;
    nsh_base::button()
        .flex_row()
        .items_center()
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .accessibility_label(language.title())
        .selected(on)
        .hover(|s| s.bg(theme().accent))
        .active(|s| s.bg(theme().accent))
        .style_when(on, |s| s.bg(theme().accent))
        .on_press(move || {
            choose_language(language);
            open.set(false);
        })
        .child(
            text(language.title())
                .text_sm()
                .text_color(move || theme().foreground),
        )
}

/// The same choice in the system's list: UIKit's `UIMenu` on iOS, a check
/// mark on the chosen one.
fn native_language_list() -> impl IntoElement {
    menu_button()
        .accessibility_label(t("interface_language"))
        .items(move || {
            Language::ALL
                .into_iter()
                .map(|language| {
                    MenuButtonItem::new(language.code(), language.title())
                        .checked(language == i18n::saved_language())
                })
                .collect::<Vec<_>>()
        })
        .on_select(move |code: &SharedString| {
            if let Some(language) = Language::from_code(code) {
                choose_language(language);
            }
        })
        .w(px(220.0))
        .flex_row()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .border_1()
        .border_color(move || theme().input)
        .rounded(move || theme().radius)
        .child(dynamic(|| {
            text(i18n::saved_language().title())
                .flex_1()
                .text_sm()
                .text_color(move || theme().foreground)
        }))
        .child(icons::icon(icons::CHEVRON_DOWN).text_color(move || theme().muted_foreground))
}

// UIKit owns the selection appearance; each settings field still owns its value.
pub(crate) fn settings_segmented<T: Copy + PartialEq + 'static>(
    options: Vec<(T, String)>,
    current: impl Fn() -> T + Copy + 'static,
    choose: impl Fn(T) + Copy + 'static,
) -> AnyElement {
    if cfg!(target_os = "ios") && os() == Os::Ios {
        let selection = options.clone();
        segmented_control()
            .items(
                options
                    .iter()
                    .map(|(_, title)| SegmentedControlItem::new(title.clone()))
                    .collect::<Vec<_>>(),
            )
            .selected_index(move || {
                let value = current();
                selection
                    .iter()
                    .position(|(option, _)| *option == value)
                    .unwrap_or(selection.len()) as u32
            })
            .on_change(move |index: &u32| {
                if let Some(&(value, _)) = options.get(*index as usize) {
                    choose(value);
                }
            })
            .max_w(relative(1.0))
            .into_element()
    } else {
        segmented(options, current, choose).into_element()
    }
}
