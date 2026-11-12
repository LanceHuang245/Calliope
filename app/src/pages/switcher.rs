//! The quick switcher (`mod+o`): a search field over the notes' titles,
//! matched fuzzily. Return opens the highlighted note; a name no note has
//! creates it.

use calliope_core::note;
use nsh_base::{CommandItem, command, dialog, theme};
use nsh_core::prelude::*;

use crate::{
    components::{card, overlay},
    store, strings,
};

/// The id of the item creating a note: no path of the vault starts so.
const NEW: &str = "/";

pub(crate) fn quick_switcher() -> impl IntoElement {
    let query = signal(String::new());
    let close = || store::set_switching(false);
    dialog()
        .animation(ModalAnimation::Fade)
        .on_cancel(close)
        .flex_1()
        .child(view().absolute().inset_0().bg(overlay).on_press(close))
        .child(
            card()
                .mx(auto())
                .mt_20()
                .w_96()
                .max_h_96()
                .p_1()
                .shadow_lg()
                .child(
                    command()
                        .items(move || items(query.get().trim()))
                        // The items are matched and ranked here, fuzzily.
                        .filterable(false)
                        .placeholder(strings::FIND_NOTE)
                        .search_accessibility_label(strings::FIND_NOTE)
                        .auto_focus(true)
                        .on_query_change(move |q: &SharedString| query.set(q.to_string()))
                        .on_select(move |id: &SharedString| {
                            close();
                            match id.strip_prefix(NEW) {
                                Some(name) => store::create_note(name),
                                None => store::select(id),
                            }
                        })
                        .on_cancel(close)
                        .search_class("px-3 py-2 text-sm")
                        .item_class("px-3 py-2 rounded-md text-sm focus:bg-accent")
                        .text_color(move || theme().foreground),
                ),
        )
}

/// The notes whose titles match `query`, best first, then the note a
/// name no note has would create.
fn items(query: &str) -> Vec<CommandItem> {
    let lower = query.to_lowercase();
    let mut found: Vec<_> = store::note_paths()
        .into_iter()
        .filter_map(|path| Some((rank(&note::title(&path).to_lowercase(), &lower)?, path)))
        .collect();
    // A stable sort: titles ranked the same keep the files' order.
    found.sort_by_key(|(rank, _)| *rank);
    let taken = found
        .iter()
        .any(|(_, path)| note::title(path).to_lowercase() == lower);
    let new = (!query.is_empty() && !taken)
        .then(|| CommandItem::new(format!("{NEW}{query}"), strings::create_note(query)));
    found
        .into_iter()
        .map(|(_, path)| {
            let title = note::title(&path).to_owned();
            CommandItem::new(path, title)
        })
        .chain(new)
        .collect()
}

/// How well `title` matches `query` (both lowercase), lower first: `None`
/// unless the query's letters are all in it, in order. A run of them
/// ranks before scattered letters, then the tighter and earlier the
/// letters, then the shorter the title.
fn rank(title: &str, query: &str) -> Option<(bool, usize, usize, usize)> {
    if let Some(at) = title.find(query) {
        return Some((false, 0, at, title.len()));
    }
    let mut letters = title.char_indices();
    let (mut first, mut last) = (None, 0);
    for wanted in query.chars() {
        let (at, _) = letters.find(|&(_, c)| c == wanted)?;
        first.get_or_insert(at);
        last = at;
    }
    let first = first.unwrap_or(0);
    Some((true, last - first, first, title.len()))
}

#[cfg(test)]
mod tests {
    use super::rank;

    #[test]
    fn ranks_runs_then_tight_letters() {
        assert_eq!(rank("roadmap", "rdmp").map(|r| r.0), Some(true));
        assert_eq!(rank("roadmap", "pr"), None);
        let mut titles = ["a road map", "my roadmap", "roadmap"];
        titles.sort_by_key(|t| rank(t, "roadmap"));
        assert_eq!(titles, ["roadmap", "my roadmap", "a road map"]);
    }
}
