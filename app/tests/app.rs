//! The app, started headless on the real engine (`cargo test`).

use nsh_engine::{EngineConfig, Root};
use nsh_testing::Headless;

#[test]
fn shows_the_welcome_note() {
    let mut app = Headless::start(EngineConfig::new(Root::of::<calliope::App>())).unwrap();
    app.wait_for("欢迎使用 Calliope").unwrap();
}
