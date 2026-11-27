//! A note: editing its Markdown, reading it rendered, or both side by side
//! (the view mode).
//!
//! Editing uses `nsh-base`'s `Editor`; reading draws the Markdown with its
//! `TextView` (code highlighted, formulas typeset natively).

use std::time::Duration;

use calliope_core::note;
use nsh_base::{
    Editor, EditorApi, EditorClasses, EditorCommand, EditorSearch, MarkdownPlugins, TextViewLinks,
    editor::{EditorCompletionItem, EditorCompletionRequest},
    h_resizable, input, resizable_panel, text_view,
    text_view::{CustomNode, Image},
    theme,
};
use nsh_core::{
    keymap::When,
    mobile::{SegmentedControlItem, segmented_control},
    platform::Os,
    prelude::*,
};

use super::{
    home::{home_page, menu_button, phone_toolbar},
    keyboard_bar::keyboard_bar,
    links::{WikiLinks, backlinks_sheet, wiki_links},
    mermaid::{mermaid, mermaid_blocks, warm_up},
};
use crate::i18n::t;

use crate::{
    components::{
        TOOLBAR_ROOM, action, banner, desktop_layout, field, icon_button, icons, segmented,
    },
    store::{self, Mode},
};

/// Below this width, Split shows the editor only.
const SPLIT_MIN_WIDTH: f32 = 720.0;

pub(crate) fn note_page() -> impl IntoElement {
    // Each note opened gets a new editor (its own undo history); renaming
    // the open note keeps it.
    let opened = computed(store::opened);
    dynamic(move || match opened.get() {
        Some(_) => untracked(note_view).into_element(),
        None => home_page().into_element(),
    })
}

fn note_view() -> impl IntoElement {
    let finding = signal(false);
    let query = signal(String::new());
    let editor = component_ref::<Editor>();
    // A search hit opens its note at its line.
    jump_to_line(editor);
    // The source has the focus: a phone's keyboard toolbar shows.
    let editing = signal(false);
    // A note with diagrams: their fonts are found while it is edited.
    if untracked(store::text).contains("```mermaid") {
        warm_up();
    }
    // Whether Split fits: a memo, so resizing the window rebuilds nothing
    // until it crosses the width.
    let wide = computed(move || window().width >= px(SPLIT_MIN_WIDTH));
    on_key_with(
        "mod+f",
        KeyOptions {
            when: When::Always,
            ..Default::default()
        },
        move |_| finding.set(true),
    );
    // A press on a blank part leaves the title (which renames).
    let page = view()
        .flex_1()
        .on_press(dismiss_keyboard)
        .child(header())
        .child(dynamic(move || {
            store::conflict().then(|| {
                banner(t("conflict"))
                    .child(action(t("keep_mine"), || store::resolve_conflict(true)))
                    .child(action(t("use_disk"), || store::resolve_conflict(false)))
            })
        }))
        .child(dynamic(move || {
            (finding.get() && store::mode() != Mode::Read).then(|| find_bar(editor, finding, query))
        }))
        .child(dynamic(move || {
            let mode = store::mode();
            let wide = wide.get();
            untracked(|| match mode {
                Mode::Edit => source(editor, query, editing).into_element(),
                Mode::Read => preview().into_element(),
                Mode::Split if !wide => source(editor, query, editing).into_element(),
                Mode::Split => h_resizable()
                    .flex_1()
                    .handle_style(|s| s.bg(theme().border))
                    .child(resizable_panel().child(source(editor, query, editing)))
                    .child(resizable_panel().child(preview()))
                    .into_element(),
            })
        }))
        // A phone shows the backlinks in a sheet when asked.
        .child((!desktop_layout()).then(backlinks_sheet))
        .child((!desktop_layout()).then(phone_toolbar))
        .child((!desktop_layout()).then(|| keyboard_bar(editor, editing)));
    // The grammar issue of the word under the pointer, or of the one a
    // finger pressed (where Harper is built: the desktops and the phones).
    #[cfg(any(target_os = "windows", target_os = "android", target_os = "ios"))]
    let page = page.child(store::grammar::hover_card(editor));
    page
}

