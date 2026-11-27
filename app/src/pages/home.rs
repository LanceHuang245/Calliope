//! The home page, where no note is open: a new note in its middle (the
//! download's progress while an empty vault syncs) and, on a phone, the
//! menu at the top and the floating toolbar at the bottom.

use calliope_sync::Provider;
use nsh_base::{MenuButtonItem, progress, theme};
use nsh_core::{element::NativeEvent, platform::Os, prelude::*};

use crate::i18n::t;

use crate::{
    components::{
        BAR_HEIGHT, ButtonVariant, TOOLBAR_BUTTON, button, desktop_layout, fab, floating_toolbar,
        icon_button, icons, muted, toolbar_toggle,
    },
    i18n,
    store::{self, Panel, Route, SyncStatus},
};

#[cfg(target_os = "ios")]
mod ios_toolbar;
#[cfg(target_os = "ios")]
pub(crate) use ios_toolbar::register as register_ios_toolbar;

const GLASS_TOOLBAR: &str = "calliope.glass-toolbar";
const GLASS_MENU: &str = "calliope.glass-menu";
const GLASS_INSET: f32 = 4.0;
const GLASS_BUTTON: f32 = 44.0;
const GLASS_STRIDE: f32 = GLASS_BUTTON + 4.0;

/// The more button's symbol in the capsule, where UIKit draws the icons.
const MORE_SYMBOL: &str = "ellipsis";

struct ToolbarAction {
    icon: &'static str,
    symbol: &'static str,
    /// Its key among the texts (`t!`), for screen readers.
    label: &'static str,
    selected: Option<fn() -> bool>,
    visible: fn() -> bool,
    press: fn(),
}

const NEW_NOTE_ACTION: ToolbarAction = ToolbarAction {
    icon: icons::PLUS,
    symbol: "plus",
    label: "new_note",
    selected: None,
    visible: || true,
    press: store::new_note,
};

pub(crate) fn home_page() -> impl IntoElement {
    view()
        .flex_1()
        .child((!desktop_layout()).then(|| view().flex_row().p_2().child(menu_button())))
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_4()
                .child(when(downloading, download_progress).otherwise(|| {
                    view()
                        .items_center()
                        .gap_4()
                        .child(text(t("no_note")).text_color(move || theme().muted_foreground))
                        .child(button(
                            ButtonVariant::Primary,
                            t("new_note"),
                            store::new_note,
                        ))
                })),
        )
        .child((!desktop_layout()).then(phone_toolbar))
}

/// Whether the vault is empty while it syncs: its first download (a
/// phone's vault made from sync), which shows its progress.
fn downloading() -> bool {
    !store::has_files() && matches!(store::sync_status(), SyncStatus::Running(..))
}

/// The files synced of how many, and a bar of them; moving with no end
/// while the run finds out how many (it lists the server first).
fn download_progress() -> impl IntoElement {
    let counts = || match store::sync_status() {
        SyncStatus::Running(done, total) => (done, total),
        _ => (0, 0),
    };
    let share = move || {
        let (done, total) = counts();
        match total {
            0 => 0.0,
            _ => done as f32 * 100.0 / total as f32,
        }
    };
    view()
        .w(px(240.0))
        .gap_2()
        .child(muted(move || {
            let (done, total) = counts();
            i18n::syncing(done, total)
        }))
        .child(
            progress()
                .value(share)
                .indeterminate(move || counts().1 == 0)
                .track_style(|s| s.h(px(6.0)).rounded_full().overflow_hidden())
                .indicator_style(|s| s.h_full().rounded_full())
                .track_class("bg-muted")
                .indicator_class("bg-primary"),
        )
}

fn sync_enabled() -> bool {
    store::sync_config().provider != Provider::Off
}

/// Shows the desktop shortcut while sync is enabled, dimmed during a run.
pub(crate) fn sync_button() -> impl IntoElement {
    when(sync_enabled, || {
        let running = || matches!(store::sync_status(), SyncStatus::Running(..));
        view()
            .opacity(move || if running() { 0.5_f32 } else { 1.0 })
            .child(icon_button(icons::REFRESH, t("sync"), store::sync_now))
    })
}

/// The phone's menu button opens the same file drawer on both platforms.
pub(crate) fn menu_button() -> impl IntoElement {
    const MENU: ToolbarAction = ToolbarAction {
        icon: icons::MENU,
        symbol: "line.3.horizontal",
        label: "menu",
        selected: None,
        visible: || true,
        press: || store::set_menu_open(true),
    };
    if cfg!(target_os = "ios") && os() == Os::Ios {
        glass_toolbar(&[MENU], &[], true).into_element()
    } else {
        icon_button(MENU.icon, t!(MENU.label), MENU.press).into_element()
    }
}

