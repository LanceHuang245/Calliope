//! The phones' keyboard toolbar (MVP phase 4): over the keyboard while the
//! note's source is edited, the Markdown a phone's keyboard hides deepest:
//! a link, a tag, a list item, a task, code.
//!
//! nsh's `input_accessory_view` is iOS's own and does not reach the
//! `Editor`, so the bar is the app's: placed by the keyboard's height, it
//! edits through `EditorApi` on both systems.

use nsh_base::{
    Editor, EditorApi, EditorPosition, EditorRange, EditorSelection, editor::EditorEdit, theme,
};
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::components::icons::{self, icon};

/// The bar of `editor`, shown while `editing` (the editor has the focus)
/// and the keyboard is up.
pub(crate) fn keyboard_bar(
    editor: ComponentRef<Editor>,
    editing: Signal<bool>,
) -> impl IntoElement {
    // A press runs `f` on the editor, which keeps the focus and the
    // keyboard (the buttons take no focus).
    let run = move |f: fn(&EditorApi)| {
        move || {
            if let Some(api) = editor.api::<EditorApi>() {
                f(&api);
            }
        }
    };
    let shown = move || editing.get() && keyboard_height() > px(0.0);
    when(shown, move || {
        view()
            .absolute()
            .left_0()
            .right_0()
            // Right over the keyboard: its height counts the bottom safe
            // area, which the page is already padded by.
            .bottom(move || (keyboard_height() - safe_area().bottom).max(px(0.0)))
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .border_t_1()
            .border_color(move || theme().border)
            .bg(move || theme().background)
            .child(key(
                text("[[").into_element(),
                t("link"),
                // Inserted text is not typed: the notes' titles are asked for.
                run(|api| {
                    api.insert("[[".into());
                    api.show_completions();
                }),
            ))
            .child(key(
                text("#").into_element(),
                t("tag"),
                run(|api| api.insert("#".into())),
            ))
            .child(key(
                icon(icons::LIST).into_element(),
                t("list"),
                run(|api| prefix_line(api, "- ")),
            ))
            .child(key(
                icon(icons::SQUARE_CHECK).into_element(),
                t("task"),
                run(|api| prefix_line(api, "- [ ] ")),
            ))
            .child(key(
                icon(icons::CODE).into_element(),
                t("code"),
                run(|api| wrap(api, "`")),
            ))
    })
}

/// A button of the bar showing `content`, named by `label`; it takes no
/// focus, so the editor keeps the keyboard.
fn key(
    content: AnyElement,
    label: impl Into<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    let label: SharedString = label.into();
    view()
        .focusable(false)
        .role(AccessibilityRole::Button)
        .min_w(px(44.0))
        .h(px(40.0))
        .items_center()
        .justify_center()
        .rounded(move || theme().radius)
        .active(|s| s.bg(theme().accent))
        .text_color(move || theme().foreground)
        .label(label)
        .on_press(on_press)
        .child(content)
}

/// Starts the caret's line with `marker` (a list item's, a task's).
fn prefix_line(api: &EditorApi, marker: &str) {
    let Some(caret) = api.selections().last().map(|s| s.head) else {
        return;
    };
    let start = EditorPosition::new(caret.line, 0);
    api.edit(vec![EditorEdit::new(
        EditorRange::new(start, start),
        marker.to_owned(),
    )]);
}

/// Puts `mark` on both sides of the selection, which stays on the text
/// between them (the caret, when nothing is selected).
fn wrap(api: &EditorApi, mark: &str) {
    let Some(selection) = api.selections().last().copied() else {
        return;
    };
    let start = selection.anchor.min(selection.head);
    let end = selection.anchor.max(selection.head);
    // Where the text will be, in UTF-16 units as `mark`'s length: taken
    // before the edit moves what follows `start`.
    let shift = mark.encode_utf16().count() as u32;
    let offsets = [start, end].map(|p| api.position_to_offset(p) + shift);
    let text = api.selected_text();
    api.edit(vec![EditorEdit::new(
        EditorRange::new(start, end),
        format!("{mark}{text}{mark}"),
    )]);
    let [start, end] = offsets.map(|o| api.offset_to_position(o));
    api.set_selections(vec![EditorSelection::new(start, end)]);
}
