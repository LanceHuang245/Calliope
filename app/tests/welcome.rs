//! A phone's first start, started headless on the real engine as `app.rs`'s
//! tests are; a process of its own, as it sets the phones' sandbox.

use std::{fs, time::Duration};

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

/// A vault made from sync with a server that is not there: the connection
/// is checked first (the form says so meanwhile), so the app says why and
/// makes no vault; Cancel goes back to the two ways in.
#[test]
fn phone_makes_a_vault_from_sync_only_once_connected() {
    let sandbox = std::env::temp_dir().join(format!("calliope-welcome-{}", std::process::id()));
    let _ = fs::remove_dir_all(&sandbox);
    fs::create_dir_all(&sandbox).unwrap();
    // SAFETY: set before the app starts, read by the app only.
    unsafe { std::env::set_var("CALLIOPE_SANDBOX", &sandbox) };
    let mut app = Headless::start(EngineConfig::new(
        Root::of::<calliope::App>().translations(calliope::translations()),
    ))
    .unwrap();

    app.wait_for("text:Create from sync").unwrap();
    app.press("text:Create from sync").unwrap();
    app.type_text("label:Vault name", "Remote").unwrap();
    // Nothing listens on the discard port.
    app.type_text("label:Server address", "http://127.0.0.1:9")
        .unwrap();
    app.type_text("label:User name", "me").unwrap();
    app.type_text("label:Password", "secret").unwrap();
    app.press("text:Create").unwrap();
    app.wait_for("Connecting…").unwrap();
    app.wait_for_within("text:/^Sync failed: /", Duration::from_secs(20))
        .unwrap();
    assert!(!app.shows("Connecting…"));
    assert!(!sandbox.join("Remote").exists());

    app.press("text:Cancel").unwrap();
    app.wait_for("text:Create from sync").unwrap();
    let _ = fs::remove_dir_all(&sandbox);
}
