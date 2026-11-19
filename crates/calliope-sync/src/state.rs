//! What the last sync left of each file: the hash of its content and its
//! stamp on this device, and its version on the remote. Kept in the vault's
//! `.calliope/sync/state.json` with the remote they are of; without them (a
//! first sync, a lost file, or another remote) both sides are merged,
//! nothing is deleted.

use std::{borrow::Cow, collections::HashMap};

use calliope_core::vault::Vault;
use serde::{Deserialize, Serialize};

use crate::{local::Stamp, remote::Version};

const PATH: &str = ".calliope/sync/state.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Record {
    pub hash: String,
    pub local: Stamp,
    pub remote: Version,
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