/// Scrolls the editor to the line a search hit opened its note at, once
/// the editor holds the note: it is made a moment after the page.
fn jump_to_line(editor: ComponentRef<Editor>) {
    effect(move || {
        let Some(line) = store::jump_line() else {
            return;
        };
        spawn(async move {
            for _ in 0..20 {
                if let Some(api) = editor.api::<EditorApi>() {
                    api.scroll_to_line(line.saturating_sub(1));
                    store::clear_jump_line();
                    return;
                }
                timer(Duration::from_millis(25)).await;
            }
        });
    });
}

fn header() -> impl IntoElement {
    let modes = match desktop_layout() {
        true => vec![
            (Mode::Edit, t("edit")),
            (Mode::Split, t("split")),
            (Mode::Read, t("read")),
        ],
        false => vec![(Mode::Edit, t("edit")), (Mode::Read, t("read"))],
    };
    // Use UIKit on an iPhone or narrow iPad, sharing the app's mode state.
    let mode_picker = if cfg!(target_os = "ios") && os() == Os::Ios && !desktop_layout() {
        segmented_control()
            .items(vec![
                SegmentedControlItem::new(t("edit")),
                SegmentedControlItem::new(t("read")),
            ])
            .selected_index(|| u32::from(store::mode() == Mode::Read))
            .on_change(|index: &u32| {
                store::set_mode(if *index == 0 { Mode::Edit } else { Mode::Read })
            })
            .into_element()
    } else {
        segmented(modes, store::mode, store::set_mode).into_element()
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .child((!desktop_layout()).then(menu_button))
        .child(title())
        .child(mode_picker)
}

/// The note's title, edited in place: Return or leaving it renames the
/// file; a name that cannot be taken goes back to the file's.
fn title() -> impl IntoElement {
    let title = signal(untracked(store::note_title));
    let commit = move || {
        store::rename_note(&title.get());
        title.set(untracked(store::note_title));
    };
    input()
        .value(title)
        .label(t("title"))
        .on_change(move |text: &str| title.set(text.to_owned()))
        .on_submit(move |_: &str| commit())
        .on_blur(commit)
        .flex_1()
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().foreground)
        .hover(|s| s.bg(theme().accent))
}

fn source(
    editor: ComponentRef<Editor>,
    query: Signal<String>,
    editing: Signal<bool>,
) -> impl IntoElement {
    // Room around the text: the gutter (its fold markers) already makes the
    // left side's; its rows move down with the text's.
    // A phone's last lines scroll clear of the floating toolbar.
    let bottom = match desktop_layout() {
        true => "pb-4".to_owned(),
        false => format!("pb-[{TOOLBAR_ROOM}px]"),
    };
    let mut classes = EditorClasses::default();
    classes.gutter = Some(format!("pt-4 {bottom} pl-1 pr-0").into());
    classes.content = Some(format!("pt-4 {bottom} pl-0 pr-4").into());
    // A finger picks the notes `[[` offers on a phone: their titles larger,
    // in a list wide enough for them (its rows keep the editor's height).
    if !is_desktop() {
        classes.completion_item = Some("text-base".into());
        classes.completion_menu = Some("min-w-[240px]".into());
    }
    let source = nsh_base::editor();
    #[cfg(any(target_os = "windows", target_os = "android", target_os = "ios"))]
    let source = store::grammar::hooks(source, editor);
    source
        .value(move || Some(store::text()))
        .on_change(store::edit)
        .language("markdown")
        .accessibility_label(t("source"))
        .soft_wrap(true)
        .line_numbers(false)
        .active_line(false)
        .classes(classes)
        .search(move || {
            let query = query.get();
            (!query.is_empty()).then(|| EditorSearch::new(query))
        })
        // `[[` offers the notes' titles.
        .completion(true)
        .completion_trigger_characters(vec![SharedString::from("[[")])
        .on_completion_request(move |request: &EditorCompletionRequest| {
            let Some(api) = editor.api::<EditorApi>() else {
                return;
            };
            let before: Vec<u16> = api
                .value()
                .encode_utf16()
                .take(request.query_start as usize)
                .collect();
            let items = match before.ends_with(&[u16::from(b'['); 2]) {
                true => store::note_paths()
                    .iter()
                    .map(|path| EditorCompletionItem::new(note::title(path).to_owned()))
                    .collect(),
                false => Vec::new(),
            };
            api.present_completions(request.id, items, None);
        })
        .on_focus(move || editing.set(true))
        .on_blur(move || editing.set(false))
        .component_ref(&editor)
        .flex_1()
}

