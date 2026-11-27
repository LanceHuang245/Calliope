//! The graph's canvas: every node, edge and title drawn once for a layout,
//! and the hovered node's own neighborhood over them.

use std::{collections::HashMap, collections::HashSet, f32::consts::TAU, rc::Rc};

use calliope_core::graph::Graph;
use nsh_base::theme;
use nsh_core::{
    canvas::{CanvasNode, CanvasTextAlign},
    color::Hsla,
    element::AdaptiveColor,
    prelude::*,
};

/// The room around the outermost nodes, for their labels.
pub(super) const MARGIN: f32 = 80.0;

/// A graph and where the layout put its nodes.
pub(super) type Laid = Rc<(Graph, Vec<(f32, f32)>)>;
/// Where the user dragged the nodes, by the node's id, from the layout's
/// origin; this device's own, in the vault's store.
pub(super) type Dragged = Signal<HashMap<String, (f32, f32)>>;

/// Where each node is drawn: where it was dragged to, or where the layout
/// put it.
pub(super) fn placed_at(laid: &Laid, dragged: &HashMap<String, (f32, f32)>) -> Vec<(f32, f32)> {
    (0..laid.0.nodes.len())
        .map(|i| match dragged.get(&laid.0.nodes[i].id) {
            Some(&at) => at,
            None => laid.1[i],
        })
        .collect()
}

/// Every node, edge and title, on a canvas of `half` × 2 points square
/// around the origin. Each node is an element of its own to screen readers
/// and tests.
pub(super) fn every_node(laid: &Laid, half: f32, dragged: Dragged) -> Painter {
    let edges = theme().muted_foreground.opacity(0.4);
    let mut p = paint(laid, (half, half), dragged, |_, _| true, |_| true, edges);
    for (node, at) in laid.0.nodes.iter().zip(placed_at(laid, &dragged.get())) {
        let r = radius(node.degree);
        let corner = point(px(half + at.0 - r), px(half + at.1 - r));
        let area = Bounds::new(corner, size(px(r * 2.0), px(r * 2.0)));
        let button = CanvasNode::new(area, AccessibilityRole::Button, node.label.as_str());
        p = p.node(button.test_id(node.id.as_str()));
    }
    p
}

/// The `hovered` node, its neighbors and their edges, on a canvas just
/// around them (and their titles) over the dimmed graph: quick to draw
/// again as the pointer moves, whatever the graph's size.
pub(super) fn neighbors(
    laid: Laid,
    half: f32,
    hovered: usize,
    dragged: Dragged,
) -> impl IntoElement {
    let touches = move |a: usize, b: usize| a == hovered || b == hovered;
    let near: HashSet<usize> = (laid.0.edges.iter().filter(|&&(a, b)| touches(a, b)))
        .flat_map(|&(a, b)| [a, b])
        .chain([hovered])
        .collect();
    let at = placed_at(&laid, &dragged.get());
    let (x0, y0, x1, y1) = near.iter().fold(
        (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
        |(x0, y0, x1, y1), &i| {
            (
                x0.min(at[i].0),
                y0.min(at[i].1),
                x1.max(at[i].0),
                y1.max(at[i].1),
            )
        },
    );
    // Its corner on the graph's canvas.
    let (left, top) = (half + x0 - MARGIN, half + y0 - MARGIN);
    canvas(move || {
        let shift = (half - left, half - top);
        paint(
            &laid,
            shift,
            dragged,
            touches,
            |i| near.contains(&i),
            theme().primary,
        )
    })
    .absolute()
    .left(px(left))
    .top(px(top))
    .w(px(x1 - x0 + MARGIN * 2.0))
    .h(px(y1 - y0 + MARGIN * 2.0))
}

/// A node's radius: larger with more links.
pub(super) fn radius(degree: usize) -> f32 {
    3.0 + (degree as f32).sqrt() * 1.5
}

/// The edges `edge` keeps in `stroke`, then the nodes `node` keeps with
/// their titles, `shift` from the layout's origin.
fn paint(
    laid: &Laid,
    shift: (f32, f32),
    dragged: Dragged,
    edge: impl Fn(usize, usize) -> bool,
    node: impl Fn(usize) -> bool,
    stroke: AdaptiveColor,
) -> Painter {
    let (graph, at) = (&laid.0, placed_at(laid, &dragged.get()));
    let (theme, dark) = (theme(), dark());
    let color = |c: AdaptiveColor| -> Hsla {
        match dark {
            true => c.dark,
            false => c.light,
        }
    };
    let mut p = Painter::new()
        .translate(shift.0, shift.1)
        .line_width(1.0)
        .stroke_color(color(stroke))
        .begin_path();
    for &(a, b) in graph.edges.iter().filter(|&&(a, b)| edge(a, b)) {
        p = p.move_to(at[a].0, at[a].1).line_to(at[b].0, at[b].1);
    }
    p = p.stroke();
    // The nodes of each color in one path: notes, and notes not yet created.
    for (fill, exists) in [(theme.primary, true), (theme.muted_foreground, false)] {
        p = p.fill_color(color(fill)).begin_path();
        for (i, n) in graph.nodes.iter().enumerate() {
            if n.exists == exists && node(i) {
                let r = radius(n.degree);
                p = p
                    .move_to(at[i].0 + r, at[i].1)
                    .arc(at[i].0, at[i].1, r, 0.0, TAU);
            }
        }
        p = p.fill();
    }
    p = p
        .font("11px sans-serif")
        .text_align(CanvasTextAlign::Center)
        .fill_color(color(theme.foreground));
    for (i, n) in graph.nodes.iter().enumerate().filter(|(i, _)| node(*i)) {
        p = p.fill_text(&n.label, at[i].0, at[i].1 + radius(n.degree) + 12.0);
    }
    p
}
