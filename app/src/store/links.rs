//! The vault's links: the index, kept up to date in the background, the
//! open note's backlinks, following a link and the quick switcher.

use std::sync::Arc;

use super::*;
use calliope_core::{
    graph::Graph,
    index::{Backlink, Index},
};

#[derive(Clone, Default)]
struct Links(Arc<Index>);
/// Counts the indexing runs: only the last one's result is kept.
#[derive(Clone, Default)]
struct Reindexing(u64);
#[derive(Clone)]
struct BacklinksHidden(bool);

/// The desktops show the backlinks at first; the phones only when asked,
/// as they take the note's room.
impl Default for BacklinksHidden {
    fn default() -> Self {
        Self(!is_desktop())
    }
}
/// The quick switcher is open.
#[derive(Clone, Default)]
struct Switching(bool);

/// Forgets the index: another vault opens.
pub(super) fn reset_links() {
    set(Links::default());
}

/// Brings the index up to date with the vault's files, off the UI thread.
pub(super) fn reindex() {
    let Some(vault) = untracked(vault) else {
        return;
    };
    global::<Reindexing>().update(|r| r.0 += 1);
    let run = untracked(|| get::<Reindexing>().0);
    // Only bump the Arc refcount on the UI thread; clone the index on the worker thread.
    let prev = untracked(|| Arc::clone(&get::<Links>().0));
    spawn_task(async move {
        let done = blocking(move || {
            let mut index = Index::clone(&prev);
            index.sync(&vault).map(|changed| (changed, index))
        });
        match done.await {
            // A run started since replaces this one.
            _ if untracked(|| get::<Reindexing>().0) != run => {}
            Ok(Ok((true, index))) => set(Links(Arc::new(index))),
            Ok(Ok(_)) => {}
            Ok(Err(e)) => log::warn!("cannot index the vault: {e}"),
            Err(e) => log::warn!("cannot index the vault: {e}"),
        }
    });
}

/// The links to the open note from the other notes.
pub(crate) fn backlinks() -> Vec<Backlink> {
    let index = get::<Links>().0;
    note_path()
        .map(|path| index.backlinks(&path))
        .unwrap_or_default()
}

/// The vault's graph; `orphans` and `missing` as [`Graph::of`] takes them.
pub(crate) fn graph(orphans: bool, missing: bool) -> Graph {
    Graph::of(&get::<Links>().0, orphans, missing)
}

/// Whether the backlinks panel shows: on, beside an open note (not the
/// settings, the trash or the graph).
pub(crate) fn backlinks_shown() -> bool {
    !get::<BacklinksHidden>().0
        && !matches!(
            route(),
            Route::Settings | Route::SettingsDetail(_) | Route::Trash | Route::Graph
        )
        && opened().is_some()
}

pub(crate) fn toggle_backlinks() {
    global::<BacklinksHidden>().update(|h| h.0 = !h.0);
}

/// Shows the backlinks or not (the phones' sheet, closed by a swipe).
pub(crate) fn set_backlinks_shown(shown: bool) {
    set(BacklinksHidden(!shown));
}

pub(crate) fn switching() -> bool {
    get::<Switching>().0
}

pub(crate) fn set_switching(open: bool) {
    set(MenuOpen(false));
    set(Switching(open));
}

/// Whether a link's `target` names a note of the vault.
pub(crate) fn link_exists(target: &str) -> bool {
    global::<Links>().with(|l| l.0.resolve(target).is_some())
}

/// Follows a link: opens the note `target` names, or asks to create it.
pub(crate) fn open_link(target: String) {
    let found = untracked(|| get::<Links>().0.resolve(&target).map(str::to_owned));
    if let Some(path) = found {
        return select(&path.into());
    }
    let ask = Ask {
        title: strings::create_note(&target),
        description: None,
        action: strings::CREATE,
        destructive: false,
    };
    confirm(ask, move || create_note(&target));
}

/// Creates the note a link to `target` would name, and opens it.
pub(crate) fn create_note(target: &str) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    // A new note stays inside the vault, out of its hidden folders.
    if target.split('/').any(|part| part.starts_with('.')) {
        return;
    }
    let path = format!("{target}.md");
    let created = fs::create_dir_all(vault.abs(&path).with_file_name(""))
        .and_then(|()| vault.write(&path, ""));
    match created {
        Ok(()) => {
            refresh();
            select(&path.into());
        }
        Err(e) => notify(format!("{path}: {e}")),
    }
}

/// Every note's path, read untracked.
pub(crate) fn note_paths() -> Vec<String> {
    untracked(|| get::<Links>().0.notes().map(str::to_owned).collect())
}
