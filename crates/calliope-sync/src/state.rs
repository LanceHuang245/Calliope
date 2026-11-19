//! What the last sync left of each file: the hash of its content and its
//! stamp on this device, and its version on the remote. Kept in the vault's
//! `.calliope/sync/state.json` with the remote they are of; without them (a
//! first sync, a lost file, or another remote) both sides are merged,
//! nothing is deleted.

use std::{borrow::Cow, collections::HashMap};

use calliope_core::vault::Vault;
use serde::{Deserialize, Serialize};

use crate::{
    local::{self, Stamp},
    remote::Version,
};

const PATH: &str = ".calliope/sync/state.json";

/// A time no file has: what a record keeps instead of one too recent to
/// vouch for the content ([`local::racy`]), so the next sync compares the
/// content, as Git does with "racily clean" files.
const UNSURE: i64 = -1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub hash: String,
    pub local: Stamp,
    pub remote: Version,
}

impl Record {
    /// The record of a file whose content hashes to `hash`, stamped
    /// `local` here and at version `remote` there.
    pub(crate) fn new(hash: String, local: Stamp, remote: Version) -> Self {
        Self {
            hash,
            local: kept(local),
            remote: kept_version(remote),
        }
    }

    /// Whether the remote version could not vouch for the content when it
    /// was recorded: the remote file has to be compared.
    pub(crate) fn unsure(&self) -> bool {
        self.remote.modified == Some(UNSURE)
    }
}

/// `stamp` as a record keeps it: [`UNSURE`] for a time too recent.
pub(crate) fn kept(stamp: Stamp) -> Stamp {
    match local::racy(stamp.modified) {
        true => Stamp {
            modified: UNSURE,
            ..stamp
        },
        false => stamp,
    }
}

/// `version` as a record keeps it: without an ETag only its time and size
/// tell it apart, so a time too recent becomes [`UNSURE`]. The server's
/// clock is taken as close to this device's.
pub(crate) fn kept_version(version: Version) -> Version {
    match version.etag.is_none() && version.modified.is_some_and(local::racy) {
        true => Version {
            modified: Some(UNSURE),
            ..version
        },
        false => version,
    }
}

pub(crate) type State = HashMap<String, Record>;

/// The file: the records and the remote they are of
/// ([`crate::Config::remote_id`]).
#[derive(Serialize, Deserialize)]
struct Saved<'a> {
    remote: Cow<'a, str>,
    files: Cow<'a, State>,
}

/// The records of the last sync with `remote`; none when the last sync was
/// with another one, whose versions mean nothing here.
pub(crate) fn load(vault: &Vault, remote: &str) -> State {
    (vault.read(PATH).ok())
        .and_then(|json| serde_json::from_str::<Saved>(&json).ok())
        .filter(|saved| saved.remote == remote)
        .map(|saved| saved.files.into_owned())
        .unwrap_or_default()
}

pub(crate) fn save(vault: &Vault, remote: &str, state: &State) -> std::io::Result<()> {
    let saved = Saved {
        remote: Cow::Borrowed(remote),
        files: Cow::Borrowed(state),
    };
    let json = serde_json::to_string(&saved).map_err(std::io::Error::other)?;
    vault.write(PATH, &json)
}
