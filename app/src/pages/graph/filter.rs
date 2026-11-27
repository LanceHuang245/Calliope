//! The graph's filters: the notes without links, those not yet created,
//! and a name that keeps only the nodes it matches.

use calliope_core::graph::Graph;
use nsh_base::{MenuButtonItem, button, menu_button, theme};
use nsh_core::prelude::*;

use crate::{
    components::{field, icons},
    i18n::t,
};

/// The field keeping the nodes whose name holds what is typed (case
/// aside); `wide` stretches it, as the phones' row under the header does.
pub(super) fn name_filter(typed: Signal<String>, wide: bool) -> impl IntoElement {
    let filter = field(t("filter_nodes"))
        .label(t("filter_nodes"))
        .on_change(move |text: &str| typed.set(text.to_owned()));
    match wide {
        true => filter.flex_1(),
        false => filter.w(px(160.0)),
    }
}

/// The system menu reads both filters independently each time it opens.
pub(super) fn mobile_filters(orphans: Signal<bool>, missing: Signal<bool>) -> impl IntoElement {
    menu_button()
        .test_id("graph-filters")
        .accessibility_label(t("filters"))
        .items(move || {
            vec![
                MenuButtonItem::new("orphans", t("orphans")).checked(orphans.get()),
                MenuButtonItem::new("uncreated", t("uncreated")).checked(missing.get()),
            ]
        })
        .on_select(move |id: &SharedString| match id.as_ref() {
            "orphans" => orphans.update(|on| *on = !*on),
            "uncreated" => missing.update(|on| *on = !*on),
            _ => {}
        })
        .w(px(44.0))
        .h(px(44.0))
        .items_center()
        .justify_center()
        .child(
            icons::icon(icons::FILTER)
                .w(px(20.0))
                .h(px(20.0))
                .pointer_events(PointerEvents::None)
                .text_color(move || theme().foreground),
        )
}

/// One of the two filters beside the field (the phones keep them in a menu).
pub(super) fn option(label: impl Into<SharedString>, on: Signal<bool>) -> impl IntoElement {
    let label: SharedString = label.into();
    button()
        .px_2()
        .py_1()
        .accessibility_label(label.clone())
        .selected(move || on.get())
        .on_press(move || on.update(|on| *on = !*on))
        .child(text(label).text_sm().text_color(move || theme().foreground))
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .style_when(move || on.get(), |s| s.bg(theme().accent))
}

/// The nodes whose name holds `query`, and the links between them; nothing
/// typed keeps them all. Their count of links stays the whole graph's, so a
/// filtered graph keeps the sizes it had.
pub(super) fn named(graph: Graph, query: &str) -> Graph {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return graph;
    }
    let mut kept = vec![usize::MAX; graph.nodes.len()];
    let mut kept_graph = Graph::default();
    for (i, node) in graph.nodes.iter().enumerate() {
        if node.label.to_lowercase().contains(&query) {
            kept[i] = kept_graph.nodes.len();
            kept_graph.nodes.push(node.clone());
        }
    }
    kept_graph.edges = graph
        .edges
        .iter()
        .filter(|&&(a, b)| kept[a] != usize::MAX && kept[b] != usize::MAX)
        .map(|&(a, b)| (kept[a], kept[b]))
        .collect();
    kept_graph
}

#[cfg(test)]
mod tests {
    use super::named;
    use calliope_core::graph::{Graph, Node};

    fn node(id: &str) -> Node {
        Node {
            id: id.to_owned(),
            label: id.to_owned(),
            exists: true,
            degree: 1,
        }
    }

    #[test]
    fn a_name_keeps_the_nodes_it_matches_and_the_links_between_them() {
        let graph = Graph {
            nodes: ["Alpha", "Alphabet", "Bee"].map(node).to_vec(),
            edges: vec![(0, 1), (1, 2)],
        };
        let kept = named(graph.clone(), " alpha ");
        let ids: Vec<_> = kept.nodes.iter().map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["Alpha", "Alphabet"]);
        assert_eq!(kept.edges, [(0, 1)]);
        // Nothing typed keeps every node and every link.
        assert_eq!(named(graph.clone(), "  "), graph);
        // A name no node holds keeps none.
        assert!(named(graph, "zed").nodes.is_empty());
    }
}
