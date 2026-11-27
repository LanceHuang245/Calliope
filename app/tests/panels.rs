//! The note's panels (MVP phase 6): its outline and its outlinks, beside the
//! note on the desktops, in a sheet on the phones.

mod common;

use std::{fs, time::Duration};

use common::{desktop, vault};
use nsh_core::{
    platform::{Idiom, Os},
    prelude::{px, size},
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::{Headless, MemoryStorage};

/// The desktops list the note's headings and the notes it links to, the ones
/// no note has yet too; the note's own buttons show each panel and the panel
/// closes with its button or its ×.
#[test]
fn desktop_lists_the_outline_and_the_outlinks() {
    let root = vault("panels");
    fs::write(
        root.join("Hello.md"),
        "# Greeting\n\nsome text\n## Notes\n\nsee [[Journal/New idea]] and [[Bee]]\n",
    )
    .unwrap();
    fs::write(root.join("Bee.md"), "# The bee\n").unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();

    // The backlinks show beside the note at first; the outline is only asked
    // for.
    app.wait_for("Hello").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.wait_for("id:panel-backlinks").unwrap();
    assert!(!app.shows("id:panel-outline"));

    // The command opens it, its headings in it.
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", "outline").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("id:panel-outline").unwrap();
    app.wait_for("id:panel-outline >> text:Greeting").unwrap();
    app.wait_for("id:panel-outline >> text:Notes").unwrap();

    // Its close button folds it away; the backlinks stay.
    app.press("id:panel-outline >> label:Close").unwrap();
    app.wait_for_gone("id:panel-outline", Duration::from_secs(2))
        .unwrap();
    app.wait_for("id:panel-backlinks").unwrap();

    // The outlinks name the note linked to, and the one no note has yet.
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", "outgoing").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("id:panel-outlinks >> text:Bee").unwrap();
    app.wait_for("id:panel-outlinks >> text:Journal/New idea")
        .unwrap();
    app.wait_for("id:panel-outlinks >> text:Not created yet")
        .unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// The phones show the panels in a sheet when their toolbar asks (at 430
/// points the bar holds its outline and outlinks buttons): the outline takes
/// the sheet away as it scrolls the note to a heading.
#[test]
fn phone_shows_the_panels_in_a_sheet() {
    let sandbox =
        std::env::temp_dir().join(format!("calliope-phone-panels-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("Notes")).unwrap();
    fs::write(
        sandbox.join("Notes/Hello.md"),
        "# Greeting\n\nsee [[Elsewhere]]\n",
    )
    .unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let mut config =
        EngineConfig::new(Root::of::<calliope::App>().translations(calliope::translations()));
    config.environment.os = Os::Android;
    config.environment.idiom = Idiom::Phone;
    config.viewport = size(px(430.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        "Notes".to_owned(),
    )]));
    let mut app = Headless::start(config).unwrap();

    // The note's page, whose toolbar holds the outline's button.
    app.wait_for("Select a note, or create one.").unwrap();
    app.press("label:Menu").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("label:Outline").unwrap();

    // The outline's sheet lists the headings; a press closes it, the note
    // scrolled to the heading.
    app.press("label:Outline").unwrap();
    app.wait_for("id:panels").unwrap();
    app.wait_for("id:panels >> text:Greeting").unwrap();
    app.press("id:panels >> text:Greeting").unwrap();
    app.wait_for_gone("id:panels", Duration::from_secs(2))
        .unwrap();
    app.wait_for("label:Markdown source").unwrap();

    // The outlinks list the note it names, no note having it yet; the
    // system's back closes the sheet.
    app.press("label:Outgoing links").unwrap();
    app.wait_for("id:panels >> text:Elsewhere").unwrap();
    app.wait_for("id:panels >> text:Not created yet").unwrap();
    app.back().unwrap();
    app.wait_for_gone("id:panels", Duration::from_secs(2))
        .unwrap();
    let _ = fs::remove_dir_all(&sandbox);
}
