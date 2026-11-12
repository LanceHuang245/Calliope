//! What a vault syncs with, kept in its `.calliope/sync.json`: the vault
//! does not sync it, so each device has its own. The password or secret
//! key is not in it: the app keeps that in the system's secure storage.

use calliope_core::vault::Vault;
use serde::{Deserialize, Serialize};

const PATH: &str = ".calliope/sync.json";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Off,
    WebDav,
    S3,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub provider: Provider,
    /// The WebDAV server's address, or the S3 endpoint.
    pub endpoint: String,
    /// The WebDAV user name, or the S3 access key id.
    pub user: String,
    /// The folder on the WebDAV server, or the prefix in the S3 bucket,
    /// that holds the vault.
    pub root: String,
    pub bucket: String,
    pub region: String,
    /// The S3 bucket goes in the path (`endpoint/bucket`), as most
    /// S3-compatible servers want, rather than in a subdomain (AWS).
    pub path_style: bool,
    /// Syncs on its own: on opening the vault and coming back to the app,
    /// after editing, and on the desktops every `interval` minutes.
    pub auto: bool,
    pub interval: u32,
    /// This device's name, in the names of conflict copies.
    pub device: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: Provider::Off,
            endpoint: String::new(),
            user: String::new(),
            root: String::new(),
            bucket: String::new(),
            region: String::new(),
            path_style: true,
            auto: true,
            interval: 5,
            device: String::new(),
        }
    }
}

impl Config {
    /// The vault's sync settings; the defaults (sync off) without any.
    pub fn load(vault: &Vault) -> Self {
        (vault.read(PATH).ok())
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, vault: &Vault) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        vault.write(PATH, &json)
    }
}
