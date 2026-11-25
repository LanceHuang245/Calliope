//! The full-text search (MVP phase 6): the query, the lines that hold it
//! found off the UI thread, and the line a hit opens its note at.

use std::time::Duration;

use calliope_core::search::Hit;

use super::*;

/// How many hits a search keeps: more than a page shows, no more to hold.
const LIMIT: usize = 200;
/// How long typing settles before a search runs.
const PAUSE: Duration = Duration::from_millis(120);

#[derive(Clone, Default)]
struct Query(SharedString);
#[derive(Clone, Default)]
struct Hits(Vec<Hit>);
/// Counts the searches asked for: only the last one's hits are kept.
#[derive(Clone, Default)]
struct Searches(u64);
/// The line of a note just opened to show, with the note's path: a line
/// of the note that opened after it is not scrolled to.
#[derive(Clone, Default)]
struct Jump(Option<(String, u32)>);

pub(crate) fn query() -> SharedString {
    get::<Query>().0
}

/// The lines the last search found.
pub(crate) fn hits() -> Vec<Hit> {
    get::<Hits>().0
}

/// Searches the notes for `text`, keeping only this search's hits.
pub(crate) fn set_query(text: &str) {
    set(Query(text.to_owned().into()));
    global::<Searches>().update(|searches| searches.0 += 1);
    let run = untracked(|| get::<Searches>().0);
    // The index is cloned on the UI thread; the search runs on the other.
    let index = search_index();
    let query = text.trim().to_owned();
    set(Hits::default());
    spawn_task(async move {
        timer(PAUSE).await;
        if untracked(|| get::<Searches>().0) != run {
            return;
        }
        let found = blocking(move || index.search(&query, LIMIT));
        if let Ok(found) = found.await
            && untracked(|| get::<Searches>().0) == run
        {
            set(Hits(found));
        }
    });
}

/// Forgets the hits: another vault opens.
pub(super) fn reset_search() {
    set(Query::default());
    set(Hits::default());
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
pub(crate) fn open_hit(hit: &Hit) {
    set(Jump(Some((hit.path.clone(), hit.line))));
    if mode() == Mode::Read {
        set_mode(Mode::Edit);
    }
    select(&SharedString::from(hit.path.clone()));
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
