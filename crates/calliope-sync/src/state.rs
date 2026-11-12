//! What the last sync left of each file: the hash of its content and its
//! stamp on this device, and its version on the remote. Kept in the vault's
//! `.calliope/sync/state.json`; without it (a first sync, or a lost file)
//! both sides are merged, nothing is deleted.

use std::collections::HashMap;

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

pub(crate) fn load(vault: &Vault) -> State {
    (vault.read(PATH).ok())
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

pub(crate) fn save(vault: &Vault, state: &State) -> std::io::Result<()> {
    let json = serde_json::to_string(state).map_err(std::io::Error::other)?;
    vault.write(PATH, &json)
}
