//! Check only the active source buffer, on a single background job at a time.

mod card;
mod check;
mod ime;

use std::{
    rc::Rc,
    sync::Mutex,
    time::{Duration, Instant},
};

use harper_core::linting::LintGroup;
use nsh_base::{
    Editor, EditorApi, EditorElement,
    editor::{
        EditorCodeActionRequest, EditorDiagnostic, EditorEdit, EditorHoverEvent,
        EditorTextPressEvent,
    },
};
use nsh_core::element::desktop::{MenuItem, popup_menu_at};

use super::*;
pub(crate) use card::hover_card;
use card::reset;
use check::Issue;

const LIVE: &str = "live_grammar";
const PAUSE: Duration = Duration::from_millis(500);
const IME_POLL: Duration = Duration::from_millis(100);

#[derive(Clone, Default)]
struct Live(bool);
#[derive(Clone, Copy, Default)]
struct Revision(u64);

#[derive(Clone, Copy, PartialEq, Eq)]
struct Version {
    opened: u64,
    revision: u64,
}

#[derive(Clone, Copy)]
struct Request {
    version: Version,
    id: u64,
    deadline: Instant,
    manual: bool,
}

struct Report {
    version: Version,
    text: SharedString,
    issues: Vec<Issue>,
}

#[derive(Clone, Default)]
struct Work {
    next: Option<Request>,
    running: bool,
    id: u64,
}

#[derive(Clone, Default)]
struct Results(Option<Rc<Report>>);

pub(crate) fn available() -> bool {
    version().is_some()
}

fn version() -> Option<Version> {
    if !matches!(route(), Route::Home | Route::Note) || mode() == Mode::Read {
        return None;
    }
    Some(Version {
        opened: opened()?,
        revision: get::<Revision>().0,
    })
}

pub(super) fn changed() {
    global::<Revision>().update(|revision| revision.0 += 1);
}

pub(crate) fn live() -> bool {
    get::<Live>().0
}

pub(crate) fn set_live(value: &bool) {
    if Storage::set_item(LIVE, if *value { "on" } else { "off" }).is_err() {
        notify(t("grammar_setting_failed"));
        return;
    }
    set(Live(*value));
    changed();
}

/// Turns the live check on or off: the settings' switch, and the command
/// the palette lists.
pub(crate) fn toggle() {
    set_live(&!live());
}

pub(super) fn start() {
    set(Live(Storage::get_item(LIVE).as_deref() == Some("on")));
    let version = computed(version);
    effect(move || {
        let version = version.get();
        let live = live();
        untracked(|| {
            // Visits and text revisions invalidate both pending jobs and menus.
            global::<Work>().update(|work| {
                work.next = None;
                work.id += 1;
            });
            set(Results::default());
            reset();
            if let Some(version) = version.filter(|_| live) {
                enqueue(version, false);
            }
        });
    });
}

pub(super) fn check_current() {
    if let Some(version) = untracked(version) {
        enqueue(version, true);
    }
}

fn enqueue(version: Version, manual: bool) {
    let mut start = false;
    global::<Work>().update(|work| {
        work.id += 1;
        work.next = Some(Request {
            version,
            id: work.id,
            deadline: Instant::now() + if manual { Duration::ZERO } else { PAUSE },
            manual,
        });
        start = !work.running;
        work.running = true;
    });
    if start {
        spawn_task(run());
    }
}

fn wanted(request: Request) -> bool {
    version() == Some(request.version) && global::<Work>().with(|work| work.id == request.id)
}

async fn run() {
    // Retain Harper's initialized dictionary and rule caches between requests.
    static CHECKER: Mutex<Option<LintGroup>> = Mutex::new(None);
    loop {
        let Some(request) = global::<Work>().with(|work| work.next) else {
            global::<Work>().update(|work| work.running = false);
            break;
        };
        let remaining = request.deadline.saturating_duration_since(Instant::now());
        if !remaining.is_zero() {
            timer(remaining).await;
            continue;
        }
        global::<Work>().update(|work| work.next = None);
        if !wanted(request) || !after_composition(request).await {
            continue;
        }
        let source = text();
        let snapshot = source.clone();
        let checked = blocking(move || {
            let mut checker = CHECKER
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(check::checker);
            let issues = check::check(&mut checker, &snapshot);
            *CHECKER.lock().unwrap() = Some(checker);
            issues
        })
        .await;
        let Ok(issues) = checked else {
            if request.manual && wanted(request) {
                notify(t("grammar_check_failed"));
            }
            continue;
        };
        if wanted(request) && after_composition(request).await {
            set(Results(Some(Rc::new(Report {
                version: request.version,
                text: source,
                issues,
            }))));
        }
    }
}

async fn after_composition(request: Request) -> bool {
    loop {
        let Ok(composing) = run_on_main_thread(ime::composing).await else {
            return false;
        };
        if !wanted(request) {
            return false;
        }
        if !composing {
            return true;
        }
        timer(IME_POLL).await;
    }
}

fn report() -> Option<Rc<Report>> {
    let version = version()?;
    get::<Results>()
        .0
        .filter(|report| report.version == version)
}

pub(crate) fn count() -> Option<usize> {
    report().map(|report| report.issues.len())
}

/// `source` with what the issues show: their marks, the fixes `mod+.`
/// lists, and the card a pointer resting on a marked word opens, or a tap
/// on it on a touch screen.
pub(crate) fn hooks(source: EditorElement, editor: ComponentRef<Editor>) -> EditorElement {
    source
        .diagnostics(diagnostics)
        .on_code_action_request(move |request: &EditorCodeActionRequest| {
            show_fixes(editor, request)
        })
        .on_hover(move |event: &EditorHoverEvent| card::hovered(event))
        .on_hover_end(card::hover_ended)
        .on_text_press(move |event: &EditorTextPressEvent| card::pressed(editor, event))
}

pub(crate) fn diagnostics() -> Vec<EditorDiagnostic> {
    report()
        .map(|report| {
            report
                .issues
                .iter()
                .map(|issue| issue.mark.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn show_fixes(editor: ComponentRef<Editor>, request: &EditorCodeActionRequest) {
    let Some(report) = report() else { return };
    let Some(bounds) = request.bounds else { return };
    let mut items = Vec::new();
    for issue in &report.issues {
        let range = issue.mark.range;
        if request.range.start > range.end || request.range.end < range.start {
            continue;
        }
        if !items.is_empty() {
            items.push(MenuItem::separator());
        }
        items.push(MenuItem::new(issue.mark.message.to_string()).disabled(true));
        for (label, edit) in &issue.fixes {
            let report = report.clone();
            let edit = edit.clone();
            items.push(MenuItem::new(label).on_select(move || {
                apply(editor, report.clone(), edit.clone());
            }));
        }
    }
    if !items.is_empty() {
        drop(popup_menu_at(
            items,
            point(
                bounds.page_bounds.origin.x,
                bounds.page_bounds.origin.y + bounds.page_bounds.size.height,
            ),
        ));
    }
}

/// Applies `edit` as the one fix of `report`: it does nothing once the
/// report is another's or the editor holds other text (a menu or a card
/// can outlive both).
fn apply(editor: ComponentRef<Editor>, report: Rc<Report>, edit: EditorEdit) {
    spawn_task(async move {
        if run_on_main_thread(ime::composing).await.unwrap_or(true) {
            return;
        }
        if self::report().is_some_and(|current| Rc::ptr_eq(&current, &report))
            && let Some(api) = editor.api::<EditorApi>()
            && api.value() == report.text.as_str()
        {
            api.edit(vec![edit]);
            api.focus();
        }
    });
}
