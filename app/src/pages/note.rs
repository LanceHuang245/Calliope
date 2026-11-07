//! A note: editing its Markdown, reading it rendered, or both side by side
//! (the view mode).
//!
//! Editing uses `nsh-base`'s `Editor`; reading draws the Markdown with its
//! `TextView` (code highlighted, formulas typeset natively).

use calliope_core::note;
use nsh_base::{
    Editor, EditorApi, EditorCommand, EditorSearch, TextViewLinks, h_resizable, input,
    resizable_panel, text_view, text_view::Image, theme,
};
use nsh_core::{keymap::When, prelude::*};

use super::home::{home_page, menu_button};
use crate::{
    components::{action, banner, field, icon_button, icons, segmented},
    store::{self, Mode},
    strings,
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
    view()
        .flex_1()
        .on_press(dismiss_keyboard)
        .child(header())
        .child(dynamic(move || {
            store::conflict().then(|| {
                banner(strings::CONFLICT)
                    .child(action(strings::KEEP_MINE, || store::resolve_conflict(true)))
                    .child(action(strings::USE_DISK, || store::resolve_conflict(false)))
            })
        }))
        .child(dynamic(move || {
            (finding.get() && store::mode() != Mode::Read).then(|| find_bar(editor, finding, query))
        }))
        .child(dynamic(move || {
            let mode = store::mode();
            let wide = wide.get();
            untracked(|| match mode {
                Mode::Edit => source(editor, query).into_element(),
                Mode::Read => preview().into_element(),
                Mode::Split if !wide => source(editor, query).into_element(),
                Mode::Split => h_resizable()
                    .flex_1()
                    .handle_style(|s| s.bg(theme().border))
                    .child(resizable_panel().child(source(editor, query)))
                    .child(resizable_panel().child(preview()))
                    .into_element(),
            })
        }))
}

fn header() -> impl IntoElement {
    let modes = match is_desktop() {
        true => vec![
            (Mode::Edit, strings::EDIT),
            (Mode::Split, strings::SPLIT),
            (Mode::Read, strings::READ),
        ],
        false => vec![(Mode::Edit, strings::EDIT), (Mode::Read, strings::READ)],
    };
    view()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .bg(move || theme().sidebar)
        .border_b_1()
        .border_color(move || theme().sidebar_border)
        .shadow_sm()
        .child((!is_desktop()).then(menu_button))
        .child(title())
        .child(segmented(modes, store::mode, store::set_mode))
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
        .label(strings::TITLE)
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

fn source(editor: ComponentRef<Editor>, query: Signal<String>) -> impl IntoElement {
    nsh_base::editor()
        .value(move || Some(store::text()))
        .on_change(store::edit)
        .language("markdown")
        .accessibility_label(strings::SOURCE)
        .soft_wrap(true)
        .line_numbers(false)
        .search(move || {
            let query = query.get();
            (!query.is_empty()).then(|| EditorSearch::new(query))
        })
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
            field(strings::FIND)
                .value(query)
                .auto_focus(true)
                .on_change(move |q: &str| query.set(q.to_owned()))
                .on_submit(move |_: &str| run(&|api| api.run(EditorCommand::FindNext)))
                .flex_1(),
        )
        .child(icon_button(
            icons::CHEVRON_UP,
            strings::PREVIOUS_MATCH,
            move || run(&|api| api.run(EditorCommand::FindPrevious)),
        ))
        .child(icon_button(
            icons::CHEVRON_DOWN,
            strings::NEXT_MATCH,
            move || run(&|api| api.run(EditorCommand::FindNext)),
        ))
        .child(
            field(strings::REPLACE)
                .value(replacement)
                .on_change(move |r: &str| replacement.set(r.to_owned()))
                .flex_1(),
        )
        .child(action(strings::REPLACE, move || {
            run(&|api| api.replace(replacement.get()))
        }))
        .child(action(strings::REPLACE_ALL, move || {
            run(&|api| {
                api.replace_all(replacement.get());
            })
        }))
        .child(icon_button(icons::X, strings::CLOSE, move || {
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
    scroll().flex_1().child(
        view()
            .p_6()
            .gap_2()
            .child(dynamic(move || parts.get().0.map(front_matter)))
            .child(
                text_view()
                    .markdown(move || parts.get().1)
                    .selectable(true)
                    .links(TextViewLinks::Open)
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
        .child(action(strings::PROPERTIES, move || {
            open.update(|o| *o = !*o)
        }))
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
