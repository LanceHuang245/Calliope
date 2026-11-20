//! The vault's trash, started headless on the real engine as `app.rs`'s
//! tests are.

mod common;

use std::{fs, time::Duration};

use common::{desktop, vault};
use nsh_testing::Headless;

/// The vault's trash, where sync puts what it deletes, opened from the
/// sidebar as the graph is: each file listed with where it was, restored
/// there, deleted for good once confirmed, then all of it at once.
#[test]
fn desktop_restores_and_empties_the_trash() {
    let root = vault("trash");
    // What sync deleted today, at the paths the files had.
    let day = root.join(format!(".trash/{}", jiff::ZonedDateTime::now().date()));
    for (path, text) in [
        ("Journal/Day.md", "day"),
        ("Old.md", "old"),
        ("Older.md", "older"),
    ] {
        fs::create_dir_all(day.join(path).parent().unwrap()).unwrap();
        fs::write(day.join(path), text).unwrap();
    }
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();
    app.press("label:Trash").unwrap();
    assert!(app.screen().selected("label:Trash").unwrap());
    // What the user deletes goes to the system's trash, not here.
    app.wait_for("text:/^What sync deletes stays here for 15 days\\. Notes you delete/")
        .unwrap();
    app.wait_for("text:/^From Journal\\/Day\\.md, deleted /")
        .unwrap();

    // Restored into its folder (the list goes by path: Journal/Day.md first).
    app.press("text:Restore >> nth=0").unwrap();
    app.wait_for("Restored to Journal/Day.md.").unwrap();
    assert_eq!(
        fs::read_to_string(root.join("Journal/Day.md")).unwrap(),
        "day"
    );
    app.wait_for_gone("text:Day.md", Duration::from_secs(2))
        .unwrap();

    // Deleted for good once confirmed: Old.md, then the rest.
    app.press("text:Delete >> nth=0").unwrap();
    app.press("role:dialog >> text:Delete").unwrap();
    app.wait_for_gone("text:Old.md", Duration::from_secs(2))
        .unwrap();
    assert!(!day.join("Old.md").exists());
    app.press("text:Empty trash").unwrap();
    app.press("role:dialog >> text:Delete").unwrap();
    app.wait_for("The trash is empty.").unwrap();
    assert!(!day.exists());
    // Back to the note, as from the graph, once the notice over the title
    // has gone.
    app.wait_for_gone("Restored to Journal/Day.md.", Duration::from_secs(5))
        .unwrap();
    app.press("label:Back").unwrap();
    app.wait_for_gone("The trash is empty.", Duration::from_secs(2))
        .unwrap();
    assert!(!app.screen().selected("label:Trash").unwrap());
    let _ = fs::remove_dir_all(&root);
}
