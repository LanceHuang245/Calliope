//! The phones' floating toolbar: the buttons a narrow window has no room for
//! go into its filter menu, so an action added to the bar never widens it
//! past the screen. A process of its own, as it sets the phones' sandbox.

use std::fs;

use nsh_core::{
    platform::{Idiom, Os},
    prelude::{px, size},
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, MemoryStorage};

/// The buttons the bar has no room for, in the system's menu.
const FILTER_MENU: &str = "id:toolbar-filters";

/// An Android phone on a vault of one note, 400 points wide: the bar holds
/// every button, the note's backlinks included. Narrowed to 300 points, the
/// last two move into the filter menu and the ones left still work.
#[test]
fn a_narrow_phone_puts_the_buttons_that_do_not_fit_in_its_menu() {
    let sandbox = std::env::temp_dir().join(format!("calliope-toolbar-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("Notes")).unwrap();
    fs::write(sandbox.join("Notes/Hello.md"), "# Hello world\n").unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let mut config =
        EngineConfig::new(Root::of::<calliope::App>().translations(calliope::translations()));
    config.environment.os = Os::Android;
    config.environment.idiom = Idiom::Phone;
    config.viewport = size(px(400.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        "Notes".to_owned(),
    )]));
    let mut app = Headless::start(config).unwrap();

    // The note's page, whose toolbar adds the backlinks: five buttons, all
    // in the bar at this width.
    app.wait_for("Select a note, or create one.").unwrap();
    app.press("label:Menu").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("label:Backlinks").unwrap();
    assert!(!app.screen().matches(FILTER_MENU).unwrap());
    for button in ["Search", "Graph", "Backlinks", "Run a command", "Trash"] {
        let shown = app.screen().matches(&format!("label:{button}")).unwrap();
        assert!(shown, "{button} is off the bar");
    }

    // Narrowed, the last two leave the bar for the menu; the first three stay.
    app.set_viewport(size(px(300.0), px(800.0))).unwrap();
    app.wait_for(FILTER_MENU).unwrap();
    for button in ["Trash", "Run a command"] {
        let shown = app.screen().matches(&format!("label:{button}")).unwrap();
        assert!(!shown, "{button} is still on the bar");
    }
    for button in ["Search", "Graph", "Backlinks"] {
        let shown = app.screen().matches(&format!("label:{button}")).unwrap();
        assert!(shown, "{button} left the bar");
    }

    // What the bar kept still works.
    app.press("label:Graph").unwrap();
    app.wait_for("id:graph-filters").unwrap();
    app.back().unwrap();
    app.wait_for_gone("id:graph-filters", std::time::Duration::from_secs(2))
        .unwrap();

    // Widened again, the menu goes and every button is back.
    app.set_viewport(size(px(400.0), px(800.0))).unwrap();
    app.wait_for("label:Trash").unwrap();
    assert!(!app.screen().matches(FILTER_MENU).unwrap());
    let _ = fs::remove_dir_all(&sandbox);
}
