//! Calliope's UI, on Native Shell: the root component, which `nsh run`,
//! `nsh package` and `cargo run` mount.
//!
//! Layers, dependencies pointing downward (nsh's Coding Guides):
//!
//! - [`shell`]: the app shell, the layout of each device idiom, no page logic;
//! - [`pages`]: one module per page, reading and changing [`store`];
//! - [`components`]: Calliope's own styled controls, built on `nsh-base`;
//! - [`store`]: the UI state, over `calliope-core` and `calliope-sync`;
//! - [`strings`]: every text of the UI, [`paths`] and [`logging`]: the
//!   app's own files.

mod components;
mod logging;
mod pages;
mod paths;
mod shell;
mod store;
mod strings;

use nsh_base::theme;
use nsh_core::prelude::*;

/// The app's UI set (`nsh.json` `"rust"`): what a TSX module may import from
/// `calliope/native`.
#[native_module("calliope/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// The app's root: the whole surface, on the theme's background.
#[component]
pub fn App() -> impl IntoElement {
    logging::init();
    store::init();
    view()
        .flex_1()
        .bg(move || theme().background)
        .text_color(move || theme().foreground)
        .child(shell::shell())
}
