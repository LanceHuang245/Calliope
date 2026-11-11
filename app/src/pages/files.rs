//! The vault's files: a tree of folders and notes. The header creates
//! notes and folders, the row under the tree holds the app's actions; each
//! row's context menu (a right click, or a held finger on a phone) acts on
//! its item. The desktops show it in the sidebar, the phones as a screen.

use calliope_core::vault::EntryKind;
use nsh_base::{TreeEntry, context_menu, dialog, menu_separator, theme, tree};
use nsh_core::prelude::*;

use crate::{
    components::{
        action, card, field, icon_button,
        icons::{self, icon},
        menu_entry, menu_popup, overlay, primary,
    },
    store::{self, Route},
    strings,
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
                    (!is_desktop()).then(|| {
                        icon_button(icons::X, strings::CLOSE, || store::set_menu_open(false))
                    }),
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
                    strings::NEW_NOTE,
                    store::new_note,
                ))
                .child(icon_button(
                    icons::FOLDER_PLUS,
                    strings::NEW_FOLDER,
                    store::new_folder,
                )),
        )
        .child(card().flex_1().p_1().child(dynamic(move || {
            match store::has_files() {
                true => files_tree().into_element(),
                false => text(strings::NO_FILES)
                    .p_2()
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
                    .into_element(),
            }
        })))
        .child(
            // The app's actions, in one row under the files.
            view()
                .flex_row()
                .gap_1()
                .pt_1()
                .child(icon_button(icons::SEARCH, strings::SEARCH, || {
                    store::set_switching(true)
                }))
                .child(icon_button(icons::NETWORK, strings::GRAPH, || {
                    store::set_route(Route::Graph)
                }))
                .child(
                    is_desktop().then(|| {
                        icon_button(icons::LINK, strings::BACKLINKS, store::toggle_backlinks)
                    }),
                )
                .child(view().flex_1())
                .child(icon_button(icons::SETTINGS, strings::SETTINGS, || {
                    store::set_route(Route::Settings)
                })),
        )
}

fn files_tree() -> impl IntoElement {
    tree()
        .items(store::tree_items)
        .expanded(store::expanded)
        .selected(store::selected)
        .on_expanded_change(|expanded: &Vec<SharedString>| store::set_expanded(expanded))
        .on_selected_change(store::select)
        .accessibility_label(strings::FILES)
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
                move || store::selected().as_ref() == Some(&id)
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
                strings::NEW_NOTE,
                false,
                move || store::new_note_in(&note_dir),
            ))
            .child(menu_entry(
                icons::FOLDER_PLUS,
                strings::NEW_FOLDER,
                false,
                move || store::new_folder_in(&folder_dir),
            ))
            .child(separator())
    });
    let (rename, delete) = (path.clone(), path);
    context_menu()
        .popup_style(menu_popup)
        .backdrop_style(|s| s.bg(overlay()))
        .trigger(row)
        .child(folder)
        .child(menu_entry(
            icons::PENCIL,
            strings::RENAME,
            false,
            move || store::start_rename(rename.clone()),
        ))
        .child(separator())
        .child(menu_entry(icons::TRASH, strings::DELETE, true, move || {
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
    dialog()
        .animation(ModalAnimation::Fade)
        .on_cancel(store::cancel_rename)
        .flex_1()
        .child(
            view()
                .absolute()
                .inset_0()
                .bg(overlay)
                .on_press(store::cancel_rename),
        )
        .child(
            card()
                .m(auto())
                .w_80()
                .gap_3()
                .p_5()
                .shadow_lg()
                .child(
                    text(strings::RENAME)
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(
                    field(strings::NAME)
                        .label(strings::NAME)
                        .default_value(name)
                        .auto_focus(true)
                        .on_change(move |text: &str| typed.set(text.to_owned()))
                        .on_submit(|text: &str| store::rename(text)),
                )
                .child(
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(action(strings::CANCEL, store::cancel_rename))
                        .child(primary(strings::RENAME, move || {
                            store::rename(&typed.get())
                        })),
                ),
        )
}
