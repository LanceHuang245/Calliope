//! The command palette and the full-text search (MVP phase 6), headless on
//! the desktop layout.

mod common;

use std::{fs, sync::Arc, time::Duration};

use common::{desktop, vault};
use nsh_core::prelude::px;
use nsh_testing::{Headless, MemoryStorage};

/// The palette runs the app's commands by name: one that writes a file
/// writes it, one that hides the sidebar hides it.
#[test]
fn desktop_runs_commands_from_the_palette() {
    let root = vault("palette");
    // What the app keeps on the device: the theme's last command writes it.
    let stored = MemoryStorage::new([("vault".to_owned(), root.display().to_string())]);
    let mut config = desktop(&root);
    config.storage = Some(stored.clone() as Arc<dyn nsh_core::storage::StorageBackend>);
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Hello").unwrap();

    // The query narrows the commands to one: a copy of the open note.
    app.press("Hello").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.press_key("mod+p").unwrap();
    app.wait_for("label:Run a command").unwrap();
    app.type_text("label:Run a command", "copy").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("text:/^Your changes were saved to /").unwrap();
    let copies: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("Hello (copy "))
        .collect();
    assert_eq!(copies.len(), 1, "{copies:?}");

    // The sidebar hides, then comes back.
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", "sidebar").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for_gone("label:Settings", Duration::from_secs(2))
        .unwrap();
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", "sidebar").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("label:Settings").unwrap();

    // A page opens: the settings, on its first category.
    app.press_key("mod+p").unwrap();
    app.type_text("label:Run a command", "settings").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for("Theme").unwrap();
    app.wait_for_gone("label:Markdown source", Duration::from_secs(2))
        .unwrap();

    // Escape closes the palette without running anything.
    app.press_key("mod+p").unwrap();
    app.wait_for("label:Run a command").unwrap();
    app.press_key("escape").unwrap();
    app.wait_for_gone("label:Run a command", Duration::from_secs(2))
        .unwrap();
    assert!(app.shows("Theme"));

    // The rows scroll inside the panel: the keys walk past the ones it
    // shows and run the last command (the theme, kept on the device).
    app.press_key("mod+p").unwrap();
    app.wait_for("label:Run a command").unwrap();
    for _ in 0..13 {
        app.press_key("down").unwrap();
    }
    app.press_key("enter").unwrap();
    app.wait_for_gone("label:Run a command", Duration::from_secs(2))
        .unwrap();
    let theme = stored.entries().get("theme").cloned();
    assert_eq!(
        theme.as_deref(),
        Some("light"),
        "the last command did not run"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The files' command button opens the palette as `mod+p` does, without a
/// key.
#[test]
fn desktop_opens_the_palette_from_the_files_actions() {
    let root = vault("palette-button");
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();

    // The button stands beside the files' actions: it opens the palette,
    // where the query narrows the rows and Return runs the one chosen.
    app.press("label:Command palette").unwrap();
    app.wait_for("label:Run a command").unwrap();
    app.type_text("label:Run a command", "sidebar").unwrap();
    app.press_key("enter").unwrap();
    app.wait_for_gone("label:Settings", Duration::from_secs(2))
        .unwrap();
    let _ = fs::remove_dir_all(&root);
}

/// The results stay inside the page: a search with more hits than fit
/// scrolls under the field.
#[test]
fn desktop_scrolls_long_result_lists() {
    let root = vault("hits");
    for i in 0..201 {
        let note = format!("Note {i:02}.md");
        fs::write(
            root.join(note),
            format!("# Note {i}\n\nthe needle in {i}\n"),
        )
        .unwrap();
    }
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();

    app.press_key("mod+shift+f").unwrap();
    app.type_text("label:Search all notes", "needle").unwrap();
    let hit = "text:Note 00.md:3";
    app.wait_for(hit).unwrap();
    app.wait_for("Showing the first 200 results. Refine your search to see other matches.")
        .unwrap();
    let before = app.screen().layout(hit).unwrap().unwrap().bounds;
    app.scroll(hit, px(300.0)).unwrap();
    let after = app.screen().layout(hit).unwrap().unwrap().bounds;
    assert!(
        after.origin.y.0 < before.origin.y.0,
        "the results did not scroll: {before:?} -> {after:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The full-text search finds the lines of every note that hold a word,
/// and opens one at its line.
#[test]
fn desktop_searches_every_notes_text() {
    let root = vault("fulltext");
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();

    app.press_key("mod+shift+f").unwrap();
    app.wait_for("label:Search all notes").unwrap();
    app.wait_for("Type to search every note.").unwrap();

    // A word in a note's line: the line and where it is.
    app.type_text("label:Search all notes", "bold").unwrap();
    app.wait_for("Some **bold** text.").unwrap();
    app.wait_for("Hello.md:6").unwrap();

    // A word no note holds.
    app.clear("label:Search all notes").unwrap();
    app.type_text("label:Search all notes", "no such word")
        .unwrap();
    app.wait_for("No note holds that.").unwrap();

    // A hit opens its note, in the source, where its line is.
    app.clear("label:Search all notes").unwrap();
    app.type_text("label:Search all notes", "bold").unwrap();
    app.wait_for("text:Hello.md:6").unwrap();
    app.press("text:Hello.md:6").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.wait_for_gone("label:Search all notes", Duration::from_secs(2))
        .unwrap();

    // The query and results survive opening a hit and coming back.
    app.press_key("mod+shift+f").unwrap();
    app.wait_for("text:Hello.md:6").unwrap();
    assert_eq!(
        app.screen().input_text("label:Search all notes").unwrap(),
        Some("bold")
    );
    app.press_key("mod+shift+f").unwrap();
    app.wait_for("label:Markdown source").unwrap();

    // What is typed into the note is found once it is saved, without
    // reading the note's old text.
    app.type_text("label:Markdown source", "Zebra").unwrap();
    // Opening search leaves the note and saves it immediately.
    app.press_key("mod+shift+f").unwrap();
    app.clear("label:Search all notes").unwrap();
    app.type_text("label:Search all notes", "zebra").unwrap();
    app.wait_for_within("text:/Zebra/", Duration::from_secs(1))
        .unwrap();
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn desktop_refreshes_an_open_search_after_a_file_changes() {
    let root = vault("live-search");
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();
    app.press_key("mod+shift+f").unwrap();
    app.type_text("label:Search all notes", "zebra").unwrap();
    app.wait_for("No note holds that.").unwrap();

    // A new on-disk match updates the results without editing the query.
    fs::write(root.join("Hello.md"), "A Zebra arrived.\n").unwrap();
    app.wait_for("text:A Zebra arrived.").unwrap();
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn desktop_search_is_scoped_to_the_open_vault() {
    let first = vault("search-first");
    let second = vault("search-second");
    fs::write(first.join("Hello.md"), "Some Zebra in the first vault.\n").unwrap();
    fs::write(second.join("New.md"), "Some Zebra in the second vault.\n").unwrap();
    let mut app = Headless::start(desktop(&first)).unwrap();
    app.on_pick_paths(vec![second.clone()]).unwrap();
    app.wait_for("Hello").unwrap();
    app.press_key("mod+shift+f").unwrap();
    app.type_text("label:Search all notes", "zebra").unwrap();
    app.wait_for("Some Zebra in the first vault.").unwrap();

    // Leave with another query pending and switch using the vault picker.
    app.type_text("label:Search all notes", " in").unwrap();
    app.press("label:Settings").unwrap();
    app.press("label:Sync").unwrap();
    app.press("text:Open folder…").unwrap();
    app.wait_for("New").unwrap();

    app.press_key("mod+shift+f").unwrap();
    app.wait_for("Type to search every note.").unwrap();
    app.type_text("label:Search all notes", "zebra").unwrap();
    app.wait_for("Some Zebra in the second vault.").unwrap();
    assert!(!app.shows("Some Zebra in the first vault."));
    let _ = fs::remove_dir_all(first);
    let _ = fs::remove_dir_all(second);
}

#[test]
fn desktop_scrolls_from_read_mode_to_the_matching_line() {
    let root = vault("search-jump");
    let mut lines = vec!["A line of prose."; 500];
    lines[399] = "Unique scroll target";
    fs::write(root.join("Hello.md"), lines.join("\n")).unwrap();
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("text:Read").unwrap();
    app.press("text:Read").unwrap();

    app.press_key("mod+shift+f").unwrap();
    app.type_text("label:Search all notes", "unique scroll target")
        .unwrap();
    app.wait_for("text:Hello.md:400").unwrap();
    app.press("text:Hello.md:400").unwrap();
    app.wait_for("label:Markdown source").unwrap();
    app.wait_for("text:Unique scroll target").unwrap();
    let line = app
        .screen()
        .layout("text:Unique scroll target")
        .unwrap()
        .unwrap();
    assert!(!line.hidden && !line.clipped);
    let _ = fs::remove_dir_all(root);
}
