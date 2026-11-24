//! The home page, where no note is open: a new note in its middle (the
//! download's progress while an empty vault syncs) and, on a phone, the
//! menu at the top and the floating toolbar at the bottom.

use calliope_sync::Provider;
use nsh_base::{progress, theme};
use nsh_core::{element::NativeEvent, platform::Os, prelude::*};

use crate::{
    components::{
        BAR_HEIGHT, ButtonVariant, button, desktop_layout, fab, floating_toolbar, icon_button,
        icons, muted, toolbar_toggle,
    },
    store::{self, Route, SyncStatus},
    strings,
};

#[cfg(target_os = "ios")]
mod ios_toolbar;
#[cfg(target_os = "ios")]
pub(crate) use ios_toolbar::register as register_ios_toolbar;

const GLASS_TOOLBAR: &str = "calliope.glass-toolbar";
const GLASS_INSET: f32 = 4.0;
const GLASS_BUTTON: f32 = 44.0;
const GLASS_STRIDE: f32 = GLASS_BUTTON + 4.0;

struct ToolbarAction {
    icon: &'static str,
    symbol: &'static str,
    label: &'static str,
    selected: fn() -> bool,
    visible: fn() -> bool,
    press: fn(),
}

const NEW_NOTE_ACTION: ToolbarAction = ToolbarAction {
    icon: icons::PLUS,
    symbol: "plus",
    label: strings::NEW_NOTE,
    selected: || false,
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
                        .child(text(strings::NO_NOTE).text_color(move || theme().muted_foreground))
                        .child(button(
                            ButtonVariant::Primary,
                            strings::NEW_NOTE,
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
            strings::syncing(done, total)
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
            .child(icon_button(icons::REFRESH, strings::SYNC, store::sync_now))
    })
}

/// The phone's menu button: the files slide in from the left.
pub(crate) fn menu_button() -> impl IntoElement {
    icon_button(icons::MENU, strings::MENU, || store::set_menu_open(true))
}

/// The phones' floating toolbar, over the home page and the note (the
/// desktops have these under the files): sync (filled while it runs), the
/// quick switcher, the graph, the note's backlinks (with a note open) and
/// the trash, with the new note button beside them. Gone while
/// the keyboard is up, the text taking the room.
pub(crate) fn phone_toolbar() -> impl IntoElement {
    // Native buttons and the fallback both use the same visibility, state and actions.
    static ACTIONS: [ToolbarAction; 5] = [
        ToolbarAction {
            icon: icons::REFRESH,
            symbol: "arrow.clockwise",
            label: strings::SYNC,
            selected: || matches!(store::sync_status(), SyncStatus::Running(..)),
            visible: sync_enabled,
            press: store::sync_now,
        },
        ToolbarAction {
            icon: icons::SEARCH,
            symbol: "magnifyingglass",
            label: strings::SEARCH,
            selected: store::switching,
            visible: || true,
            press: || store::set_switching(true),
        },
        ToolbarAction {
            icon: icons::NETWORK,
            symbol: "point.3.connected.trianglepath.dotted",
            label: strings::GRAPH,
            selected: || store::route() == Route::Graph,
            visible: || true,
            press: store::toggle_graph,
        },
        ToolbarAction {
            icon: icons::LINK,
            symbol: "link",
            label: strings::BACKLINKS,
            selected: store::backlinks_shown,
            visible: || store::opened().is_some(),
            press: store::toggle_backlinks,
        },
        ToolbarAction {
            icon: icons::TRASH,
            symbol: "trash",
            label: strings::TRASH,
            selected: || store::route() == Route::Trash,
            visible: || true,
            press: store::toggle_trash,
        },
    ];
    when(
        || keyboard_height() <= px(0.0),
        || {
            if cfg!(target_os = "ios") && os() == Os::Ios && os_version().0 >= 26 {
                return floating_toolbar(
                    glass_toolbar(&ACTIONS),
                    glass_toolbar(&[NEW_NOTE_ACTION]),
                    true,
                )
                .into_element();
            }
            let buttons = view()
                .flex_row()
                .gap_1()
                .child(ACTIONS.iter().map(|action| {
                    when(action.visible, move || {
                        toolbar_toggle(action.icon, action.label, action.selected, action.press)
                    })
                }));
            let add = &NEW_NOTE_ACTION;
            floating_toolbar(buttons, fab(add.icon, add.label, add.press), false).into_element()
        },
    )
}

fn glass_toolbar(actions: &'static [ToolbarAction]) -> impl IntoElement {
    native(GLASS_TOOLBAR)
        .w(move || {
            let shown = actions.iter().filter(|action| (action.visible)()).count();
            px(shown as f32 * GLASS_STRIDE - (GLASS_STRIDE - GLASS_BUTTON) + 2.0 * GLASS_INSET)
        })
        .h(px(BAR_HEIGHT))
        .prop("items", move || {
            serde_json::Value::Array(
                actions
                    .iter()
                    .enumerate()
                    .filter(|(_, action)| (action.visible)())
                    .map(|(index, action)| {
                        serde_json::json!({
                            "index": index,
                            "label": action.label,
                            "symbol": action.symbol,
                            "selected": (action.selected)(),
                        })
                    })
                    .collect(),
            )
        })
        .on_native_event(move |event: &NativeEvent| {
            if &*event.name == "press"
                && let Some(index) = event.data.as_u64()
                && let Some(action) = actions.get(index as usize)
                && (action.visible)()
            {
                (action.press)();
            }
        })
}
