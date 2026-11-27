//! The vault's files: a tree of folders and notes. The header creates
//! notes and folders, the row under the tree holds the app's actions; each
//! row's context menu (a right click, or a held finger on a phone) acts on
//! its item. The desktops show it in the sidebar, the phones as a screen.

use calliope_core::vault::EntryKind;
use nsh_base::{ContextMenuEvents, TreeEntry, context_menu, menu_separator, theme, tree};
use nsh_core::prelude::*;

use super::home::sync_button;
use crate::i18n::t;

use crate::{
    components::{
        ButtonVariant, button, card, desktop_layout, dialog_card, field, icon_button, icon_toggle,
        icons::{self, icon},
        menu_entry, menu_popup, modal, overlay,
    },
    store::{self, Route},
};

pub(crate) fn files_page() -> impl IntoElement {
    view()
        .flex_1()
        .gap_2()
        .child(
            card()
                .flex_row()
                .items_center()
                .p_1()
                // On a phone the files are the menu, over the page.
                .child(
                    (!desktop_layout())
                        .then(|| icon_button(icons::X, t("close"), || store::set_menu_open(false))),
                )
                .child(
                    text(store::vault_name)
                        .flex_1()
                        .px_2()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(icon_button(
                    icons::FILE_PLUS,
                    t("new_note"),
                    store::new_note,
                ))
                .child(icon_button(
                    icons::FOLDER_PLUS,
                    t("new_folder"),
                    store::new_folder,
                )),
        )
        .child(card().flex_1().p_1().child(dynamic(move || {
            match store::has_files() {
                true => files_tree().into_element(),
                false => text(t("no_files"))
                    .p_2()
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
                    .into_element(),
            }
        })))
        .child(
            // The app's actions, in one row under the files (a phone has
            // sync, search and the graph in its top bar).
            view()
                .flex_row()
                .gap_1()
                .pt_1()
                .child(desktop_layout().then(|| {
                    view()
                        .flex_row()
                        .gap_1()
                        .child(sync_button())
                        .child(icon_button(icons::SEARCH, t("search"), || {
                            store::set_switching(true)
                        }))
                        .child(icon_toggle(
                            icons::NETWORK,
                            t("graph"),
                            || store::route() == Route::Graph,
                            store::toggle_graph,
                        ))
                        // The commands, where the phones' toolbar has them.
                        .child(icon_button(icons::COMMAND, t("command_palette"), || {
                            store::set_palette(true)
                        }))
                        .child(icon_toggle(
                            icons::TRASH,
                            t("trash"),
                            || store::route() == Route::Trash,
                            store::toggle_trash,
                        ))
                }))
                .child(view().flex_1())
                .child(icon_button(icons::SETTINGS, t("settings"), || {
                    store::set_route(Route::Settings)
                })),
        )
}

/// The item highlighted in the file tree: a note only while the note page is
/// active, so clicking it again from Graph, Settings, or Home re-opens it.
fn tree_selected() -> Option<SharedString> {
    store::selected()
        .filter(|id| store::route() == Route::Note || store::kind_of(id) != Some(EntryKind::Note))
}

fn files_tree() -> impl IntoElement {
    tree()
        .items(store::tree_items)
        .expanded(store::expanded)
        .selected(tree_selected)
        // A held finger that opens a row's menu still reports its press to
        // the tree: a context menu cancels the presses of its row alone, and
        // this press is the row's parent's (see `item_menu`). It is no tap,
        // so it neither expands nor opens its item.
        .on_expanded_change(|expanded: &Vec<SharedString>| {
            if !store::row_menu_open() {
                store::set_expanded(expanded);
            }
        })
        .on_selected_change(|id: &SharedString| {
            if !store::row_menu_open() {
                store::select(id);
            }
        })
        .accessibility_label(t("files"))
        .flex_1()
        .render_item(|entry: &TreeEntry| {
            let id = entry.id.clone();
            let kind = store::kind_of(&id);
            let paths = match kind {
                Some(EntryKind::Folder) => icons::FOLDER,
                Some(EntryKind::Note) => icons::FILE_TEXT,
                Some(EntryKind::Image) => icons::IMAGE,
                _ => icons::FILE,
            };
            let selected = {
                let id = id.clone();
                move || tree_selected().as_ref() == Some(&id)
            };
            let row = view()
                .flex_row()
                .items_center()
                .gap_2()
                .h_8()
                .pl(px(8.0 + 16.0 * entry.depth as f32))
                .rounded(move || theme().radius)
                .hover(|s| s.bg(theme().accent))
                .style_when(selected, |s| s.bg(theme().sidebar_accent))
                .child(icon(paths).text_color(move || theme().muted_foreground))
                .child(
                    text(entry.label.clone())
                        .text_sm()
                        .text_color(move || theme().foreground),
                );
            item_menu(id.to_string(), kind, row)
        })
}

/// What a right click on a row (a held finger on a phone) offers: in a
/// folder, new items in it; for every item, renaming and deleting it.
fn item_menu(path: String, kind: Option<EntryKind>, row: View) -> impl IntoElement {
    let folder = (kind == Some(EntryKind::Folder)).then(|| {
        let (note_dir, folder_dir) = (path.clone(), path.clone());
        view()
            .child(menu_entry(
                icons::FILE_PLUS,
                t("new_note"),
                false,
                move || store::new_note_in(&note_dir),
            ))
            .child(menu_entry(
                icons::FOLDER_PLUS,
                t("new_folder"),
                false,
                move || store::new_folder_in(&folder_dir),
            ))
            .child(separator())
    });
    let (rename, delete) = (path.clone(), path);
    context_menu()
        .popup_style(menu_popup)
        .backdrop_style(|s| s.bg(overlay()))
        // The store follows the menu: the press a held finger goes on to
        // report to the tree acts on nothing while it is up.
        .on_open_change(|e: &ContextMenuEvents| {
            if let ContextMenuEvents::OpenChange(open, _) = e {
                store::set_row_menu_open(*open);
            }
        })
        .trigger(row)
        .child(folder)
        .child(menu_entry(icons::PENCIL, t("rename"), false, move || {
            store::start_rename(rename.clone())
        }))
        .child(separator())
        .child(menu_entry(icons::TRASH, t("delete"), true, move || {
            store::delete(delete.clone())
        }))
}

fn separator() -> impl IntoElement {
    menu_separator()
        .h(px(1.0))
        .my_1()
        .bg(move || theme().border)
}

/// The dialog renaming the item at `path` (a note by its title), centered
/// over a dimmed backdrop: Return or Rename renames; Cancel, Escape or the
/// backdrop closes it.
pub(crate) fn rename_dialog(path: String) -> impl IntoElement {
    let name = path.rsplit('/').next().unwrap_or(&path);
    let name = name.strip_suffix(".md").unwrap_or(name).to_owned();
    let typed = signal(name.clone());
    let field = field(t("name"))
        .label(t("name"))
        .default_value(name)
        .auto_focus(true)
        .on_change(move |text: &str| typed.set(text.to_owned()))
        .on_submit(|text: &str| store::rename(text));
    let buttons = view()
        .flex_row()
        .justify_end()
        .gap_2()
        .child(button(
            ButtonVariant::Secondary,
            t("cancel"),
            store::cancel_rename,
        ))
        .child(button(ButtonVariant::Primary, t("rename"), move || {
            store::rename(&typed.get())
        }));
    modal(
        store::cancel_rename,
        dialog_card(t("rename"), Some(field.into_element()), buttons),
    )
}
