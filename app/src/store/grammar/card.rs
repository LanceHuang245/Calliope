//! The card beside a word with a grammar issue: on the desktops it opens
//! once the pointer rested on the word for `OPEN_DELAY`, stays while the
//! pointer is on it, and goes once the pointer left both, as a browser
//! plugin's hover card; on the phones a tap on the word opens it at once.
//! The word under the pointer or the finger is the trigger: the editor
//! reports the character it rests on or the word it was pressed on, and
//! only the issues marked there show.

use std::time::Duration;

use nsh_base::{
    Editor, EditorApi,
    editor::{EditorDiagnostic, EditorHoverEvent, EditorRange, EditorTextPressEvent},
    theme as tokens,
};
// The text elements, which the store's own `text` (`super::*`) hides.
use nsh_core::element::{overlay, text, view, when};

use crate::components::action;

use super::*;

/// How long the pointer rests on a marked word before its card opens (the
/// editor reports the rest 150 ms in, so the card comes about 0.65 s in).
const OPEN_DELAY: Duration = Duration::from_millis(500);
/// How long the card stays once the pointer left the word: the time to
/// move from the word onto the card.
const CLOSE_DELAY: Duration = Duration::from_millis(300);

/// What the card shows: the word's box in the root view, and the issues at
/// it with the report their fixes are for.
pub(super) struct Card {
    bounds: Bounds<Points>,
    report: Rc<Report>,
    issues: Vec<usize>,
}

/// The pointer on a marked word, and the card it opened.
#[derive(Clone, Default)]
struct Hover {
    /// Counts the pointer's moves: a timer of an older one does nothing.
    moves: u64,
    card: Option<Rc<Card>>,
    /// The pointer is on the card, which keeps it open.
    over: bool,
}

fn shown() -> Option<Rc<Card>> {
    get::<Hover>().card
}

/// The card that opens and goes, with where the pointer is; every timer
/// from before this move does nothing, and it reports the move's number.
fn moved(card: Option<Rc<Card>>, over: bool) -> u64 {
    global::<Hover>().update(|hover| {
        hover.moves += 1;
        hover.card = card;
        hover.over = over;
    });
    global::<Hover>().with_untracked(|hover| hover.moves)
}

/// Whether the pointer is still where it was `moves` moves ago, and not on
/// the card.
fn keeps(moves: u64) -> bool {
    global::<Hover>().with_untracked(|hover| hover.moves == moves && !hover.over)
}

/// The card's issues: the report's ones marked like the editor's
/// diagnostics at the character under the pointer.
fn indices(report: &Report, diagnostics: &[EditorDiagnostic]) -> Vec<usize> {
    report
        .issues
        .iter()
        .enumerate()
        .filter(|(_, issue)| {
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.range == issue.mark.range)
        })
        .map(|(index, _)| index)
        .collect()
}

/// The card's issues for a finger's word: the report's ones marked within
/// `range`, which is the whole word the finger went down on (a mark can be
/// shorter, or run over two words).
fn touched(report: &Report, range: EditorRange) -> Vec<usize> {
    report
        .issues
        .iter()
        .enumerate()
        .filter(|(_, issue)| {
            let mark = issue.mark.range;
            mark.start <= range.end && range.start <= mark.end
        })
        .map(|(index, _)| index)
        .collect()
}

/// The pointer rests on `event`'s character (`onHover`): the card of the
/// issues there opens once the pointer rested long enough on them, or the
/// last word's card goes.
pub(super) fn hovered(event: &EditorHoverEvent) {
    let here = report().map(|report| {
        let issues = indices(&report, &event.diagnostics);
        (report, issues)
    });
    let bounds = event.page_bounds;
    let moves = moved(None, false);
    let Some((report, issues)) = here.filter(|(_, issues)| !issues.is_empty()) else {
        return;
    };
    spawn_task(async move {
        timer(OPEN_DELAY).await;
        if !keeps(moves) {
            return;
        }
        moved(
            Some(Rc::new(Card {
                bounds,
                report,
                issues,
            })),
            false,
        );
    });
}

/// A finger pressed `event`'s word (`onTextPress`): the card of the issues
/// there opens at once, or the last card goes. The desktops keep the
/// pointer's hover.
pub(super) fn pressed(editor: ComponentRef<Editor>, event: &EditorTextPressEvent) {
    if is_desktop() {
        return;
    }
    let Some(report) = report() else { return };
    let issues = touched(&report, event.range);
    let bounds = editor.api::<EditorApi>().and_then(|api| {
        api.range_bounds(event.range)
            .first()
            .map(|it| it.page_bounds)
    });
    let Some(bounds) = bounds.filter(|_| !issues.is_empty()) else {
        // Another word, or none: the card the tap replaced goes.
        left();
        return;
    };
    moved(
        Some(Rc::new(Card {
            bounds,
            report,
            issues,
        })),
        false,
    );
}