/// The phones' floating toolbar, over the home page and the note (the
/// desktops have these under the files): sync (filled while it runs), the
/// quick switcher, the graph, the note's backlinks, outline and outlinks
/// (with a note open), the commands (`mod+p` on the desktops) and the
/// trash, with the new note button beside them. The ones a narrow phone has
/// no room for go into the filter button at the bar's end: UIKit's own menu
/// on the capsule, [`filter_menu`] on the buttons the app draws. Gone while
/// the keyboard is up, the text taking the room.
pub(crate) fn phone_toolbar() -> impl IntoElement {
    // Native buttons and the fallback both use the same visibility, state and
    // actions; the ones the bar's width leaves no room for go into its menu,
    // so a new action is a new entry here and nothing else.
    static ACTIONS: &[ToolbarAction] = &[
        ToolbarAction {
            icon: icons::REFRESH,
            symbol: "arrow.clockwise",
            label: "sync",
            selected: Some(|| matches!(store::sync_status(), SyncStatus::Running(..))),
            visible: sync_enabled,
            press: store::sync_now,
        },
        ToolbarAction {
            icon: icons::SEARCH,
            symbol: "magnifyingglass",
            label: "search",
            selected: Some(store::switching),
            visible: || true,
            press: || store::set_switching(true),
        },
        ToolbarAction {
            icon: icons::NETWORK,
            symbol: "point.3.connected.trianglepath.dotted",
            label: "graph",
            selected: Some(|| store::route() == Route::Graph),
            visible: || true,
            press: store::toggle_graph,
        },
        ToolbarAction {
            icon: icons::LINK,
            symbol: "link",
            label: "backlinks",
            selected: Some(|| store::panel_shown(Panel::Backlinks)),
            visible: || store::opened().is_some(),
            press: || store::toggle_panel(Panel::Backlinks),
        },
        ToolbarAction {
            icon: icons::COMMAND,
            symbol: "command",
            label: "run_command",
            selected: None,
            visible: || true,
            press: || store::set_palette(true),
        },
        ToolbarAction {
            icon: icons::TRASH,
            symbol: "trash",
            label: "trash",
            selected: Some(|| store::route() == Route::Trash),
            visible: || true,
            press: store::toggle_trash,
        },
        // The note's panels, after the buttons that were on the bar before
        // them: a narrow phone keeps those and puts these in its menu.
        ToolbarAction {
            icon: icons::LIST,
            symbol: "list.bullet",
            label: "outline",
            selected: Some(|| store::panel_shown(Panel::Outline)),
            visible: || store::opened().is_some(),
            press: || store::toggle_panel(Panel::Outline),
        },
        ToolbarAction {
            icon: icons::EXTERNAL_LINK,
            symbol: "arrow.up.forward.square",
            label: "outlinks",
            selected: Some(|| store::panel_shown(Panel::Outlinks)),
            visible: || store::opened().is_some(),
            press: || store::toggle_panel(Panel::Outlinks),
        },
    ];
    when(
        || keyboard_height() <= px(0.0),
        || {
            // Which buttons the bar keeps follows its width: the ones it has
            // no room for go into its filter menu, so another action shortens
            // the row instead of widening it past the screen.
            dynamic(|| {
                let cut = filter_from(ACTIONS, window().width.0).unwrap_or(ACTIONS.len());
                let (kept, filtered) = ACTIONS.split_at(cut);
                if native_bar() {
                    // What the bar has no room for goes inside the capsule:
                    // UIKit's own menu button at its end shows it.
                    return floating_toolbar(
                        glass_toolbar(kept, filtered, false),
                        glass_toolbar(&[NEW_NOTE_ACTION], &[], false),
                        true,
                    )
                    .into_element();
                }
                let buttons = view()
                    .flex_row()
                    .gap_1()
                    .child(kept.iter().map(|action| {
                        when(action.visible, move || {
                            toolbar_toggle(
                                action.icon,
                                t!(action.label),
                                move || action.selected.is_some_and(|selected| selected()),
                                action.press,
                            )
                        })
                    }))
                    .child(filter_menu(filtered));
                let add = &NEW_NOTE_ACTION;
                floating_toolbar(buttons, fab(add.icon, t!(add.label), add.press), false)
                    .into_element()
            })
        },
    )
}

/// The filter menu's test id.
const FILTER_MENU: &str = "toolbar-filters";

/// One button and the 4 points after it.
const BUTTON_SLOT: f32 = TOOLBAR_BUTTON + 4.0;

/// What the bar's buttons share their row with: 4 points of padding a side,
/// the gap and the [`fab`] (as tall as the bar).
const BAR_ROOM: f32 = 8.0 + 12.0 + BAR_HEIGHT;

/// Whether the bar is UIKit's glass capsule (`ios_toolbar`) rather than the
/// buttons the app draws.
fn native_bar() -> bool {
    cfg!(target_os = "ios") && os() == Os::Ios && os_version().0 >= 26
}

