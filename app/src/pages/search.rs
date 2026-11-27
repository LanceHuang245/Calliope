//! The full-text search (`mod+shift+f`): a field over every note's text,
//! and under it the lines that hold the query, each opening its note at
//! that line (MVP phase 6).

use calliope_core::{note, search::Hit};
use nsh_base::{button, theme};
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::{
    components::{field, muted, page},
    store,
};

pub(crate) fn search_page() -> impl IntoElement {
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
                    field(t("search_notes"))
                        .label(t("search_notes"))
                        .value(|| store::query().to_string())
                        .auto_focus(true)
                        .on_change(store::set_query)
                        .flex_1(),
                ),
        )
        .child(scroll().flex_1().child(dynamic(results)));
    page(t("search"), store::back_to_note, None, content)
}

/// The hits, or what to do before there are any.
fn results() -> AnyElement {
    let results = store::search_results();
    let typed = !store::query().trim().is_empty();
    untracked(|| {
        let message = match results {
            Some((hits, truncated)) if !hits.is_empty() => {
                return list(hits, truncated).into_element();
            }
            Some(_) => "no_hits",
            None if typed => "searching_notes",
            None => "type_to_search",
        };
        view().p_6().child(muted(t(message))).into_element()
    })
}

fn list(hits: Vec<Hit>, truncated: bool) -> impl IntoElement {
    view()
        .w_full()
        .max_w(px(720.0))
        .p_4()
        .gap(px(2.0))
        .child(truncated.then(|| muted(t("search_truncated"))))
        .child(hits.into_iter().map(row))
}

/// A hit: the note's title and where it is, the line under them; a press
/// opens the note there.
fn row(
    Hit {
        path,
        line,
        context,
    }: Hit,
) -> impl IntoElement {
    let title = note::title(&path).to_owned();
    let location = format!("{path}:{line}");
    let path = SharedString::from(path);
    button()
        .w_full()
        .items_start()
        .gap(px(2.0))
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .active(|s| s.bg(theme().accent))
        .on_press(move || store::open_hit(&path, line))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(text(title).text_sm().text_color(move || theme().foreground))
                .child(muted(location)),
        )
        .child(
            text(context)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}
