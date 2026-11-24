//! The graph of the vault: a node for each note, an edge for each link,
//! drawn on a canvas. A drag moves it and a pinch zooms it, natively; on
//! the desktops a mouse's wheel (or Ctrl + wheel) zooms it around the
//! pointer and two fingers on a touchpad move it. A press on a node opens
//! its note, and a pointer over one highlights its neighbors.
//!
//! Every node is drawn once, for each layout: moving and zooming draw
//! nothing again, and a hovered node dims them and draws itself, its
//! neighbors and their edges on a small canvas over them.

use std::{cell::Cell, collections::HashSet, f32::consts::TAU, rc::Rc};

use calliope_core::graph::Graph;
use nsh_base::{button, theme};
use nsh_core::{
    GlassEffect,
    canvas::{CanvasNode, CanvasTextAlign},
    color::Hsla,
    element::AdaptiveColor,
    platform::Os,
    prelude::*,
};

use crate::{components::page, store, strings};

/// The room around the outermost nodes, for their labels.
const MARGIN: f32 = 80.0;
/// How near a node's middle a pointer counts as on it, beyond its radius.
const REACH: f32 = 4.0;
/// A mouse wheel's line, in points (as Windows counts it).
const LINE: f32 = 16.0;
/// How far a wheel zooms: twice as near (or far) for each this many points.
const ZOOM: f32 = 200.0;

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
    let options = view()
        .flex_row()
        .gap_2()
        .child(option(strings::ORPHANS, orphans))
        .child(option(strings::UNCREATED, missing));
    let diagram = dynamic(move || laid.get().map(|laid| untracked(|| diagram(laid))));
    page(
        strings::GRAPH,
        store::back_to_note,
        Some(options.into_element()),
        diagram,
    )
}

