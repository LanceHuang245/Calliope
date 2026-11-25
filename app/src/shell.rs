//! The app shell: the layout of each device idiom, with no page logic.
//!
//! - Desktop (Windows, macOS, Linux; tablets from 900 points wide): the
//!   app's own title bar over a sidebar (the files and the app's actions)
//!   beside the workspace (the note, or the settings) and the open note's
//!   backlinks, with the app's shortcuts and a status bar in the bottom
//!   right corner. A tablet keeps the system's status bar and back, and
//!   has no title bar or shortcuts.
//! - Phone (iOS, Android): one screen at a time, the files, a note or the
//!   settings (the system tab bar and navigation stacks come later).

use std::time::Duration;

use nsh_base::{SheetEvent, nav_page, nav_stack, sheet, theme};
use nsh_core::{
    desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes},
    element::desktop::{current_window, title_bar},
    platform::Os,
    prelude::*,
};

use crate::{
    components::{card, confirm_dialog, desktop_layout, floating_surface, icon_button, icons},
    pages::{
        files::{files_page, rename_dialog},
        graph::graph_page,
        home::home_page,
        links::backlinks_panel,
        note::note_page,
        palette::command_palette,
        search::search_page,
        settings::{CATEGORIES, desktop_settings, settings_categories, settings_detail},
        status::status_bar,
        switcher::quick_switcher,
        trash::trash_page,
        welcome::welcome_page,
    },
    store::{self, Mode, Notice, Route},
    strings,
};

pub(crate) fn shell() -> impl IntoElement {
    // A tablet turned, or put beside another app, can change the layout.
    let wide = computed(desktop_layout);
    let layout = dynamic(move || {
        let wide = wide.get();
        untracked(|| {
            // Each layout starts with its own backlinks: the desktops' beside
            // the note, the phones' sheet only when asked. The phones' has
            // no Split: a tablet turned from it edits.
            store::set_backlinks_shown(wide);
            if !wide && store::mode() == Mode::Split {
                store::set_mode(Mode::Edit);
            }
            match wide {
                true => desktop().into_element(),
                false => mobile().into_element(),
            }
        })
    });
    view()
        .flex_1()
        .child(layout)
        .child(dynamic(move || store::renaming().map(rename_dialog)))
        .child(dynamic(move || store::switching().then(quick_switcher)))
        .child(dynamic(move || store::palette().then(command_palette)))
        // What the desktops ask before deleting or creating.
        .child(dynamic(move || {
            store::asking()
                .map(|ask| confirm_dialog(ask.title, ask.description, ask.action, store::answer))
        }))
}

fn desktop() -> impl IntoElement {
    let tablet = !is_desktop();
    if tablet {
        // A tablet's back (Android) leaves the graph, the settings, the
        // trash or the search for the note, as their back button does.
        on_back_press(|| {
            let route = untracked(store::route);
            let page = matches!(
                route,
                Route::Graph
                    | Route::Settings
                    | Route::SettingsDetail(_)
                    | Route::Trash
                    | Route::Search
            );
            if page {
                store::back_to_note();
            }
            page
        });
    } else {
        // The commands' own table names their keys (MVP phase 6).
        for command in store::commands() {
            if let Some(keys) = command.keys {
                on_key(keys, move |_| store::run_command(command.id));
            }
        }
        on_key("mod+p", |_| store::set_palette(true));
        on_key("mod+shift+f", |_| store::toggle_search());
        hide_system_title_bar();
    }
    // Keep the settings sidebar mounted when only its selected category changes.
    let workspace = computed(|| match store::route() {
        Route::SettingsDetail(_) => Route::Settings,
        route => route,
    });
    // Three cards on the sidebar's color, as far from each other as from
    // the window's edges: the files, the workspace, the backlinks.
    let body = view()
        .flex_1()
        .flex_row()
        .gap_2()
        .p_2()
        .pt_0()
        .bg(move || theme().sidebar)
        .child(dynamic(move || {
            (!matches!(store::route(), Route::Settings | Route::SettingsDetail(_))
                && !store::sidebar_hidden())
            .then(sidebar)
        }))
        .child(
            card()
                .flex_1()
                .overflow_hidden()
                .child(dynamic(move || {
                    let route = workspace.get();
                    let has_vault = store::vault().is_some();
                    untracked(|| match (route, has_vault) {
                        (Route::Settings, _) => desktop_settings().into_element(),
                        (Route::Trash, true) => trash_page().into_element(),
                        (Route::Graph, true) => graph_page().into_element(),
                        (Route::Search, true) => search_page().into_element(),
                        (_, true) => note_page().into_element(),
                        (_, false) => welcome_page().into_element(),
                    })
                }))
                .child(notices()),
        )
        // The open note's backlinks, on the right.
        .child(dynamic(move || {
            store::backlinks_shown().then(|| card().w(px(260.0)).child(backlinks_panel()))
        }));
    let layout = view()
        .flex_1()
        .child((!tablet).then(app_title_bar))
        .child(body)
        .child(status_bar());
    match tablet {
        // Clear of the system's status bar and gesture bar, on the
        // sidebar's color.
        true => layout
            .bg(move || theme().sidebar)
            .px_safe_area()
            .pt_safe_area_plus(px(8.0))
            .pb_safe_area(),
        false => layout,
    }
}

