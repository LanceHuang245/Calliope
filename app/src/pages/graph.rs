//! The graph of the vault: a node for each note, an edge for each link,
//! drawn on a canvas. A drag moves it and a pinch (Ctrl + wheel on the
//! desktops) zooms it, natively; a press on a node opens its note, and a
//! pointer over one highlights its neighbors.
//!
//! Every node is drawn again when the hovered one changes; moving and
//! zooming draw nothing again.

use std::{cell::Cell, collections::HashSet, f32::consts::TAU, rc::Rc};

use calliope_core::graph::Graph;
use nsh_base::{button, theme};
use nsh_core::{
    canvas::{CanvasNode, CanvasTextAlign},
    color::Hsla,
    element::AdaptiveColor,
    prelude::*,
};

use super::home::menu_button;
use crate::{store, strings};

/// The room around the outermost nodes, for their labels.
const MARGIN: f32 = 80.0;
/// How near a node's middle a pointer counts as on it, beyond its radius.
const REACH: f32 = 4.0;

/// A graph and where the layout put its nodes.
type Laid = Rc<(Graph, Vec<(f32, f32)>)>;

pub(crate) fn graph_page() -> impl IntoElement {
    let orphans = signal(false);
    let missing = signal(false);
    // Laid out off the UI thread, again when the links or the options change.
    let laid = signal(None::<Laid>);
    effect(move || {
        let graph = store::graph(orphans.get(), missing.get());
        spawn(async move {
            let done = blocking(move || {
                let at = graph.layout();
                (graph, at)
            });
            if let Ok(done) = done.await {
                laid.set(Some(Rc::new(done)));
            }
        });
    });
    view()
        .flex_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .px_4()
                .py_2()
                .child((!is_desktop()).then(menu_button))
                .child(
                    text(strings::GRAPH)
                        .flex_1()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(option(strings::ORPHANS, orphans))
                .child(option(strings::UNCREATED, missing)),
        )
        .child(dynamic(move || {
            laid.get().map(|laid| untracked(|| diagram(laid)))
        }))
}

/// An option of the graph: a button, filled while it is on.
fn option(label: &'static str, on: Signal<bool>) -> impl IntoElement {
    button()
        .px_2()
        .py_1()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .style_when(move || on.get(), |s| s.bg(theme().accent))
        .on_press(move || on.update(|on| *on = !*on))
        .child(text(label).text_sm().text_color(move || theme().foreground))
}

fn diagram(laid: Laid) -> impl IntoElement {
    // The canvas: a square around the origin that holds every node.
    let reach = |m: f32, p: &(f32, f32)| m.max(p.0.abs()).max(p.1.abs());
    let half = laid.1.iter().fold(0.0, reach) + MARGIN;
    let (x, y, scale) = (
        animated_value(0.0),
        animated_value(0.0),
        animated_value(1.0),
    );
    let hovered = signal(None::<usize>);
    let fitted = Cell::new(false);
    let node_at = {
        let laid = laid.clone();
        move |point: Point<Points>| {
            let (px, py) = (point.x.0 - half, point.y.0 - half);
            (laid.0.nodes.iter().zip(&laid.1))
                .position(|(node, at)| (at.0 - px).hypot(at.1 - py) <= radius(node.degree) + REACH)
        }
    };
    let (over, pressed, painted) = (node_at.clone(), node_at, laid.clone());
    view()
        .flex_1()
        .overflow_hidden()
        .pan_values(Some(&x), Some(&y))
        .pinch_value(&scale)
        // Once it has a size: the whole graph in view, in its middle.
        .on_layout(move |bounds: &Bounds<Points>| {
            if !fitted.replace(true) {
                let (w, h) = (bounds.size.width.0, bounds.size.height.0);
                x.set(w / 2.0 - half);
                y.set(h / 2.0 - half);
                scale.set((w.min(h) / (half * 2.0)).min(1.0));
            }
        })
        .child(
            canvas(move || paint(&painted, half, hovered.get()))
                .absolute()
                .w(px(half * 2.0))
                .h(px(half * 2.0))
                .bind_translate_x(&x)
                .bind_translate_y(&y)
                .bind_scale(&scale)
                .on_mouse_move(move |e: &MouseEvent| hovered.set(over(e.position)))
                .on_mouse_leave(move |_: &MouseEvent| hovered.set(None))
                .on_press(move |e: &PressEvent| {
                    if let Some(node) = pressed(e.position).map(|i| &laid.0.nodes[i]) {
                        match node.exists {
                            true => store::select(&node.id.clone().into()),
                            false => store::open_link(node.id.clone()),
                        }
                    }
                }),
        )
}

/// A node's radius: larger with more links.
fn radius(degree: usize) -> f32 {
    3.0 + (degree as f32).sqrt() * 1.5
}

/// The drawing: the edges, then the nodes with their titles. With a node
/// `hovered`, it, its edges and its neighbors stand out from the others.
fn paint(laid: &Laid, half: f32, hovered: Option<usize>) -> Painter {
    let (graph, at) = (&laid.0, &laid.1);
    let (theme, dark) = (theme(), dark());
    let color = |c: AdaptiveColor| -> Hsla {
        match dark {
            true => c.dark,
            false => c.light,
        }
    };
    let touches = |&&(a, b): &&(usize, usize)| hovered == Some(a) || hovered == Some(b);
    let near: HashSet<usize> = (graph.edges.iter().filter(touches))
        .flat_map(|&(a, b)| [a, b])
        .chain(hovered)
        .collect();
    let lit = |i: usize| hovered.is_none() || near.contains(&i);

    let mut p = Painter::new().translate(half, half).line_width(1.0);
    // The edges, a hovered node's over the others.
    let quiet = theme.muted_foreground.opacity(0.4);
    for (stand_out, stroke) in [(false, quiet), (true, theme.primary)] {
        p = p.stroke_color(color(stroke)).begin_path();
        for &(a, b) in graph.edges.iter().filter(|e| touches(e) == stand_out) {
            p = p.move_to(at[a].0, at[a].1).line_to(at[b].0, at[b].1);
        }
        p = p.stroke();
    }
    // The nodes of each color in one path: notes, notes not yet created,
    // and those away from the hovered node.
    let fills = [
        (theme.primary, true, true),
        (theme.muted_foreground, false, true),
        (theme.border, true, false),
        (theme.border, false, false),
    ];
    for (fill, exists, is_lit) in fills {
        p = p.fill_color(color(fill)).begin_path();
        for (i, node) in graph.nodes.iter().enumerate() {
            if node.exists == exists && lit(i) == is_lit {
                let r = radius(node.degree);
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
    for (i, node) in graph.nodes.iter().enumerate().filter(|(i, _)| lit(*i)) {
        let below = radius(node.degree) + 12.0;
        p = p.fill_text(&node.label, at[i].0, at[i].1 + below);
    }
    // Each node is an element of its own to screen readers and tests.
    for (node, at) in graph.nodes.iter().zip(at) {
        let r = radius(node.degree);
        let corner = point(px(half + at.0 - r), px(half + at.1 - r));
        let area = Bounds::new(corner, size(px(r * 2.0), px(r * 2.0)));
        let button = CanvasNode::new(area, AccessibilityRole::Button, node.label.as_str());
        p = p.node(button.test_id(node.id.as_str()));
    }
    p
}
