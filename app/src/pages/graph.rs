//! The graph of the vault: a node for each note, an edge for each link,
//! drawn on a canvas (`paint`). A drag moves it and a pinch zooms it,
//! natively; on the desktops a mouse's wheel (or Ctrl + wheel) zooms it
//! around the pointer and two fingers on a touchpad move it, a drag that
//! starts on a node moves that node (kept on this device, in the store) and
//! a typed name keeps the nodes it matches (`filter`). A press on a node
//! opens its note, and a pointer over one highlights its neighbors.

use std::{cell::Cell, rc::Rc};

use nsh_core::{TouchEvent, platform::Os, prelude::*};

use crate::i18n::t;

use crate::{components::page, store};

mod filter;
mod paint;

use self::{
    filter::{mobile_filters, name_filter, named, option},
    paint::{Dragged, Laid, MARGIN, every_node, neighbors, placed_at, radius},
};

/// How near a node's middle a pointer counts as on it, beyond its radius.
const REACH: f32 = 4.0;
/// A mouse wheel's line, in points (as Windows counts it).
const LINE: f32 = 16.0;
/// How far a wheel zooms: twice as near (or far) for each this many points.
const ZOOM: f32 = 200.0;

pub(crate) fn graph_page() -> impl IntoElement {
    let mobile = matches!(os(), Os::Ios | Os::Android);
    let orphans = signal(mobile);
    let missing = signal(mobile);
    let typed = signal(String::new());
    let dragged: Dragged = signal(store::node_positions());
    // Laid out off the UI thread, again when the links or the filters change.
    let laid = signal(None::<Laid>);
    effect(move || {
        let graph = named(store::graph(orphans.get(), missing.get()), &typed.get());
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
    let diagram = dynamic(move || laid.get().map(|laid| untracked(|| diagram(laid, dragged))));
    if !mobile {
        let options = view()
            .flex_row()
            .gap_2()
            .child(name_filter(typed, false))
            .child(option(t("orphans"), orphans))
            .child(option(t("uncreated"), missing));
        return page(
            t("graph"),
            store::back_to_note,
            Some(options.into_element()),
            diagram,
        );
    }
    // The phones' header keeps the filters in its menu; the field that
    // names the nodes goes under it, over the graph.
    let filters = mobile_filters(orphans, missing).into_element();
    let field = view()
        .flex_row()
        .px_3()
        .py_2()
        .child(name_filter(typed, true));
    page(
        t("graph"),
        store::back_to_note,
        Some(filters),
        view().flex_1().child(field).child(diagram),
    )
}

fn diagram(laid: Laid, dragged: Dragged) -> impl IntoElement {
    // The canvas: a square around the origin that holds every node, with
    // room for its labels, and at least the window's short side, so that a
    // node can be dragged anywhere in view.
    let reach = |m: f32, p: &(f32, f32)| m.max(p.0.abs()).max(p.1.abs());
    let room = laid.1.iter().chain(dragged.get().values()).fold(0.0, reach) + MARGIN;
    let least = window().width.0.min(window().height.0) / 2.0;
    let half = room.max(least);
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
    // The node under a point of the canvas, if there is one.
    let node_at = {
        let laid = laid.clone();
        move |point: Point<Points>| {
            let (px, py) = (point.x.0 - half, point.y.0 - half);
            let at = placed_at(&laid, &dragged.get_untracked());
            (0..laid.0.nodes.len()).find(|&i| {
                (at[i].0 - px).hypot(at[i].1 - py) <= radius(laid.0.nodes[i].degree) + REACH
            })
        }
    };
    // The node a drag holds, where it was and where the finger landed; and
    // whether it moved, which is what the end keeps for the next graph.
    let held = signal(None::<(usize, (f32, f32), Point<Points>)>);
    let moved = signal(false);
    let end_drag = move || {
        if held.get_untracked().is_some() {
            held.set(None);
            if moved.get_untracked() {
                moved.set(false);
                store::save_node_positions(dragged.get_untracked());
            }
        }
    };
    let (over, pressed, all, near) = (node_at.clone(), node_at, laid.clone(), laid.clone());
    // A drag that starts on a node is the node's, not the graph's pan: the
    // press says which node, and the claim takes the touches as it moves.
    let claim = move |request: &ClaimRequest| {
        if held.get_untracked().is_some() {
            request.claim();
        }
    };
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
            // A pan a node's drag took the touches over from leaves the
            // graph where the pan found it: the finger moves the node.
            if held.get_untracked().is_some() {
                return place(placed.get_untracked());
            }
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
                    canvas(move || every_node(&all, half, dragged))
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
                        .map(|i| untracked(|| neighbors(laid, half, i, dragged)))
                }))
                // Hearing the touches (on the way to the claim) is what
                // puts the view on the responders' path: the claim below
                // is asked only of the views that do.
                .on_touch(|_: &TouchEvent| {})
                .claim_touches(TouchClaim::on_move().ask(claim))
                // The node follows the finger from where it held it; the
                // graph's own pan is not asked, the claim took the touches.
                .on_responder_move({
                    let laid = laid.clone();
                    move |e: &ResponderEvent| {
                        let Some((i, start, from)) = held.get_untracked() else {
                            return;
                        };
                        let scale = placed.get_untracked().2.max(0.05);
                        let (dx, dy) = (
                            (e.page.x.0 - from.x.0) / scale,
                            (e.page.y.0 - from.y.0) / scale,
                        );
                        let at = (
                            (start.0 + dx).clamp(-room, room),
                            (start.1 + dy).clamp(-room, room),
                        );
                        let id = laid.0.nodes[i].id.clone();
                        moved.set(true);
                        dragged.update(|dragged| {
                            dragged.insert(id, at);
                        });
                    }
                })
                .on_responder_release(move |_: &ResponderEvent| end_drag())
                .on_responder_terminate(move |_: &ResponderEvent| end_drag())
                // Only when the pointer goes to another node, or off one, and
                // not while a drag moves the graph (it lags behind the
                // pointer, which would seem to go back and forth).
                .on_mouse_move({
                    let over = over.clone();
                    move |e: &MouseEvent| {
                        let node = over(e.position);
                        if e.pressed_button.is_none() && hovered.get_untracked() != node {
                            hovered.set(node);
                        }
                    }
                })
                // The press says which node a drag would move, and from
                // where: its place now, and the point the finger landed on.
                .on_press_down({
                    let laid = laid.clone();
                    move |e: &PressEvent| {
                        let node = over(e.position)
                            .map(|i| (i, placed_at(&laid, &dragged.get_untracked())[i]));
                        held.set(node.map(|(i, at)| (i, at, e.page_position)));
                        moved.set(false);
                    }
                })
                .on_mouse_leave(move |_: &MouseEvent| hovered.set(None))
                .on_press(move |e: &PressEvent| {
                    end_drag();
                    if let Some(node) = pressed(e.position).map(|i| &laid.0.nodes[i]) {
                        match node.exists {
                            true => store::select(&node.id.clone().into()),
                            false => store::open_link(node.id.clone()),
                        }
                    }
                }),
        )
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
