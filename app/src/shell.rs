//! The app shell: the layout of each device idiom, with no page logic.
//!
//! - Desktop (Windows, macOS, Linux): the app's own title bar over a
//!   sidebar (the files and the app's actions) beside the workspace (the
//!   note, or the settings) and the open note's backlinks, with the app's
//!   shortcuts.
//! - Phone (iOS, Android): one screen at a time, the files, a note or the
//!   settings (the system tab bar and navigation stacks come later).

use nsh_base::{SheetEvent, sheet, theme};
use nsh_core::{
    desktop::{TitleBarOptions, TitleBarStyle, WindowAttributes},
    element::desktop::{current_window, title_bar},
    platform::Os,
    prelude::*,
};

use crate::{
    components::{banner, card, icon_button, icons},
    pages::{
        files::{files_page, rename_dialog},
        graph::graph_page,
        home::home_page,
        links::backlinks_panel,
        note::note_page,
        settings::settings_page,
        switcher::quick_switcher,
        welcome::welcome_page,
    },
    store::{self, Route},
    strings,
};

pub(crate) fn shell() -> impl IntoElement {
    let layout = match is_desktop() {
        true => desktop().into_element(),
        false => mobile().into_element(),
    };
    view()
        .flex_1()
        .child(layout)
        .child(dynamic(move || store::renaming().map(rename_dialog)))
        .child(dynamic(move || store::switching().then(quick_switcher)))
}

fn desktop() -> impl IntoElement {
    on_key("mod+n", |_| store::new_note());
    on_key("mod+e", |_| store::toggle_read());
    on_key("mod+o", |_| store::set_switching(true));
    on_key("mod+g", |_| store::set_route(Route::Graph));
    on_key("mod+\\", |_| store::toggle_sidebar());
    hide_system_title_bar();
    // Three cards on the sidebar's color, as far from each other as from
    // the window's edges: the files, the workspace, the backlinks.
    let body = view()
        .flex_1()
        .flex_row()
        .gap_2()
        .p_2()
        .pt_0()
        .bg(move || theme().sidebar)
        .child(dynamic(move || (!store::sidebar_hidden()).then(sidebar)))
        .child(
            card()
                .flex_1()
                .overflow_hidden()
                .child(notice())
                .child(dynamic(move || {
                    let route = store::route();
                    let has_vault = store::vault().is_some();
                    untracked(|| match (route, has_vault) {
                        (Route::Settings, _) => settings_page().into_element(),
                        (Route::Graph, true) => graph_page().into_element(),
                        (_, true) => note_page().into_element(),
                        (_, false) => welcome_page().into_element(),
                    })
                })),
        )
        // The open note's backlinks, on the right.
        .child(dynamic(move || {
            let shown = !store::backlinks_hidden()
                && !matches!(store::route(), Route::Settings | Route::Graph)
                && store::opened().is_some();
            shown.then(|| card().w(px(260.0)).child(backlinks_panel()))
        }));
    view().flex_1().child(app_title_bar()).child(body)
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
    // The system's back (Android) goes back to the home page.
    on_back_press(|| match untracked(store::route) {
        Route::Home => false,
        _ => {
            store::set_route(Route::Home);
            true
        }
    });
    view()
        .flex_1()
        .p_safe_area()
        .child(notice())
        .child(dynamic(move || {
            let route = store::route();
            let has_vault = store::vault().is_some();
            untracked(|| match route {
                _ if !has_vault => welcome_page().into_element(),
                Route::Home => home_page().into_element(),
                Route::Note => note_page().into_element(),
                Route::Graph => graph_page().into_element(),
                Route::Settings => settings_page().into_element(),
            })
        }))
        .child(menu())
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

fn notice() -> impl IntoElement {
    dynamic(move || {
        store::notice().map(|message| {
            banner(message).child(icon_button(icons::X, strings::CLOSE, store::dismiss_notice))
        })
    })
}
