//! Desktop settings navigation and persistence.

mod common;

use std::{collections::BTreeMap, fs, sync::Arc, time::Duration};

use common::{desktop, vault};
use nsh_core::{
    platform::{Idiom, Os},
    prelude::{px, size},
    storage::{StorageBackend, StorageChange},
};
use nsh_engine::{EngineConfig, Root};
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

    // Pages and dialogs opened after saving still use the startup language.
    app.press("label:About").unwrap();
    app.wait_for("Export logs…").unwrap();
    app.press_key("mod+p").unwrap();
    app.wait_for("label:Run a command").unwrap();
    app.wait_for("text:New note").unwrap();

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

/// A phone's count sits under the note's title, on the left; the Editor's
/// switch keeps it away, on that device.
#[test]
fn a_phone_counts_the_note_under_its_title() {
    let sandbox = std::env::temp_dir().join(format!("calliope-count-phone-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(sandbox.join("Notes")).unwrap();
    fs::write(sandbox.join("Notes/Hello.md"), "# Hi\n\n中文 abc\n").unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let stored = MemoryStorage::new([("vault".to_owned(), "Notes".to_owned())]);
    let mut config =
        EngineConfig::new(Root::of::<calliope::App>().translations(calliope::translations()));
    config.environment.os = Os::Android;
    config.environment.idiom = Idiom::Phone;
    config.viewport = size(px(430.0), px(800.0));
    config.storage = Some(stored.clone() as Arc<dyn StorageBackend>);
    let mut app = Headless::start(config).unwrap();

    // The note's own characters, the whitespace aside.
    app.wait_for("Select a note, or create one.").unwrap();
    app.press("label:Menu").unwrap();
    app.press("Hello").unwrap();
    app.wait_for("8 characters").unwrap();

    // The editor's setting keeps it away.
    app.press("label:Menu").unwrap();
    app.press("label:Settings").unwrap();
    app.wait_for("label:Editor").unwrap();
    app.press("label:Editor").unwrap();
    app.press("label:Character count").unwrap();
    assert_eq!(stored.entries()["characters"], "off");
    app.back().unwrap();
    app.wait_for_gone("text:8 characters", Duration::from_secs(2))
        .unwrap();
    let _ = fs::remove_dir_all(&sandbox);
}

#[test]
fn desktop_reports_a_refused_language_write() {
    struct RefusesLanguage(Arc<MemoryStorage>);

    impl StorageBackend for RefusesLanguage {
        fn load(&self) -> Result<BTreeMap<String, String>, String> {
            self.0.load()
        }

        fn write(&self, change: StorageChange) -> Result<(), String> {
            if matches!(&change, StorageChange::Set(key, _) if key == "language") {
                return Err("test backend refuses the language write".into());
            }
            self.0.write(change)
        }
    }

    let root = vault("refused-language");
    let stored = MemoryStorage::new([("vault".to_owned(), root.display().to_string())]);
    let mut config = desktop(&root);
    config.storage = Some(Arc::new(RefusesLanguage(stored.clone())));
    let mut app = Headless::start(config).unwrap();
    app.wait_for("label:Settings").unwrap();
    app.press("label:Settings").unwrap();
    app.wait_for("label:Language").unwrap();
    app.press("label:Language").unwrap();
    app.wait_for("label:简体中文").unwrap();
    app.press("label:简体中文").unwrap();
    app.wait_for("The interface language could not be saved.")
        .unwrap();
    assert!(app.shows("English"));
    assert!(!app.shows("The language was saved."));
    assert!(!stored.entries().contains_key("language"));
    let _ = fs::remove_dir_all(&root);
}