/// The pointer left the word (`onHoverEnd`): the card stays while the
/// pointer may be on its way to it.
pub(super) fn hover_ended() {
    let hover = untracked(get::<Hover>);
    let moves = moved(hover.card.clone(), hover.over);
    if hover.card.is_none() {
        return;
    }
    spawn_task(async move {
        timer(CLOSE_DELAY).await;
        if keeps(moves) {
            moved(None, false);
        }
    });
}

/// The pointer came over the card: it stays open.
pub(super) fn entered() {
    moved(untracked(shown), true);
}

/// The pointer left the card: it goes.
pub(super) fn left() {
    moved(None, false);
}

/// Changes what the note shows: the next one counts from here.
pub(super) fn reset() {
    moved(None, false);
}

/// The card beside the word the pointer rests on or the finger pressed,
/// above everything else.
pub(crate) fn hover_card(editor: ComponentRef<Editor>) -> impl IntoElement {
    when(
        move || shown().is_some(),
        move || shown().map(|card| body(editor, card)),
    )
}

/// The card's box: placed under the word, it keeps open while the pointer
/// is on it, and goes when a press lands outside it or the system's back
/// asks (the phones' way out of it).
fn body(editor: ComponentRef<Editor>, card: Rc<Card>) -> impl IntoElement {
    let word = card.bounds;
    overlay()
        .anchor_bounds(move || word)
        .placement(Placement::Bottom)
        .align(Align::Start)
        .offset(px(6.0))
        .margin(px(8.0))
        .flex_col()
        .gap_1()
        .p_2()
        .min_w(px(180.0))
        .max_w(px(320.0))
        .rounded(move || tokens().radius)
        .border_1()
        .border_color(move || tokens().border)
        .bg(move || tokens().popover)
        .shadow_lg()
        // The pointer here left the text, which ends the hover: the card
        // stays until it leaves the card too.
        .on_mouse_enter(move |_: &MouseEvent| entered())
        .on_mouse_leave(move |_: &MouseEvent| left())
        // The finger that opened the card is on its word, and its lift is
        // reported here too: only a press anywhere else closes the card.
        .on_press_outside(move |at: &Point<Points>| {
            if !within(word, *at) {
                left();
            }
        })
        .on_cancel(left)
        .child(
            card.issues
                .iter()
                .map(|index| issue(editor, &card, *index))
                .collect::<Vec<_>>(),
        )
}

/// Whether `point` is in `bounds`.
fn within(bounds: Bounds<Points>, point: Point<Points>) -> bool {
    let (x, y) = (bounds.origin.x.0, bounds.origin.y.0);
    x <= point.x.0
        && point.x.0 <= x + bounds.size.width.0
        && y <= point.y.0
        && point.y.0 <= y + bounds.size.height.0
}

/// One issue of the card: what it says, then the fixes of it.
fn issue(editor: ComponentRef<Editor>, card: &Card, index: usize) -> View {
    let issue = &card.report.issues[index];
    let mut row = view().flex_col().gap_1().child(
        text(issue.mark.message.clone())
            .text_xs()
            .text_color(move || tokens().muted_foreground),
    );
    for (label, edit) in &issue.fixes {
        let (report, edit) = (card.report.clone(), edit.clone());
        row = row.child(action(label.clone(), move || {
            apply(editor, report.clone(), edit.clone());
            // The edit replaces the marked text: the card is about a place
            // that no longer is.
            left();
        }));
    }
    row
}

#[cfg(test)]
mod tests {
    use nsh_base::editor::{EditorPosition, EditorRange};

    use super::*;

    fn mark(start: u32, end: u32, message: &str) -> EditorDiagnostic {
        let mut mark = EditorDiagnostic::default();
        mark.range = EditorRange::new(EditorPosition::new(0, start), EditorPosition::new(0, end));
        mark.message = message.to_owned().into();
        mark
    }

    fn report() -> Report {
        Report {
            version: Version {
                opened: 0,
                revision: 0,
            },
            text: SharedString::default(),
            issues: [mark(4, 7, "an"), mark(10, 14, "teh")]
                .into_iter()
                .map(|mark| Issue {
                    mark,
                    fixes: Vec::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn the_card_shows_the_issues_marked_under_the_pointer() {
        let report = report();
        assert_eq!(indices(&report, &[mark(10, 14, "teh")]), [1]);
        // Both at one character: both show.
        assert_eq!(
            indices(&report, &[mark(4, 7, "an"), mark(10, 14, "teh")]),
            [0, 1]
        );
        assert!(indices(&report, &[mark(20, 24, "elsewhere")]).is_empty());
        assert!(indices(&report, &[]).is_empty());
    }

    /// A word, as the editor reports the one a finger pressed.
    fn word(start: u32, end: u32) -> EditorRange {
        EditorRange::new(EditorPosition::new(0, start), EditorPosition::new(0, end))
    }

    #[test]
    fn a_tap_shows_the_issues_marked_in_its_word() {
        let report = report();
        assert_eq!(touched(&report, word(4, 7)), [0]);
        // A mark running over two words is in either of them.
        assert_eq!(touched(&report, word(2, 5)), [0]);
        assert_eq!(touched(&report, word(3, 14)), [0, 1]);
        assert!(touched(&report, word(20, 24)).is_empty());
    }
}
