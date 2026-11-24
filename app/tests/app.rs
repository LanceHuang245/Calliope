//! The app, started headless on the real engine (`cargo test`), each test
//! on a vault of its own.

mod common;

use std::{
    fs,
    time::{Duration, SystemTime},
};

use common::{desktop, vault};
use nsh_core::{
    file_dialog::FileRequest,
    platform::Lifecycle,
    prelude::{px, size},
    time::Clock,
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{DialogAnswer, Headless};

/// The phone layout `EngineConfig::new` gives (iOS's): creating the vault,
/// the home page, the files in the menu, renaming in the system's alert, a
/// note, then the pages the floating toolbar and the menu push.
#[test]
fn phone_opens_reads_and_creates_notes() {
    let root = vault("phone");
    // The phones keep their vaults in `CALLIOPE_SANDBOX`; the only test that
    // reads it.
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", root.parent().unwrap()) };
    let mut config = EngineConfig::new(Root::of::<calliope::App>());
    config.environment.os_version = (26, 0);
    let mut app = Headless::start(config).unwrap();

    // Without a vault the phone names one; here the vault made above.
    let name = root.file_name().unwrap().to_str().unwrap();
    app.wait_for("label:Vault name").unwrap();
    app.type_text("label:Vault name", name).unwrap();
    app.press("text:Create").unwrap();

    // The home page; the menu shows the files, folders first, notes by
    // their title.
    app.wait_for("Select a note, or create one.").unwrap();
    app.press("label:Menu").unwrap();
    app.wait_for("Journal").unwrap();
    app.wait_for("Hello").unwrap();

    // A held finger on a row offers renaming, in the system's alert with a
    // text field.
    app.on_dialog(|_| DialogAnswer::TypeAndPress {
        text: "Diary".into(),
        button: "Rename".into(),
    });
    app.long_press("text:Journal").unwrap();
    // The menu slides in from the bottom: on a slower machine (macOS) it
    // is still coming once the long press settles.
    app.wait_for("text:Rename").unwrap();
    app.press("text:Rename").unwrap();
    app.wait_for("Diary").unwrap();
    assert!(root.join("Diary").is_dir());

    // A note opens in Edit, closing the menu; Read renders it, its front
    // matter folded.
    app.press("Hello").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.wait_for_gone("Diary", Duration::from_secs(2)).unwrap();
    app.press("Read").unwrap();
    app.wait_for("Hello world").unwrap();
    app.wait_for("Properties").unwrap();
    // The backlinks show only when asked, saying so when there are none.
    assert!(!app.shows("No other note links here."));
    app.press("label:Backlinks").unwrap();
    app.wait_for("No other note links here.").unwrap();
    // The system's back closes their sheet.
    app.back().unwrap();
    app.wait_for_gone("No other note links here.", Duration::from_secs(2))
        .unwrap();

    // Back to the home page (the system's back), a new note is made on disk.
    app.back().unwrap();
    app.press("text:New note").unwrap();
    app.wait_for("label:Title").unwrap();
    assert!(root.join("Untitled.md").exists());
    // The separate new-note button beside the iOS toolbar makes another.
    app.press("label:New note").unwrap();
    app.wait(Duration::from_millis(500)).unwrap();
    assert!(root.join("Untitled 1.md").exists());

    // With the source edited and the keyboard up, the keyboard toolbar
    // starts the line as a task, then wraps the caret in code.
    app.press("Edit").unwrap();
    app.type_text("label:Markdown source", "Buy milk").unwrap();
    app.set_environment("keyboard up", |e| e.keyboard_height = px(300.0))
        .unwrap();
    app.press("label:Task").unwrap();
    app.press("label:Code").unwrap();
    app.wait(Duration::from_millis(1500)).unwrap();
    let typed = fs::read_to_string(root.join("Untitled 1.md")).unwrap();
    assert_eq!(typed, "- [ ] Buy milk``");
    app.set_environment("keyboard down", |e| e.keyboard_height = px(0.0))
        .unwrap();
    assert!(!app.shows("label:Task"));
    app.press("Read").unwrap();

    // Another app's change shows when the app comes back to the front.
    fs::write(root.join("Untitled 1.md"), "# Changed elsewhere\n").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.set_environment("foreground", |e| e.lifecycle = Lifecycle::Active)
        .unwrap();
    app.wait_for("Changed elsewhere").unwrap();

    assert!(!app.shows("label:Sync"));
    // The floating toolbar pushes the graph and the trash over the note,
    // the system's back returning to it; and opens the quick switcher.
    app.press("label:Graph").unwrap();
    app.wait_for("id:graph-filters").unwrap();
    assert!(!app.shows("text:Filters"));
    assert!(!app.shows("Orphans"));
    assert!(!app.shows("Uncreated notes"));
    app.back().unwrap();
    app.wait_for_gone("id:graph-filters", Duration::from_secs(2))
        .unwrap();
    app.press("label:Trash").unwrap();
    app.wait_for("The trash is empty.").unwrap();
    app.back().unwrap();
    app.wait_for_gone("The trash is empty.", Duration::from_secs(2))
        .unwrap();
    app.set_viewport(size(px(360.0), px(780.0))).unwrap();
    app.press("label:Search").unwrap();
    let search = "label:Find or create a note";
    app.wait_for(search).unwrap();
    let bounds = app.screen().layout(search).unwrap().unwrap().bounds;
    assert!(bounds.origin.x.0 >= 16.0);
    assert!(bounds.origin.x.0 + bounds.size.width.0 <= 344.0);
    app.back().unwrap();

    // The menu opens a category list, and Back pops one page at a time.
    app.press("label:Menu").unwrap();
    app.wait_for("label:Settings").unwrap();
    app.press("label:Settings").unwrap();
    app.wait_for("label:Appearance").unwrap();
    app.wait_for_gone("Diary", Duration::from_secs(2)).unwrap();
    assert!(!app.shows("System") && !app.shows("Save"));
    app.press("label:Sync").unwrap();
    app.wait_for("text:Save").unwrap();
    app.back().unwrap();
    app.wait_for_gone("text:Save", Duration::from_secs(2))
        .unwrap();
    app.press("label:Appearance").unwrap();
    app.wait_for("System").unwrap();
    app.back().unwrap();
    app.wait_for_gone("System", Duration::from_secs(2)).unwrap();
    app.wait_for("label:About").unwrap();
    app.press("label:About").unwrap();
    app.wait_for("Export logs…").unwrap();
    app.back().unwrap();
    app.wait_for_gone("Export logs…", Duration::from_secs(2))
        .unwrap();
    app.back().unwrap();
    app.wait_for_gone("label:Appearance", Duration::from_secs(2))
        .unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// The desktop layout, on the last vault opened: autosave, and another
/// app's change to a note with unsaved changes.
#[test]
fn desktop_autosaves_and_keeps_both_sides_of_a_conflict() {
    let root = vault("desktop");
    let mut config = desktop(&root);
    config.environment.file_requests = true;
    // Time moves only with the test's steps: no autosave can come between
    // typing and another app's write, on a fast machine or a slow one.
    config.clock = Clock::manual(SystemTime::now());
    let mut app = Headless::start(config).unwrap();

    // (Headless windows have no title bar: the app's own one, drawn where
    // the platform reports one, shows on the desktops only.)
    app.wait_for("Hello").unwrap();

    // Typing is saved within a second or so.
    app.press("Hello").unwrap();
    app.type_text("label:Markdown source", "X").unwrap();
    app.wait(Duration::from_millis(1500)).unwrap();
    let saved = fs::read_to_string(root.join("Hello.md")).unwrap();
    assert!(saved.contains('X'), "not saved: {saved:?}");

    // Another app writes while there are unsaved changes: a conflict,
    // neither side overwritten.
    app.type_text("label:Markdown source", "Y").unwrap();
    fs::write(root.join("Hello.md"), "# Theirs\n").unwrap();
    app.wait_for_within(
        "This note was changed by another app.",
        Duration::from_secs(5),
    )
    .unwrap();
    app.wait(Duration::from_millis(1500)).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("Hello.md")).unwrap(),
        "# Theirs\n"
    );

    // Keeping mine writes it.
    app.press("Keep mine").unwrap();
    let mine = fs::read_to_string(root.join("Hello.md")).unwrap();
    assert!(mine.contains('Y'), "not kept: {mine:?}");

    // The title renames the note in place.
    app.clear("label:Title").unwrap();
    app.type_text("label:Title", "Greeting").unwrap();
    app.submit("label:Title").unwrap();
    app.wait_for("Greeting").unwrap();
    assert!(root.join("Greeting.md").exists() && !root.join("Hello.md").exists());
    app.wait(Duration::from_millis(500)).unwrap();

    // A folder's context menu makes a note in it.
    app.right_click("text:Journal").unwrap();
    app.press("text:New note").unwrap();
    app.wait_for("Untitled").unwrap();
    assert!(root.join("Journal").join("Untitled.md").exists());

    // Renaming happens in a dialog.
    app.right_click("text:Journal").unwrap();
    app.press("text:Rename").unwrap();
    app.clear("label:Name").unwrap();
    app.type_text("label:Name", "Diary").unwrap();
    app.submit("label:Name").unwrap();
    app.wait_for("Diary").unwrap();
    // The dialog fades out before the next right click.
    app.wait(Duration::from_millis(500)).unwrap();
    assert!(root.join("Diary").join("Untitled.md").exists());

    // A note's context menu deletes it, once confirmed in Calliope's dialog.
    app.right_click("text:Greeting").unwrap();
    app.press("text:Delete").unwrap();
    // To the system's trash, as the dialog says.
    app.wait_for("text:/^It will be moved to the (Recycle Bin|system's Trash)\\.$/")
        .unwrap();
    app.press("role:dialog >> text:Delete").unwrap();
    // Headless records the request to the system's trash, and moves nothing.
    let trash = FileRequest::MoveToTrash(root.join("Greeting.md"));
    assert!(app.screen().file_requests().contains(&trash));
    let _ = fs::remove_dir_all(&root);
}

