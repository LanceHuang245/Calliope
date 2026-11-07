//! Watching a vault for changes made by other apps (desktops only; the
//! phones scan again when the app comes back to the foreground).

use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

/// Watches a vault's folder; [`Watcher::take_changed`] tells whether
/// anything changed since it was last asked. Stops when dropped.
pub struct Watcher {
    _inner: RecommendedWatcher,
    changed: Arc<AtomicBool>,
}

impl Watcher {
    pub fn new(root: &Path) -> notify::Result<Self> {
        let changed = Arc::new(AtomicBool::new(false));
        let flag = changed.clone();
        let mut inner =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // Reads are not changes: the app's own reads would loop.
                if event.is_ok_and(|e| !e.kind.is_access()) {
                    flag.store(true, Ordering::Relaxed);
                }
            })?;
        inner.watch(root, RecursiveMode::Recursive)?;
        Ok(Self {
            _inner: inner,
            changed,
        })
    }

    pub fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }
}
