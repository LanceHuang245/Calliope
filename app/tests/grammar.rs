//! Grammar checking: the desktops' status bar, its `mod+.` menu and the
//! card the pointer rests on, and the phones' tap on a marked word.

#![cfg(any(target_os = "windows", target_os = "android", target_os = "ios"))]

mod common;

use std::{fs, time::Duration};

use common::{desktop, vault};
use nsh_core::{
    platform::{Idiom, Os},
    prelude::{Point, px, size},
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, MemoryStorage};

const SOURCE: &str = "label:Markdown source";
/// The note's one row of text: `an` is marked, `test` is not.
const ROW: &str = "text:an test";
/// The card's fix of `an` (Harper's own label for its suggestion).
const FIX: &str = "text:/^Replace with/";
/// The phones' toolbar button that opens the commands (`mod+p` on the
/// desktops), and the command that turns the live check on and off.
const TOOLBAR_COMMANDS: &str = "label:Run a command";
const TOGGLE_GRAMMAR: &str = "text:Toggle live grammar checking";
const COUNT: &str = "text:/^[0-9]+ grammar issues?$/";
const CHECK: &str = "Check current note";

fn command(app: &mut Headless, name: &str) {
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", name).unwrap();
    app.press_key("enter").unwrap();
}

#[test]
fn checks_unsaved_text_with_sync_off_and_undoes_a_fix() {
    let root = vault("grammar-manual");
    let note = root.join("Hello.md");
    fs::write(&note, "").unwrap();
    let mut config = desktop(&root);
    config.environment.os = Os::Windows;
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Hello").unwrap();
    app.press("Hello").unwrap();
    app.wait_for(SOURCE).unwrap();
    app.type_text(SOURCE, "This is an test.").unwrap();
    assert!(!app.shows(COUNT));

    command(&mut app, CHECK);
    app.wait_for("text:1 grammar issue").unwrap();
    assert!(!app.shows("label:Sync now"));

    app.type_text(SOURCE, "").unwrap();
    app.press_key("mod+home").unwrap();
    for _ in 0..9 {
        app.press_key("right").unwrap();
    }
    app.press_key("mod+.").unwrap();
    app.choose_menu_item("Replace with: “a”").unwrap();
    app.wait(Duration::from_secs(1)).unwrap();
    assert_eq!(fs::read_to_string(&note).unwrap(), "This is a test.");

    app.press_key("mod+z").unwrap();
    app.wait(Duration::from_secs(1)).unwrap();
    assert_eq!(fs::read_to_string(&note).unwrap(), "This is an test.");

    app.press_key("mod+e").unwrap();
    app.wait_for_gone(COUNT, Duration::from_secs(2)).unwrap();
    app.press_key("mod+p").unwrap();
    assert!(!app.shows(CHECK));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn live_check_is_saved_and_replaces_the_count_on_note_switch() {
    let root = vault("grammar-live");
    fs::write(root.join("Hello.md"), "This is an test.").unwrap();
    fs::write(root.join("Clean.md"), "This is a test.").unwrap();
    fs::create_dir(root.join(".calliope")).unwrap();
    fs::write(
        root.join(".calliope/sync.json"),
        r#"{"provider":"webdav","auto":false}"#,
    )
    .unwrap();
    let stored = MemoryStorage::new([("vault".to_owned(), root.display().to_string())]);
    let mut config = desktop(&root);
    config.environment.os = Os::Windows;
    config.storage = Some(stored.clone());
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Hello").unwrap();

    app.press("label:Settings").unwrap();
    app.press("label:Editor").unwrap();
    app.press("label:Live grammar checking").unwrap();
    assert_eq!(stored.entries()["live_grammar"], "on");
    app.press("label:Back").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("text:1 grammar issue").unwrap();

    app.press("Clean").unwrap();
    app.wait_for_gone("text:1 grammar issue", Duration::from_secs(2))
        .unwrap();
    app.wait_for("text:0 grammar issues").unwrap();
    app.wait_for("text:Not synced yet").unwrap();
    app.type_text(SOURCE, "").unwrap();
    app.press_key("mod+a").unwrap();
    app.type_text(SOURCE, "This is an test.").unwrap();
    app.wait_for("text:1 grammar issue").unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// An Android phone on a vault of one note: a tap on the marked word opens
/// its card, a tap on a word without issues closes it, and the card's fix
/// applies, as `mod+.` does on the desktops.
#[test]
fn a_tap_opens_the_card_of_the_word_it_lands_on() {
    let sandbox =
        std::env::temp_dir().join(format!("calliope-grammar-phone-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("Notes")).unwrap();
    let note = sandbox.join("Notes/Hello.md");
    fs::write(&note, "an test\n").unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let mut config =
        EngineConfig::new(Root::of::<calliope::App>().translations(calliope::translations()));
    config.environment.os = Os::Android;
    config.environment.idiom = Idiom::Phone;
    config.viewport = size(px(400.0), px(800.0));
    let stored = MemoryStorage::new([("vault".to_owned(), "Notes".to_owned())]);
    config.storage = Some(stored.clone());
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Select a note, or create one.").unwrap();

    // The commands carry the check's setting as well: the phones' way to
    // it beside the settings' pages.
    app.press(TOOLBAR_COMMANDS).unwrap();
    app.press(TOGGLE_GRAMMAR).unwrap();
    assert_eq!(stored.entries()["live_grammar"], "on");
    app.press(TOOLBAR_COMMANDS).unwrap();
    app.press(TOGGLE_GRAMMAR).unwrap();
    assert_eq!(stored.entries()["live_grammar"], "off");

    app.press("label:Menu").unwrap();
    app.wait_for("Hello").unwrap();
    app.press("Hello").unwrap();
    app.wait_for(SOURCE).unwrap();

    // The check is off, so the phone asks for one in the commands the
    // toolbar opens (`Check current note`). The first check builds
    // Harper's dictionaries, and the card opens only once it is in: the tap
    // on the marked word is tried until it does.
    app.press(TOOLBAR_COMMANDS).unwrap();
    app.press("text:Check current note").unwrap();
    let inside = Point::new(px(4.0), px(6.0));
    let mut opened = false;
    for _ in 0..60 {
        app.press_at(ROW, inside).unwrap();
        app.wait(Duration::from_millis(100)).unwrap();
        if app.screen().matches(FIX).unwrap_or(false) {
            opened = true;
            break;
        }
    }
    assert!(opened, "the card never opened on the marked word");

    // A tap on the word the check left alone closes it.
    app.press_at(ROW, Point::new(px(40.0), px(6.0))).unwrap();
    app.wait_for_gone(FIX, Duration::from_secs(2)).unwrap();

    // The card's fix lands in the note, as the desktops' menu does.
    app.press_at(ROW, inside).unwrap();
    app.wait_for(FIX).unwrap();
    app.press(FIX).unwrap();
    app.wait(Duration::from_secs(1)).unwrap();
    assert_eq!(fs::read_to_string(&note).unwrap(), "a test\n");

    // Live checking, for the notes that come next, is the editor's
    // setting: off until it is turned on, in the phones' own way in (the
    // menu's gear, then the category).
    app.press("label:Menu").unwrap();
    app.press("label:Settings").unwrap();
    app.press("label:Editor").unwrap();
    app.press("label:Live grammar checking").unwrap();
    assert_eq!(stored.entries()["live_grammar"], "on");
    let _ = fs::remove_dir_all(&sandbox);
}
