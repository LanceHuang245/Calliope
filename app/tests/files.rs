//! The vault's files: what the tree offers for a file that is no note.

mod common;

use std::{fs, time::Duration};

use common::{desktop, vault};
use nsh_testing::Headless;

/// A file that is no note opens in the system's own program: the row's
/// menu says so. A note opens in Calliope, so its menu does not.
#[test]
fn desktop_offers_the_systems_program_for_a_file_that_is_no_note() {
    let root = vault("menu-external");
    fs::write(root.join("paper.pdf"), "%PDF-1.4\n").unwrap();
    fs::write(root.join("picture.png"), "not really a picture").unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();

    // The PDF and the picture: the system's program, and the same renaming
    // and deleting every row has.
    for file in ["paper.pdf", "picture.png"] {
        app.right_click(&format!("text:{file}")).unwrap();
        app.wait_for("text:Open with the default app").unwrap();
        assert!(app.screen().matches("text:Rename").unwrap(), "{file}");
        // Escape closes the menu (pressing the item would open the file in
        // this machine's own program).
        app.press_key("escape").unwrap();
        app.wait_for_gone("text:Open with the default app", Duration::from_secs(2))
            .unwrap();
    }

    // A note has no other program: it opens here.
    app.right_click("text:Hello").unwrap();
    app.wait_for("text:Rename").unwrap();
    assert!(!app.shows("Open with the default app"));
    app.press_key("escape").unwrap();
    let _ = fs::remove_dir_all(&root);
}
