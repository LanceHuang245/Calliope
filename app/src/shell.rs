//! The app shell: the layout of each device idiom, with no page logic.
//!
//! - Desktop (Windows, macOS, Linux): a sidebar (the files) beside the
//!   workspace (the note, or the settings), with the app's shortcuts.
//! - Phone (iOS, Android): one screen at a time, the files, a note or the
//!   settings (the system tab bar and navigation stacks come later).

use nsh_base::{SheetEvent, sheet, theme};
use nsh_core::prelude::*;

use crate::{
    components::{action, banner, icon_button, icons},
    pages::{
        files::{files_page, rename_dialog},
        home::home_page,
        note::note_page,
        settings::settings_page,
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
}

fn desktop() -> impl IntoElement {
    on_key("mod+n", |_| store::new_note());
    on_key("mod+e", |_| store::toggle_read());
    on_key("mod+\\", |_| store::toggle_sidebar());
    view()
        .flex_1()
        .flex_row()
        .child(dynamic(move || (!store::sidebar_hidden()).then(sidebar)))
        .child(view().flex_1().child(notice()).child(dynamic(move || {
            let route = store::route();
            let has_vault = store::vault().is_some();
            untracked(|| match (route, has_vault) {
                (Route::Settings, _) => settings_page().into_element(),
                (_, true) => note_page().into_element(),
                (_, false) => no_vault().into_element(),
            })
        })))
}

fn sidebar() -> impl IntoElement {
    view()
        .w(px(260.0))
        .border_r_1()
        .border_color(move || theme().sidebar_border)
        .child(files_page())
}

fn no_vault() -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            text(strings::APP_NAME)
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(text(strings::NO_VAULT).text_color(move || theme().muted_foreground))
        .child(
            view()
                .flex_row()
                .gap_2()
                .child(action(strings::OPEN_FOLDER, || store::pick_vault(false)))
                .child(action(strings::NEW_VAULT, || store::pick_vault(true))),
        )
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
            untracked(|| match route {
                Route::Home => home_page().into_element(),
                Route::Note => note_page().into_element(),
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
        .child(view().flex_1().p_safe_area().child(files_page()))
}

fn notice() -> impl IntoElement {
    dynamic(move || {
        store::notice().map(|message| {
            banner(message).child(icon_button(icons::X, strings::CLOSE, store::dismiss_notice))
        })
    })
}
