//! Runs Calliope directly on this machine's desktop backend (`cargo run`).
//! iOS and Android run through `nsh run app --target ios|android`.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

/// The app's id (`nsh.json` `"id"`): it names the app's data directory,
/// as in the builds `nsh package` makes.
const APP_ID: &str = "io.github.lancehuang245.calliope";

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    let mut window = nsh_core::desktop::MainWindowOptions::default();
    window.default_position = Some(nsh_core::desktop::WindowPosition::Center);
    // Reopen where the user left the window; centered only on the first run.
    window.restore_bounds = true;
    platform::run_app(
        platform::AppSource::Rust(
            platform::Root::of::<calliope::App>()
                .app_id(APP_ID)
                .main_window(window),
        ),
        platform::AppOptions {
            title: "Calliope".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: APP_ID.to_owned(),
            size: (1200, 800),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("Run this binary on macOS, Windows or Linux; use nsh run for iOS and Android.");
