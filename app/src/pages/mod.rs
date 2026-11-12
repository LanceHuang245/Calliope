//! The pages, one module each. A page reads and changes the store; it never
//! reaches into another page.

pub(crate) mod files;
pub(crate) mod graph;
pub(crate) mod home;
pub(crate) mod links;
pub(crate) mod mermaid;
pub(crate) mod note;
pub(crate) mod settings;
pub(crate) mod switcher;
pub(crate) mod sync;
pub(crate) mod welcome;