/// Links between notes: the backlinks of the open note, a wiki link
/// followed in the rendered note, and a note created from a link to none.
#[test]
fn desktop_follows_links_and_lists_backlinks() {
    let root = vault("links");
    fs::write(
        root.join("A.md"),
        "see [[Bee|my bee]] and [[Journal/New idea]]\n",
    )
    .unwrap();
    fs::write(root.join("Bee.md"), "# The bee\n").unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();

    // The note linked to lists the link's note and line; a press opens it.
    app.wait_for("Bee").unwrap();
    app.press("text:Bee").unwrap();
    app.wait_for("Backlinks").unwrap();
    // The backlinks button is on while the panel shows.
    assert!(app.screen().selected("label:Backlinks").unwrap());
    app.press("label:Backlinks").unwrap();
    app.wait_for_gone("text:Backlinks", Duration::from_secs(2))
        .unwrap();
    assert!(!app.screen().selected("label:Backlinks").unwrap());
    app.press("label:Backlinks").unwrap();
    app.wait_for("text:Backlinks").unwrap();
    let line = "see [[Bee|my bee]] and [[Journal/New idea]]";
    app.wait_for_within(line, Duration::from_secs(5)).unwrap();
    app.press(line).unwrap();
    app.wait_for("No other note links here.").unwrap();

    // Rendered, a link opens its note.
    app.press("Read").unwrap();
    app.press("text:my bee").unwrap();
    app.wait_for("The bee").unwrap();

    // A link to no note creates it, once confirmed, and the new note
    // has the link's note as a backlink within a second.
    app.press(line).unwrap();
    app.press("text:Journal/New idea").unwrap();
    app.press("role:dialog >> text:Create").unwrap();
    app.wait_for_within(line, Duration::from_secs(1)).unwrap();
    assert!(root.join("Journal").join("New idea.md").exists());

    // The quick switcher opens the note found by letters of its title, and
    // creates the note of a name no note has.
    app.press_key("mod+o").unwrap();
    app.type_text("label:Find or create a note", "hlo").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("Hello world").unwrap();
    app.press("label:Search").unwrap();
    app.type_text("label:Find or create a note", "Plan")
        .unwrap();
    app.press("Create the note “Plan”").unwrap();
    app.wait_for("label:Title").unwrap();
    assert!(root.join("Plan.md").exists());

    // Typing `[[` in the source offers the notes' titles.
    app.press("Edit").unwrap();
    app.type_text("label:Markdown source", "[[").unwrap();
    app.wait_for("New idea").unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// Mermaid code that is no diagram shows its reason and the code, and
/// other code blocks stay code.
#[test]
fn desktop_shows_why_a_diagram_cannot_be_drawn() {
    let root = vault("mermaid");
    fs::write(
        root.join("Chart.md"),
        "```mermaid\nnotADiagram here\n```\n\n```rust\nlet kept = 1;\n```\n",
    )
    .unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();

    app.wait_for("Chart").unwrap();
    app.press("text:Chart").unwrap();
    app.press("Read").unwrap();
    // The system's fonts are read first, once: slow in a debug build.
    app.wait_for_within("No Mermaid diagram type detected", Duration::from_secs(120))
        .unwrap();
    app.wait_for("notADiagram here").unwrap();
    app.wait_for("let kept = 1;").unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// The graph: linked notes as nodes, a press on one opens its note; the
/// options add the notes without links and those not yet created.
#[test]
fn desktop_draws_the_graph_and_opens_a_node() {
    let root = vault("graph");
    fs::write(root.join("A.md"), "[[Bee]] and [[Later]]\n").unwrap();
    fs::write(root.join("Bee.md"), "# The bee\n").unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();

    // Linked notes only, at first.
    app.wait_for("Bee").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("id:Bee.md").unwrap();
    assert!(!app.shows("id:Later") && !app.shows("id:Hello.md"));
    app.press("Orphans").unwrap();
    app.wait_for("id:Hello.md").unwrap();
    app.press("Uncreated notes").unwrap();
    app.wait_for("id:Later").unwrap();

    // A node opens its note.
    app.press("id:Bee.md").unwrap();
    app.press("Read").unwrap();
    app.wait_for("The bee").unwrap();

    // Clicking the already-open note in the sidebar while on Graph returns to it.
    app.press_key("mod+g").unwrap();
    app.wait_for("id:Bee.md").unwrap();
    assert!(app.screen().selected("label:Graph").unwrap());
    app.press("label:Files >> text:Bee").unwrap();
    app.wait_for("The bee").unwrap();
    assert!(!app.screen().selected("label:Graph").unwrap());

    // Graph's back button and toggle both return to the open note.
    app.press("label:Graph").unwrap();
    app.wait_for("label:Back").unwrap();
    app.press("label:Back").unwrap();
    app.wait_for("The bee").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("Orphans").unwrap();
    app.press_key("mod+g").unwrap();
    app.wait_for("The bee").unwrap();
    let _ = fs::remove_dir_all(&root);
}
