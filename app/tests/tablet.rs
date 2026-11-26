//! A tablet (MVP phase 4), started headless on the real engine: the
//! desktops' layout from 900 points wide, the phones' below, switching as
//! it turns. A process of its own, as it sets the phones' sandbox.

use std::{fs, time::Duration};

use nsh_core::{
    platform::{Idiom, Os},
    prelude::{px, size},
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, MemoryStorage};

/// An Android tablet on a vault of one note: in landscape the sidebar's
/// files and Split; turned to portrait the menu, without Split (the note
/// edited); turned back, Split again.
#[test]
fn tablet_shows_the_desktop_layout_when_wide_enough() {
    let sandbox = std::env::temp_dir().join(format!("calliope-tablet-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("Notes")).unwrap();
    fs::write(sandbox.join("Notes/Hello.md"), "# Hello world\n").unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let mut config =
        EngineConfig::new(Root::of::<calliope::App>().translations(calliope::translations()));
    config.environment.os = Os::Android;
    config.environment.idiom = Idiom::Pad;
    config.viewport = size(px(1100.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        "Notes".to_owned(),
    )]));
    let mut app = Headless::start(config).unwrap();

    // Landscape: the files in the sidebar, no menu and no floating toolbar
    // (the graph's button is the sidebar's only); the note offers Split.
    app.wait_for("Hello").unwrap();
    assert!(!app.shows("label:Menu"));
    assert_eq!(app.screen().count("label:Graph").unwrap(), 1);
    app.press("Hello").unwrap();
    app.press("Split").unwrap();
    app.wait_for("Hello world").unwrap();

    // Portrait, narrower: the phones' layout, the files in the menu; Split
    // gone, the note edited.
    app.set_viewport(size(px(800.0), px(1100.0))).unwrap();
    app.wait_for("label:Menu").unwrap();
    assert!(!app.shows("Split"));
    app.wait_for_gone("Hello world", Duration::from_secs(2))
        .unwrap();

    app.set_viewport(size(px(1100.0), px(800.0))).unwrap();
    app.wait_for("Split").unwrap();
    assert!(!app.shows("label:Menu"));
    let _ = fs::remove_dir_all(&sandbox);
}
