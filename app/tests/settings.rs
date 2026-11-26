//! Desktop settings navigation and persistence.

mod common;

use std::{fs, sync::Arc, time::Duration};

use common::{desktop, vault};
use nsh_testing::{Headless, MemoryStorage};

/// Sync in the settings: the toolbar shortcut follows the saved provider.
/// WebDAV's settings go to the vault's `.calliope/sync.json` (the password,
/// left empty here, would go to the system's keyring). The notice saying so
/// goes after three seconds; a problem stays until closed. A device name
/// left empty is the computer's again.
#[test]
fn desktop_saves_the_sync_settings() {
    let root = vault("sync");
    let mut app = Headless::start(desktop(&root)).unwrap();
    app.wait_for("Hello").unwrap();

    assert!(!app.shows("label:Sync"));
    app.press("label:Settings").unwrap();
    app.wait_for("label:Sync").unwrap();
    app.press("label:Sync").unwrap();
    app.wait_for("text:Save").unwrap();
    assert!(app.shows("Sync provider"));
    // While sync is off, only Save is shown.
    assert!(!app.shows("Test connection") && !app.shows("Sync now"));
    app.press("text:WebDAV").unwrap();
    app.wait_for("text:Test connection").unwrap();
    // A problem stays until closed, under the notices that come after it.
    app.press("text:Test connection").unwrap();
    app.wait_for("text:/^Sync failed:/").unwrap();
    app.type_text("label:Server address", "https://dav.example.com/dav")
        .unwrap();
    app.type_text("label:User name", "me").unwrap();
    // The device name is the computer's at first; left empty, it stays.
    let device = (app.screen().input_text("label:Device name").unwrap())
        .unwrap()
        .to_owned();
    assert!(!device.is_empty());
    app.clear("label:Device name").unwrap();
    // Not on its own: saved, it would sync with the made-up server at once.
    app.press("text:Off >> nth=1").unwrap();
    app.press("text:Save").unwrap();
    app.wait_for("The sync settings were saved.").unwrap();
    // The notice stays three seconds, then goes on its own.
    app.wait(Duration::from_secs(1)).unwrap();
    assert!(app.shows("The sync settings were saved."));
    app.wait_for_gone("The sync settings were saved.", Duration::from_secs(5))
        .unwrap();
    assert!(app.shows("Sync failed: endpoint is empty."));
    app.press("label:Close").unwrap();
    app.wait_for_gone("text:/^Sync failed:/", Duration::from_secs(2))
        .unwrap();
    let saved = fs::read_to_string(root.join(".calliope/sync.json")).unwrap();
    assert!(saved.contains(r#""provider": "webdav""#), "{saved}");
    assert!(saved.contains("https://dav.example.com/dav"), "{saved}");
    assert!(saved.contains(r#""auto": false"#), "{saved}");
    // Opened again, the settings show the computer's name.
    app.press("label:Back").unwrap();
    app.wait_for("label:Sync").unwrap();
    app.press("label:Settings").unwrap();
    app.wait_for("System").unwrap();
    assert!(app.shows("Theme"));
    assert!(!app.shows("Save"));
    app.press("label:About").unwrap();
    app.wait_for("Export logs…").unwrap();
    assert!(!app.shows("System"));
    app.press("label:Sync").unwrap();
    app.wait_for("label:Device name").unwrap();
    let shown = app.screen().input_text("label:Device name").unwrap();
    assert_eq!(shown, Some(device.as_str()));
    app.press("text:Off >> nth=0").unwrap();
    app.press("text:Save").unwrap();
    app.wait_for("The sync settings were saved.").unwrap();
    app.wait_for_gone("The sync settings were saved.", Duration::from_secs(5))
        .unwrap();
    app.press("label:Back").unwrap();
    app.wait_for("label:Settings").unwrap();
    assert!(!app.shows("label:Sync"));
    let _ = fs::remove_dir_all(&root);
}

/// The interface's language: picked in the Appearance settings, kept on
/// the device at once (with a dialog saying a restart shows it), and spoken
/// from the next start on, since the pages showing were made with the old
/// texts.
#[test]
fn desktop_keeps_the_chosen_language() {
    let root = vault("language");
    let stored = MemoryStorage::new([("vault".to_owned(), root.display().to_string())]);
    let mut config = desktop(&root);
    config.storage = Some(stored.clone() as Arc<dyn nsh_core::storage::StorageBackend>);
    let mut app = Headless::start(config).unwrap();
    app.wait_for("Hello").unwrap();
    app.press("label:Settings").unwrap();
    app.wait_for("Theme").unwrap();

    // The row unfolds into both languages, each named in its own language.
    assert!(!app.shows("简体中文"));
    app.press("label:Language").unwrap();
    app.wait_for("label:简体中文").unwrap();

    // A pick is kept at once; the dialog says what shows it.
    app.press("label:简体中文").unwrap();
    app.wait_for("The language was saved.").unwrap();
    assert_eq!(
        stored.entries().get("language").map(String::as_str),
        Some("zh-Hans")
    );
    // The row shows the pick; the rest of the page keeps the old texts.
    app.wait_for("text:简体中文").unwrap();
    assert!(app.shows("Theme"));
    app.press("text:OK").unwrap();
    app.wait_for_gone("The language was saved.", Duration::from_secs(2))
        .unwrap();

    // The next start speaks it: the same device, opened again.
    let mut config = desktop(&root);
    config.storage = Some(stored.clone() as Arc<dyn nsh_core::storage::StorageBackend>);
    let mut app = Headless::start(config).unwrap();
    app.wait_for("label:设置").unwrap();
    app.press("label:设置").unwrap();
    app.wait_for("外观").unwrap();
    assert!(app.shows("语言"));
    let _ = fs::remove_dir_all(&root);
}
