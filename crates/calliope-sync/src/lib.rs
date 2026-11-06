//! Calliope's sync, independent of any UI: a vault mirrored to remote
//! storage. A [`backend`] is where the files go (WebDAV and S3 first;
//! OneDrive, Dropbox and Git later), the [`engine`] compares the vault, the
//! remote and the last synced [`state`] to decide what to upload, download
//! or delete, and [`conflict`] settles files changed on both sides.

pub mod backend;
pub mod conflict;
pub mod engine;
pub mod state;

#[cfg(feature = "s3")]
pub mod s3;
#[cfg(feature = "webdav")]
pub mod webdav;
