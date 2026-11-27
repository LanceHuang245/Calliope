//! Grammar checking on the Windows desktop layout.

#![cfg(target_os = "windows")]

mod common;

use std::{fs, time::Duration};

use common::{desktop, vault};
use nsh_core::platform::Os;
use nsh_testing::{Headless, MemoryStorage};

const SOURCE: &str = "label:Markdown source";
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
