//! Calliope's knowledge base, independent of any UI: a vault is a folder of
//! Markdown notes and their attachments, the notes link to each other, and
//! the links make the backlinks and the graph.
//!
//! Everything here is plain Rust on the file system, so it runs and is tested
//! the same on every platform, and the app and the sync engine share it.

pub mod graph;
pub mod index;
pub mod link;
pub mod note;
pub mod vault;
