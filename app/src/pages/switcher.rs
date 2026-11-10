//! The quick switcher (`mod+o`): a search field over the notes' titles.
//! Return opens the highlighted note; a name no note has creates it.

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

/// Every note by its title and, for a `query` no title holds, the note it
/// would create.
fn items(query: &str) -> Vec<CommandItem> {
    let lower = query.to_lowercase();
    let paths = store::note_paths();
    let found = paths
        .iter()
        .any(|path| note::title(path).to_lowercase().contains(&lower));
    let new = (!found).then(|| {
        CommandItem::new(format!("{NEW}{query}"), strings::create_note(query))
            .keywords([query.to_owned()])
    });
    paths
        .into_iter()
        .map(|path| {
            let title = note::title(&path).to_owned();
            CommandItem::new(path, title)
        })
        .chain(new)
        .collect()
}
