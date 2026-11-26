//! The command palette (`mod+p`): the app's commands in one list, matched
//! by what they say and run on Return (MVP phase 6).

use nsh_base::{CommandItem, command, theme};
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::{
    components::{PALETTE_ITEM, PALETTE_LIST, PALETTE_SEARCH, modal, palette_panel},
    store,
};

pub(crate) fn command_palette() -> impl IntoElement {
    let close = || store::set_palette(false);
    let panel = palette_panel().child(
        command()
            .items(items)
            .placeholder(t("run_command"))
            .search_accessibility_label(t("run_command"))
            .auto_focus(true)
            // Choosing a command closes the palette first: what it opens
            // takes the page.
            .on_select(move |id: &SharedString| {
                close();
                store::run_command(id);
            })
            .on_cancel(close)
            .search_class(PALETTE_SEARCH)
            .item_class(PALETTE_ITEM)
            .list_class(PALETTE_LIST)
            .text_color(move || theme().foreground),
    );
    modal(close, panel)
}

/// The commands as the palette's rows; the ones the desktops have a key
/// for show it.
fn items() -> Vec<CommandItem> {
    store::commands()
        .iter()
        .map(|found| match found.keys {
            Some(keys) => CommandItem::new(found.id, t!(found.title)).shortcut(keys),
            None => CommandItem::new(found.id, t!(found.title)),
        })
        .collect()
}
