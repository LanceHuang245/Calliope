//! Where the graph's nodes were dragged to: kept on this device, apart from
//! the vault (a drag never writes a note), and for one vault at a time.

use super::*;

/// The vault's dragged nodes by id, as `(x, y)` from the layout's origin.
pub(crate) fn node_positions() -> HashMap<String, (f32, f32)> {
    match key() {
        Some(key) => {
            serde_json::from_str(&Storage::get_item(&key).unwrap_or_default()).unwrap_or_default()
        }
        None => HashMap::new(),
    }
}

/// Keeps `at` as the vault's dragged nodes: the graph opens with them.
pub(crate) fn save_node_positions(at: HashMap<String, (f32, f32)>) {
    let Some(key) = key() else {
        return;
    };
    let saved = match serde_json::to_string(&at) {
        Ok(json) => Storage::set_item(&key, json).map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    if let Err(e) = saved {
        log::warn!("cannot keep the graph's positions: {e}");
    }
}

/// The storage key of the open vault's positions: each vault keeps its own
/// nodes.
fn key() -> Option<String> {
    let path = vault_path();
    (!path.is_empty()).then(|| format!("graph/{path}"))
}
