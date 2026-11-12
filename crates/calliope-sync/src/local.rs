//! The local side: the vault's files with their modification times and
//! sizes, and the hash that tells whether a file's content changed.

use std::{collections::HashMap, fs, io, path::Path, time::UNIX_EPOCH};

use calliope_core::vault::Vault;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::excluded;

/// A local file's modification time (in nanoseconds) and size: while both
/// match its record, its content is taken as unchanged without reading it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stamp {
    pub modified: i64,
    pub size: u64,
}

impl Stamp {
    fn of(meta: &fs::Metadata) -> Self {
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok());
        Self {
            modified: modified.map_or(0, |d| d.as_nanos() as i64),
            size: meta.len(),
        }
    }
}

/// The stamp of the file at `path`, `None` when it is not there.
pub(crate) fn stamp(vault: &Vault, path: &str) -> Option<Stamp> {
    fs::metadata(vault.abs(path)).ok().map(|m| Stamp::of(&m))
}

/// Every file of the vault that syncs, by path.
pub(crate) fn list(vault: &Vault) -> io::Result<HashMap<String, Stamp>> {
    fn walk(dir: &Path, rel: &str, files: &mut HashMap<String, Stamp>) -> io::Result<()> {
        for item in fs::read_dir(dir)? {
            let item = item?;
            let name = item.file_name().to_string_lossy().into_owned();
            let path = match rel {
                "" => name,
                _ => format!("{rel}/{name}"),
            };
            if excluded(&path) {
                continue;
            }
            let meta = item.metadata()?;
            if meta.is_dir() {
                walk(&item.path(), &path, files)?;
            } else {
                files.insert(path, Stamp::of(&meta));
            }
        }
        Ok(())
    }
    let mut files = HashMap::new();
    walk(vault.root(), "", &mut files)?;
    Ok(files)
}

pub(crate) fn hash(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
