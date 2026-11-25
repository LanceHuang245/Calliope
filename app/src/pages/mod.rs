//! The pages, one module each. A page reads and changes the store; it never
//! reaches into another page.

pub(crate) mod files;
pub(crate) mod graph;
pub(crate) mod home;
pub(crate) mod keyboard_bar;
pub(crate) mod links;
pub(crate) mod mermaid;
pub(crate) mod note;
pub(crate) mod palette;
pub(crate) mod search;
pub(crate) mod settings;
pub(crate) mod status;
pub(crate) mod switcher;
pub(crate) mod sync;
pub(crate) mod trash;
pub(crate) mod welcome;
