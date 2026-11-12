//! Calliope's sync, independent of any UI: a vault mirrored file for file
//! to remote storage, WebDAV or S3 (through OpenDAL), the way the Remotely
//! Save plugin of Obsidian does it. Each run lists both sides and compares
//! each file with the record the last run left of it ([`state`]), to tell
//! which side changed it ([`plan`]); then it uploads, downloads or deletes
//! ([`engine`]). A file changed on both sides keeps the local version and
//! gets the remote one as a conflict copy beside it: nothing is overwritten
//! or deleted unseen.
//!
//! - [`config`]: what a vault syncs with, in its `.calliope/sync.json`;
//! - [`remote`] and [`local`]: the two sides, their files and versions.

pub mod config;
mod engine;
mod local;
mod plan;
mod remote;
mod state;
#[cfg(test)]
mod tests;
mod tls;

pub use config::{Config, Provider};
pub use engine::{Report, sync, test};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{}", describe(.0))]
    Remote(#[from] opendal::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The run would delete this many files, more than it may without the
    /// user's go-ahead (MVP 8.5).
    #[error("this sync would delete {0} files")]
    TooManyDeletions(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A remote error in a few words for the user (its `Debug` has it all): a
/// refused sign-in, else its first cause (`Connection refused`, a DNS
/// error), else its message.
fn describe(e: &opendal::Error) -> String {
    use std::error::Error as _;
    // OpenDAL gives a refused sign-in no kind of its own, only its status.
    let full = e.to_string();
    if full.contains("status: 401") || full.contains("status: 403") {
        return "the server refused the user name or password".into();
    }
    let cause = std::iter::successors(e.source(), |&c| c.source()).last();
    match cause {
        // rustls says "unexpected error" of what it was told (`tls.rs`).
        Some(cause) => (cause.to_string().trim_start_matches("unexpected error: ")).to_owned(),
        None if !e.message().trim().is_empty() => e.message().trim().to_owned(),
        None => e.kind().to_string(),
    }
}

/// Files that are not synced: Calliope's own (`.calliope/`, `.trash/`),
/// anything hidden (`.git/`, `.DS_Store`) and the systems' clutter.
fn excluded(path: &str) -> bool {
    path.split('/')
        .any(|part| part.starts_with('.') || part == "Thumbs.db" || part == "desktop.ini")
}
