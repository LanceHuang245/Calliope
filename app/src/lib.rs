//! Calliope's UI, on Native Shell: the root component, which `nsh run`,
//! `nsh package` and `cargo run` mount.
//!
//! Layers, dependencies pointing downward (nsh's Coding Guides):
//!
//! - [`shell`]: the app shell, the layout of each device idiom, no page logic;
//! - [`pages`]: one module per page, reading and changing [`store`];
//! - [`components`]: Calliope's own styled controls, built on `nsh-base`;
//! - [`store`]: the UI state, over `calliope-core`;
//! - [`strings`]: every text of the UI, [`paths`], [`logging`] and
//!   [`secrets`]: the app's own files.

mod components;
mod logging;
mod pages;
mod paths;
mod secrets;
mod shell;
mod store;
mod strings;

use nsh_base::theme;
use nsh_core::{element::desktop::window as app_window, prelude::*};

/// The app's UI set (`nsh.json` `"rust"`): what a TSX module may import from
/// `calliope/native`.
#[native_module("calliope/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

// The iOS runner installs this factory before it mounts the app.
#[cfg(target_os = "ios")]
#[nsh_core::element::native_components(ios)]
fn ios_components(components: &nsh_platform_ios::Components) {
    pages::home::register_ios_toolbar(components);
}

/// The app's root. On the desktops it shows in the window it declares
/// (nsh makes none for a Rust root); off them `window()` shows its content
/// in the app's one surface.
#[component]
pub fn App() -> impl IntoElement {
    paths::init();
    logging::init();
    store::init();
    let content = || {
        view()
            .flex_1()
            .bg(move || theme().background)
            .text_color(move || theme().foreground)
            .child(shell::shell())
    };
    app_window().id("main").content(content)
}
