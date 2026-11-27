//! The full-text search (MVP phase 6): the query, the lines that hold it
//! found off the UI thread, and the line a hit opens its note at.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use calliope_core::search::Hit;

use super::*;

/// How many hits a search keeps: more than a page shows, no more to hold.
const LIMIT: usize = 200;
/// How long typing settles before a search runs.
const PAUSE: Duration = Duration::from_millis(120);

#[derive(Clone, Default)]
struct Query(SharedString);
#[derive(Clone, Default)]
struct Results(Option<(Vec<Hit>, bool)>);
/// The current search shares its cancellation flag with the worker.
#[derive(Clone, Default)]
struct SearchRun(Option<Arc<AtomicBool>>);
/// The line of a note just opened to show, with the note's path: a line
/// of the note that opened after it is not scrolled to.
#[derive(Clone, Default)]
struct Jump(Option<(SharedString, u32)>);

pub(crate) fn query() -> SharedString {
    get::<Query>().0
}

/// The completed hits and whether more matched, or None while searching.
pub(crate) fn search_results() -> Option<(Vec<Hit>, bool)> {
    get::<Results>().0
}

pub(crate) fn set_query(text: &str) {
    if untracked(query).as_str() == text {
        return;
    }
    set(Query(text.to_owned().into()));
    refresh_search();
}

/// Cancels work when the query, index or open page changes.
pub(super) fn refresh_search() {
    cancel_search();
    let query = untracked(query);
    if untracked(route) != Route::Search || query.trim().is_empty() {
        return;
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    set(SearchRun(Some(cancelled.clone())));
    spawn_task(async move {
        timer(PAUSE).await;
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        // Take the latest snapshot after typing settles.
        let index = search_index();
        let stop = cancelled.clone();
        let found = blocking(move || {
            index.search_until(&query, LIMIT + 1, || stop.load(Ordering::Relaxed))
        });
        if let Ok(mut hits) = found.await
            && !cancelled.load(Ordering::Relaxed)
        {
            let truncated = hits.len() > LIMIT;
            hits.truncate(LIMIT);
            set(Results(Some((hits, truncated))));
        }
    });
}

fn cancel_search() {
    global::<SearchRun>().update(|run| {
        if let Some(cancelled) = run.0.take() {
            cancelled.store(true, Ordering::Relaxed);
        }
    });
    set(Results::default());
}

/// Discards search and jump state before another vault is published.
pub(super) fn reset_search() {
    cancel_search();
    set(Query::default());
    set(Jump::default());
}

/// Opens the full-text search, or returns to the note when it is shown.
pub(crate) fn toggle_search() {
    match untracked(route) {
        Route::Search => back_to_note(),
        _ => set_route(Route::Search),
    }
}

/// Opens the note `hit` is in, at its line: the note is shown in the
/// source, which is where a line is scrolled to.
pub(crate) fn open_hit(path: &SharedString, line: u32) {
    if kind_of(path) != Some(EntryKind::Note) {
        return;
    }
    set(Jump(Some((path.clone(), line))));
    if mode() == Mode::Read {
        set_mode(Mode::Edit);
    }
    select(path);
    if untracked(route) == Route::Search {
        set(Jump::default());
    }
}

/// The line the open note shows, until its editor scrolled there.
pub(crate) fn jump_line() -> Option<u32> {
    match get::<Jump>().0 {
        Some((path, line)) if note_path().as_deref() == Some(path.as_str()) => Some(line),
        _ => None,
    }
}

pub(crate) fn clear_jump_line() {
    if jump_line().is_some() {
        set(Jump(None));
    }
}
