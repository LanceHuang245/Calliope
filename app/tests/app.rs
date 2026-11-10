//! The app, started headless on the real engine (`cargo test`), each test
//! on a vault of its own.

use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use nsh_core::{
    file_dialog::FileRequest,
    platform::{Idiom, Lifecycle, Os},
    prelude::{px, size},
    time::Clock,
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{DialogAnswer, Headless, MemoryStorage};

/// A vault in a fresh temporary folder: a folder and a note with front
/// matter.
fn vault(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("calliope-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Journal")).unwrap();
    fs::write(
        root.join("Hello.md"),
        "---\ntags: [a]\n---\n# Hello world\n\nSome **bold** text.\n",
    )
    .unwrap();
    root
}

/// The phone layout `EngineConfig::new` gives: creating the vault, the home
/// page, the files in the menu, then a note.
#[test]
fn phone_opens_reads_and_creates_notes() {
    let root = vault("phone");
    // The phones keep their vaults in `CALLIOPE_SANDBOX`; the only test that
    // reads it.
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", root.parent().unwrap()) };
    let mut app = Headless::start(EngineConfig::new(Root::of::<calliope::App>())).unwrap();

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

    // A note opens in Edit, closing the menu; Read renders it, its front
    // matter folded.
    app.press("Hello").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.wait_for_gone("Journal", Duration::from_secs(2))
        .unwrap();
    app.press("Read").unwrap();
    app.wait_for("Hello world").unwrap();
    app.wait_for("Properties").unwrap();

    // Back to the home page (the system's back), a new note is made on disk.
    app.back().unwrap();
    app.press("text:New note").unwrap();
    app.wait_for("label:Title").unwrap();
    assert!(root.join("Untitled.md").exists());

    // Another app's change shows when the app comes back to the front.
    fs::write(root.join("Untitled.md"), "# Changed elsewhere\n").unwrap();
    app.set_environment("background", |e| e.lifecycle = Lifecycle::Background)
        .unwrap();
    app.set_environment("foreground", |e| e.lifecycle = Lifecycle::Active)
        .unwrap();
    app.wait_for("Changed elsewhere").unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// The desktop layout, on the last vault opened: autosave, and another
/// app's change to a note with unsaved changes.
#[test]
fn desktop_autosaves_and_keeps_both_sides_of_a_conflict() {
    let root = vault("desktop");
    let mut config = EngineConfig::new(Root::of::<calliope::App>());
    config.environment.idiom = Idiom::Desktop;
    config.environment.os = Os::Linux;
    config.environment.file_requests = true;
    // Time moves only with the test's steps: no autosave can come between
    // typing and another app's write, on a fast machine or a slow one.
    config.clock = Clock::manual(SystemTime::now());
    config.viewport = size(px(1200.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        root.display().to_string(),
    )]));
    let mut app = Headless::start(config).unwrap();

    // The app draws its own title bar where the platform reports one.
    app.wait_for("Hello").unwrap();
    assert!(!app.shows("Calliope"));
    app.set_environment("title bar", |e| e.title_bar.top = px(32.0))
        .unwrap();
    app.wait_for("text:Calliope").unwrap();

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

    // A note's context menu deletes it, once confirmed.
    app.on_dialog(|_| DialogAnswer::Press("Delete".into()));
    app.right_click("text:Greeting").unwrap();
    app.press("text:Delete").unwrap();
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
    let mut config = EngineConfig::new(Root::of::<calliope::App>());
    config.environment.idiom = Idiom::Desktop;
    config.environment.os = Os::Linux;
    config.viewport = size(px(1200.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        root.display().to_string(),
    )]));
    let mut app = Headless::start(config).unwrap();

    // The note linked to lists the link's note and line; a press opens it.
    app.wait_for("Bee").unwrap();
    app.press("text:Bee").unwrap();
    app.wait_for("Backlinks").unwrap();
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
    app.on_dialog(|_| DialogAnswer::Press("Create".into()));
    app.press("text:Journal/New idea").unwrap();
    app.wait_for_within(line, Duration::from_secs(1)).unwrap();
    assert!(root.join("Journal").join("New idea.md").exists());

    // The quick switcher opens the note found by its title, and creates
    // the note of a name no note has.
    app.press_key("mod+o").unwrap();
    app.type_text("label:Find or create a note", "bee").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("The bee").unwrap();
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
