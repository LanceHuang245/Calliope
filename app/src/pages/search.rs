//! The full-text search (`mod+shift+f`): a field over every note's text,
//! and under it the lines that hold the query, each opening its note at
//! that line (MVP phase 6).

use calliope_core::{note, search::Hit};
use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::{
    components::{field, muted, page},
    store, strings,
};

pub(crate) fn search_page() -> impl IntoElement {
    let query = signal(String::new());
    let content = view()
        .flex_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(move || theme().border)
                .child(
                    field(strings::SEARCH_NOTES)
                        .label(strings::SEARCH_NOTES)
                        .value(query)
                        .auto_focus(true)
                        .on_change(move |text: &str| store::set_query(text))
                        .flex_1(),
                ),
        )
        .child(scroll().flex_1().child(dynamic(results)));
    page(strings::SEARCH, store::back_to_note, None, content)
}

/// The hits, or what to do before there are any.
fn results() -> AnyElement {
    let hits = store::hits();
    let typed = !store::query().trim().is_empty();
    untracked(|| match (hits.is_empty(), typed) {
        (false, _) => list(hits).into_element(),
        (true, true) => view().p_6().child(muted(strings::NO_HITS)).into_element(),
        (true, false) => view()
            .p_6()
            .child(muted(strings::TYPE_TO_SEARCH))
            .into_element(),
    })
}

fn list(hits: Vec<Hit>) -> impl IntoElement {
    view()
        .w_full()
        .max_w(px(720.0))
        .p_4()
        .gap(px(2.0))
        .child(hits.into_iter().map(row))
}

/// A hit: the note's title and where it is, the line under them; a press
/// opens the note there.
fn row(hit: Hit) -> impl IntoElement {
    let opened = hit.clone();
    let where_ = format!("{}:{}", hit.path, hit.line);
    button()
        .w_full()
        .items_start()
        .gap(px(2.0))
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .active(|s| s.bg(theme().accent))
        .on_press(move || store::open_hit(&opened))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    text(note::title(&hit.path).to_owned())
                        .text_sm()
                        .text_color(move || theme().foreground),
                )
                .child(muted(where_)),
        )
        .child(
            text(hit.context)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}