/// Each filter has its own selection, shown as a glass pill on iOS 26.
fn option(label: &'static str, on: Signal<bool>) -> impl IntoElement {
    let option = button()
        .px_2()
        .py_1()
        .accessibility_label(label)
        .selected(move || on.get())
        .on_press(move || on.update(|on| *on = !*on))
        .child(text(label).text_sm().text_color(move || theme().foreground));
    if os() == Os::Ios && os_version().0 >= 26 {
        option.rounded_full().style_when(
            move || on.get(),
            |s| {
                s.glass_effect(GlassEffect::Regular)
                    .glass_tint_color(theme().accent)
            },
        )
    } else {
        option
            .rounded(move || theme().radius)
            .hover(|s| s.bg(theme().accent))
            .style_when(move || on.get(), |s| s.bg(theme().accent))
    }
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
    // Where the graph is (x, y, scale), for the wheel: the values move on
    // the UI thread, and drags and pinches say where they left them.
    let placed = signal((0.0_f32, 0.0_f32, 1.0_f32));
    let place = move |at: (f32, f32, f32)| {
        x.set(at.0);
        y.set(at.1);
        scale.set(at.2);
        placed.set(at);
    };
    let node_at = {
        let laid = laid.clone();
        move |point: Point<Points>| {
            let (px, py) = (point.x.0 - half, point.y.0 - half);
            (laid.0.nodes.iter().zip(&laid.1))
                .position(|(node, at)| (at.0 - px).hypot(at.1 - py) <= radius(node.degree) + REACH)
        }
    };
    let (over, pressed, all, near) = (node_at.clone(), node_at, laid.clone(), laid.clone());
    view()
        .flex_1()
        .overflow_hidden()
        .pan_values(Some(&x), Some(&y))
        .pinch_value(&scale)
        // Once it has a size: the whole graph in view, in its middle.
        .on_layout(move |bounds: &Bounds<Points>| {
            if !fitted.replace(true) {
                let (w, h) = (bounds.size.width.0, bounds.size.height.0);
                let fit = (w.min(h) / (half * 2.0)).min(1.0);
                place((w / 2.0 - half, h / 2.0 - half, fit));
            }
        })
        .on_pan_end(move |e: &PanEvent| {
            if let (Some(x), Some(y)) = (e.value_x, e.value_y) {
                placed.update(|at| (at.0, at.1) = (x, y));
            }
        })
        .on_pinch_end(move |e: &GestureEvent| placed.update(|at| at.2 = e.value))
        .on_wheel(move |e: &WheelEvent| place(wheeled(placed.get_untracked(), half, e)))
        .child(
            view()
                .absolute()
                .w(px(half * 2.0))
                .h(px(half * 2.0))
                .bind_translate_x(&x)
                .bind_translate_y(&y)
                .bind_scale(&scale)
                .child(
                    canvas(move || every_node(&all, half))
                        .absolute()
                        .inset_0()
                        .opacity(move || match hovered.get() {
                            Some(_) => 0.25_f32,
                            None => 1.0,
                        }),
                )
                .child(dynamic(move || {
                    let laid = near.clone();
                    hovered
                        .get()
                        .map(|i| untracked(|| neighbors(laid, half, i)))
                }))
                // Only when the pointer goes to another node, or off one, and
                // not while a drag moves the graph (it lags behind the
                // pointer, which would seem to go back and forth).
                .on_mouse_move(move |e: &MouseEvent| {
                    let node = over(e.position);
                    if e.pressed_button.is_none() && hovered.get_untracked() != node {
                        hovered.set(node);
                    }
                })
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

/// Every node, edge and title, on a canvas of `half` × 2 points square
/// around the origin. Each node is an element of its own to screen readers
/// and tests.
fn every_node(laid: &Laid, half: f32) -> Painter {
    let edges = theme().muted_foreground.opacity(0.4);
    let mut p = paint(laid, (half, half), |_, _| true, |_| true, edges);
    for (node, at) in laid.0.nodes.iter().zip(&laid.1) {
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
fn neighbors(laid: Laid, half: f32, hovered: usize) -> impl IntoElement {
    let touches = move |a: usize, b: usize| a == hovered || b == hovered;
    let near: HashSet<usize> = (laid.0.edges.iter().filter(|&&(a, b)| touches(a, b)))
        .flat_map(|&(a, b)| [a, b])
        .chain([hovered])
        .collect();
    let at = &laid.1;
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

/// Where the wheel `e` puts the graph at `(x, y, s)` (its translation and
/// scale; `half` is half its canvas's side): a mouse's wheel, or Ctrl +
/// wheel (a touchpad's pinch on Windows), zooms around the pointer; two
/// fingers on a touchpad move it.
fn wheeled((x, y, s): (f32, f32, f32), half: f32, e: &WheelEvent) -> (f32, f32, f32) {
    let d = e.delta.points(px(LINE));
    if matches!(e.delta, WheelDelta::Points(_)) && !e.modifiers.ctrl {
        return (x - d.x.0, y - d.y.0, s);
    }
    // The canvas scales around its middle: the point under the pointer
    // stays under it.
    let k = 2.0_f32.powf(-d.y.0 / ZOOM);
    let (qx, qy) = (e.position.x.0 - half, e.position.y.0 - half);
    (qx - k * (qx - x), qy - k * (qy - y), s * k)
}

/// A node's radius: larger with more links.
fn radius(degree: usize) -> f32 {
    3.0 + (degree as f32).sqrt() * 1.5
}

/// The edges `edge` keeps in `stroke`, then the nodes `node` keeps with
/// their titles, `shift` from the layout's origin.
fn paint(
    laid: &Laid,
    shift: (f32, f32),
    edge: impl Fn(usize, usize) -> bool,
    node: impl Fn(usize) -> bool,
    stroke: AdaptiveColor,
) -> Painter {
    let (graph, at) = (&laid.0, &laid.1);
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

#[cfg(test)]
mod tests {
    use nsh_core::{keymap::Modifiers, prelude::*};

    use super::wheeled;

    const HALF: f32 = 100.0;

    fn wheel(delta: WheelDelta, ctrl: bool) -> WheelEvent {
        let at = point(px(60.0), px(90.0));
        let modifiers = Modifiers {
            ctrl,
            ..Modifiers::default()
        };
        WheelEvent::new(at, at, delta, modifiers)
    }

    /// Where a point `c` of the canvas shows with the graph at `(x, y, s)`.
    fn shown((x, y, s): (f32, f32, f32), c: (f32, f32)) -> (f32, f32) {
        (x + HALF + s * (c.0 - HALF), y + HALF + s * (c.1 - HALF))
    }

    #[test]
    fn touchpads_move_the_graph_and_wheels_zoom_around_the_pointer() {
        let at = (10.0, 20.0, 0.5);
        let fingers = WheelDelta::Points(point(px(5.0), px(-8.0)));
        assert_eq!(wheeled(at, HALF, &wheel(fingers, false)), (5.0, 28.0, 0.5));

        // The canvas's point under the pointer, before.
        let under = (
            HALF + (60.0 - 10.0 - HALF) / 0.5,
            HALF + (90.0 - 20.0 - HALF) / 0.5,
        );
        let notches = WheelDelta::Lines(point(0.0, -3.0));
        let pinch = WheelDelta::Points(point(px(0.0), px(-48.0)));
        for e in [wheel(notches, false), wheel(pinch, true)] {
            let zoomed = wheeled(at, HALF, &e);
            assert!(zoomed.2 > 0.5, "up zooms in: {zoomed:?}");
            let (sx, sy) = shown(zoomed, under);
            assert!((sx - 60.0).abs() < 1e-3 && (sy - 90.0).abs() < 1e-3);
        }
        let down = wheeled(at, HALF, &wheel(WheelDelta::Lines(point(0.0, 3.0)), false));
        assert!(down.2 < 0.5, "down zooms out: {down:?}");
    }
}