/// Hides the system's title bar: the app fills the window up to its top
/// edge and draws its own ([`app_title_bar`]); the system's window controls
/// stay.
fn hide_system_title_bar() {
    let mut bar = TitleBarOptions::default();
    bar.style = TitleBarStyle::Hidden;
    let mut attributes = WindowAttributes::default();
    attributes.title_bar = Some(bar);
    current_window().set(attributes);
}

/// The app's title bar: it drags the window and leaves its ends to the
/// system's window controls (macOS's traffic lights on the left, Windows'
/// and Linux's buttons on the right). macOS centers the title, Windows and
/// Linux start it on the left. Nothing while the platform reports no bar
/// (full screen).
fn app_title_bar() -> impl IntoElement {
    let centered = os() == Os::MacOs;
    dynamic(move || {
        let area = title_bar();
        (area.top > px(0.0)).then(|| {
            let bar = view()
                .h(area.top)
                .pl(area.left)
                .pr(area.right)
                .flex_row()
                .items_center()
                .bg(move || theme().sidebar)
                .window_drag(true)
                .child(
                    text(strings::APP_NAME)
                        .px_3()
                        .text_sm()
                        .text_color(move || theme().muted_foreground),
                );
            match centered {
                true => bar.justify_center(),
                false => bar,
            }
        })
    })
}

fn sidebar() -> impl IntoElement {
    view().w(px(260.0)).child(files_page())
}

fn mobile() -> impl IntoElement {
    // The system's back (Android): from the note to the home page, from the
    // home page out of the app; the pages pushed over them pop themselves.
    on_back_press(|| {
        let note = untracked(store::route) == Route::Note;
        if note {
            store::set_route(Route::Home);
        }
        note
    });
    let layout = dynamic(move || {
        let has_vault = store::vault().is_some();
        untracked(|| match has_vault {
            true => stack().into_element(),
            false => view()
                .flex_1()
                .p_safe_area()
                .child(welcome_page())
                .into_element(),
        })
    });
    // The notices over every page, below the status bar.
    view()
        .flex_1()
        .child(layout)
        .child(
            view()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .mt_safe_area()
                .child(notices()),
        )
        .child(menu())
}

/// The phone's navigation stack: the note (the home page without one)
/// under the graph, the settings or the trash, each pushed with the
/// system's header and back.
fn stack() -> impl IntoElement {
    // The note stays open under the pages pushed over it.
    let note = computed(|| store::route() != Route::Home);
    let main = dynamic(move || match note.get() {
        true => untracked(note_page).into_element(),
        false => untracked(home_page).into_element(),
    });
    let shown = |route| move || store::route() == route;
    nav_stack()
        .flex_1()
        .child(
            nav_page()
                .header_visible(false)
                .child(view().flex_1().p_safe_area().child(main)),
        )
        .child(when(shown(Route::Graph), graph_page))
        // Keep the category list under its detail page for the native back gesture.
        .child(when(
            || matches!(store::route(), Route::Settings | Route::SettingsDetail(_)),
            settings_categories,
        ))
        .child(CATEGORIES.map(|category| {
            when(shown(Route::SettingsDetail(category)), move || {
                settings_detail(category)
            })
        }))
        .child(when(shown(Route::Trash), trash_page))
}

/// The phone's menu: the files, slid in from the left over the whole page.
fn menu() -> impl IntoElement {
    sheet()
        .side(Placement::Left)
        .open(move || Some(store::menu_open()))
        .on_open_change(|e: &SheetEvent| {
            if let SheetEvent::OpenChange(open, _) = e {
                store::set_menu_open(*open);
            }
        })
        .popup_style(|s| s.w_full().h_full())
        .child(
            view()
                .flex_1()
                .p_safe_area()
                .bg(move || theme().sidebar)
                .child(view().flex_1().p_2().child(files_page())),
        )
}

/// How long news stays.
const NEWS_TIME: Duration = Duration::from_secs(3);

/// The notices over the top of the page, the newest on top: each drops in
/// fading and rises away fading, news after three seconds, a problem (an
/// error, a conflict) only when closed.
fn notices() -> impl IntoElement {
    view()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .p_2()
        .gap_2()
        .child(each(
            store::notices,
            |notice| notice.id,
            |notice, _| untracked(|| toast(notice.get())),
        ))
}

fn toast(notice: Notice) -> impl IntoElement {
    let motion = motion().entering(MotionPreset::FadeInDown);
    let hide = move || {
        spawn(async move {
            motion.play(MotionPreset::FadeOutUp).await;
            store::dismiss_notice(notice.id);
        });
    };
    // The task ends with the toast: closed first, it does nothing.
    if !notice.stays {
        spawn(async move {
            timer(NEWS_TIME).await;
            hide();
        });
    }
    let toast = view()
        .bind_motion(&motion)
        .flex_row()
        .items_center()
        .gap_2()
        .pl_4()
        .pr_1()
        .py_1()
        .rounded(move || theme().radius)
        .shadow_md();
    floating_surface(toast)
        .child(
            text(notice.message)
                .flex_1()
                .text_sm()
                .text_color(move || theme().popover_foreground)
                .select_text(),
        )
        .child(icon_button(icons::X, strings::CLOSE, hide))
}