/// How many of the bar's buttons a `width`-point window holds:
/// [`TOOLBAR_BUTTON`] each, 4 apart, beside the fab and — with `filtered` —
/// the filter button, which the bar ends with.
fn buttons_that_fit(width: f32, filtered: bool) -> usize {
    let filter = match (filtered, native_bar()) {
        (false, _) => 0.0,
        (true, true) => TOOLBAR_BUTTON + 12.0,
        (true, false) => BUTTON_SLOT,
    };
    let room = width - BAR_ROOM - filter;
    (((room + 4.0) / BUTTON_SLOT).floor() as usize).max(1)
}

/// Where the buttons the bar keeps stop in `actions`: the index of the first
/// its filter menu holds, `None` while every visible one fits.
fn filter_from(actions: &'static [ToolbarAction], width: f32) -> Option<usize> {
    let shown = actions.iter().filter(|action| (action.visible)()).count();
    if shown <= buttons_that_fit(width, false) {
        return None;
    }
    actions
        .iter()
        .enumerate()
        .filter(|(_, action)| (action.visible)())
        .nth(buttons_that_fit(width, true))
        .map(|(index, _)| index)
}

/// The buttons the bar has no room for, in the app's own menu, each with the
/// state it is in; nothing while they all fit. The UIKit capsule shows them
/// in a menu of its own at the bar's end instead ([`glass_toolbar`]).
fn filter_menu(actions: &'static [ToolbarAction]) -> Option<impl IntoElement> {
    any_visible(actions).then(|| {
        nsh_base::menu_button()
            .test_id(FILTER_MENU)
            .accessibility_label(t("more_actions"))
            .items(move || {
                actions
                    .iter()
                    .filter(|action| (action.visible)())
                    .map(|action| {
                        let item =
                            MenuButtonItem::new(action.label, t!(action.label)).icon(action.symbol);
                        match action.selected {
                            Some(selected) => item.checked(selected()),
                            None => item,
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .on_select(move |id: &SharedString| {
                if let Some(action) = actions.iter().find(|action| action.label == id.as_ref()) {
                    (action.press)();
                }
            })
            .w(px(TOOLBAR_BUTTON))
            .h(px(TOOLBAR_BUTTON))
            .items_center()
            .justify_center()
            .rounded_full()
            .child(
                icons::icon(icons::MORE)
                    .w(px(20.0))
                    .h(px(20.0))
                    .pointer_events(PointerEvents::None)
                    .text_color(move || theme().muted_foreground),
            )
    })
}

/// Whether any of `actions` shows: the filter button shows for one, and a
/// capsule counts a slot for it.
fn any_visible(actions: &'static [ToolbarAction]) -> bool {
    actions.iter().any(|action| (action.visible)())
}

fn glass_toolbar(
    actions: &'static [ToolbarAction],
    filtered: &'static [ToolbarAction],
    compact: bool,
) -> impl IntoElement {
    // The standalone menu drops the capsule's padding and keeps its 44-point touch target.
    let inset = if compact { 0.0 } else { GLASS_INSET };
    native(if compact { GLASS_MENU } else { GLASS_TOOLBAR })
        .w(move || {
            let shown = actions.iter().filter(|action| (action.visible)()).count()
                + usize::from(any_visible(filtered));
            px(shown as f32 * GLASS_STRIDE - (GLASS_STRIDE - GLASS_BUTTON) + 2.0 * inset)
        })
        .h(px(if compact { GLASS_BUTTON } else { BAR_HEIGHT }))
        .prop("items", move || {
            serde_json::Value::Array(
                actions
                    .iter()
                    .enumerate()
                    .filter(|(_, action)| (action.visible)())
                    .map(|(index, action)| {
                        serde_json::json!({
                            "index": index,
                            "label": t!(action.label),
                            "symbol": action.symbol,
                            "selected": action.selected.map(|selected| selected()),
                        })
                    })
                    .collect(),
            )
        })
        // The filter is the capsule's last button, with the actions the bar
        // has no room for in its menu.
        .prop("filter", move || {
            serde_json::json!({
                "label": t("more_actions"),
                "symbol": MORE_SYMBOL,
                "rows": filtered
                    .iter()
                    .filter(|action| (action.visible)())
                    .map(|action| serde_json::json!({
                        "id": action.label,
                        "label": t!(action.label),
                        "symbol": action.symbol,
                        "checked": action.selected.map(|selected| selected()),
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .on_native_event(move |event: &NativeEvent| match &*event.name {
            "press" => {
                if let Some(index) = event.data.as_u64()
                    && let Some(action) = actions.get(index as usize)
                    && (action.visible)()
                {
                    (action.press)();
                }
            }
            "choose" => {
                if let Some(id) = event.data.as_str()
                    && let Some(action) = filtered.iter().find(|action| action.label == id)
                {
                    (action.press)();
                }
            }
            _ => {}
        })
}
