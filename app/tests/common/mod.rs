//! What the app's tests share: a vault to start on, and the desktop
//! layout.

use std::{
    fs,
    path::{Path, PathBuf},
};

use nsh_core::{
    platform::{Idiom, Os},
    prelude::{px, size},
};
use nsh_engine::{EngineConfig, Root};
use nsh_testing::MemoryStorage;

/// A vault in a fresh temporary folder: a folder and a note with front
/// matter.
pub fn vault(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("calliope-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("Journal")).unwrap();
    fs::write(
        root.join("Hello.md"),
        "---\ntags: [a]\n---\n# Hello world\n\nSome **bold** text.\n",
    )
    .unwrap();
    root
}

/// The desktop layout, in a 1200 × 800 window, on the vault at `root` as
/// the last one opened.
pub fn desktop(root: &Path) -> EngineConfig {
    let mut config = EngineConfig::new(Root::of::<calliope::App>());
    config.environment.idiom = Idiom::Desktop;
    config.environment.os = Os::Linux;
    config.viewport = size(px(1200.0), px(800.0));
    config.storage = Some(MemoryStorage::new([(
        "vault".to_owned(),
        root.display().to_string(),
    )]));
    config
}