/// Find and replace over the source (`mod+f`).
fn find_bar(
    editor: ComponentRef<Editor>,
    finding: Signal<bool>,
    query: Signal<String>,
) -> impl IntoElement {
    let replacement = signal(String::new());
    let run = move |f: &dyn Fn(&EditorApi)| {
        if let Some(api) = editor.api::<EditorApi>() {
            f(&api);
        }
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .border_b_1()
        .border_color(move || theme().border)
        .child(
            field(t("find"))
                .value(query)
                .auto_focus(true)
                .on_change(move |q: &str| query.set(q.to_owned()))
                .on_submit(move |_: &str| run(&|api| api.run(EditorCommand::FindNext)))
                .flex_1(),
        )
        .child(icon_button(
            icons::CHEVRON_UP,
            t("previous_match"),
            move || run(&|api| api.run(EditorCommand::FindPrevious)),
        ))
        .child(icon_button(
            icons::CHEVRON_DOWN,
            t("next_match"),
            move || run(&|api| api.run(EditorCommand::FindNext)),
        ))
        .child(
            field(t("replace"))
                .value(replacement)
                .on_change(move |r: &str| replacement.set(r.to_owned()))
                .flex_1(),
        )
        .child(action(t("replace"), move || {
            run(&|api| api.replace(replacement.get()))
        }))
        .child(action(t("replace_all"), move || {
            run(&|api| {
                api.replace_all(replacement.get());
            })
        }))
        .child(icon_button(icons::X, t("close"), move || {
            finding.set(false);
            query.set(String::new());
        }))
}

/// The rendered note: its front matter folded away, then its Markdown.
fn preview() -> impl IntoElement {
    let parts = computed(move || {
        let text = store::preview();
        let (front, body) = note::split_front_matter(&text);
        (
            front.map(str::to_owned),
            SharedString::from(body.to_owned()),
        )
    });
    provide_context(MarkdownPlugins::new().with(WikiLinks));
    // A phone's last lines scroll clear of the floating toolbar.
    let bottom = match desktop_layout() {
        true => px(24.0),
        false => px(TOOLBAR_ROOM),
    };
    scroll().flex_1().child(
        view()
            .p_6()
            .pb(bottom)
            .gap_2()
            .child(dynamic(move || parts.get().0.map(front_matter)))
            .child(
                text_view()
                    .markdown(move || parts.get().1)
                    .selectable(true)
                    .links(TextViewLinks::Open)
                    .extensions(vec![wiki_links(), mermaid_blocks()])
                    // The wiki links are `WikiLinks`' to show.
                    .render_custom(|node: &CustomNode| mermaid(node))
                    .render_image(|img: &Image| vault_image(img))
                    .text_color(move || theme().foreground),
            ),
    )
}

/// The front matter: folded, unfolded by a press.
fn front_matter(yaml: String) -> impl IntoElement {
    let open = signal(false);
    let source = SharedString::from(format!("```yaml\n{yaml}```"));
    view()
        .child(action(t("properties"), move || open.update(|o| *o = !*o)))
        .child(dynamic(move || {
            open.get().then(|| {
                text_view()
                    .markdown(source.clone())
                    .text_color(theme().foreground)
            })
        }))
}

/// An image of the note: a file of the vault next to the note, or a URL;
/// as wide as it is, at most as wide as the text.
fn vault_image(img: &Image) -> impl IntoElement + use<> {
    let src = store::image_src(&img.src);
    let natural = signal(None::<(f32, f32)>);
    let measured = image_size(src.clone());
    spawn(async move {
        if let Ok(size) = measured.await {
            natural.set(Some((size.width.0 as f32, size.height.0 as f32)));
        }
    });
    view()
        .w(move || natural.get().map_or(px(0.0), |(w, _)| px(w)))
        .max_w(relative(1.0))
        .child(
            image(src)
                .object_fit(ResizeMode::Contain)
                .label(SharedString::from(img.alt.clone()))
                .rounded(px(8.0))
                .w_full()
                .aspect_ratio(move || {
                    natural
                        .get()
                        .filter(|(w, h)| *w > 0.0 && *h > 0.0)
                        .map_or(1.0, |(w, h)| w / h)
                }),
        )
}
