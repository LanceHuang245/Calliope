//! Syncing the open vault (MVP F8): its settings, the runs and what starts
//! them (the sync button, opening the vault, coming back to the app, 30
//! seconds after an edit, and every few minutes on the desktops), and what
//! the UI shows of them.

use std::{
    cell::Cell,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use calliope_sync::{Config, Error, Provider};
use nsh_core::platform::Lifecycle;

use super::{note::save, *};
use crate::secrets;

/// Edits sync this long after the last one.
const AFTER_EDITS: Duration = Duration::from_secs(30);
/// Opening the vault or coming back to the app syncs, unless it synced
/// this recently (a desktop's window comes back often).
const AGAIN_AFTER: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
struct SyncConfig(Config);
/// The run that would delete this many files, waiting for the user.
#[derive(Clone, Default)]
struct PendingDeletions(Option<usize>);
/// The password went to a file, for want of secure storage.
#[derive(Clone, Default)]
struct SecretInFile(bool);

/// What the UI shows of syncing.
#[derive(Clone, Default, PartialEq)]
pub(crate) enum SyncStatus {
    #[default]
    Idle,
    /// Files done, of how many.
    Running(usize, usize),
    /// When the last run ended (`15:30`).
    Done(String),
    Failed(String),
}

thread_local! {
    static RUNNING: Cell<bool> = const { Cell::new(false) };
    /// When the open note was last saved, while that is not synced yet.
    static EDITED: Cell<Option<Instant>> = const { Cell::new(None) };
    /// When the last run ended.
    static LAST_RUN: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The running run's files done and files to do, as its thread reports
/// them (one run at a time).
static PROGRESS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];

pub(crate) fn sync_config() -> Config {
    get::<SyncConfig>().0
}

pub(crate) fn sync_status() -> SyncStatus {
    get::<SyncStatus>()
}

pub(crate) fn pending_deletions() -> Option<usize> {
    get::<PendingDeletions>().0
}

pub(crate) fn secret_in_file() -> bool {
    get::<SecretInFile>().0
}

/// The vault's account in the secrets: its folder.
fn account(vault: &Vault) -> String {
    vault.root().display().to_string()
}

/// Reads the settings of the vault just opened, then syncs it.
pub(super) fn load_sync() {
    let Some(vault) = untracked(vault) else {
        return;
    };
    let mut config = Config::load(&vault);
    if config.device.is_empty() {
        config.device = device_name();
    }
    set(SyncConfig(config));
    set(SyncStatus::Idle);
    show_secret_in_file(&vault);
    LAST_RUN.set(None);
    auto_sync();
}

/// A Linux desktop without a keyring keeps the password in a file: the
/// settings say so.
fn show_secret_in_file(vault: &Vault) {
    set(SecretInFile(
        cfg!(target_os = "linux") && secrets::in_file(&account(vault)),
    ));
}

/// Keeps `config` for the open vault, and `secret` (the password or secret
/// key) unless it is empty, which keeps the one before.
pub(crate) fn save_sync(config: Config, secret: &str) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    if let Err(e) = config.save(&vault) {
        return notify(e);
    }
    if !secret.is_empty() {
        if let Err(e) = secrets::set(&account(&vault), secret) {
            notify(e);
        }
        show_secret_in_file(&vault);
    }
    set(SyncConfig(config));
    notify(strings::SYNC_SAVED);
}

/// Checks the settings being edited, `secret` empty for the saved one.
pub(crate) fn test_sync(config: Config, secret: String) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    let account = account(&vault);
    spawn_task(async move {
        let tested = background(async move {
            let secret = match secret.is_empty() {
                true => secrets::get(&account).unwrap_or_default(),
                false => secret,
            };
            calliope_sync::test(&config, &secret).await
        });
        match tested.await {
            Ok(Ok(())) => notify(strings::SYNC_WORKS),
            Ok(Err(e)) => notify(strings::sync_failed(&e)),
            Err(e) => notify(e),
        }
    });
}

/// The sync button; it opens the settings while sync is off.
pub(crate) fn sync_now() {
    match untracked(sync_config).provider {
        Provider::Off => set_route(Route::Settings),
        _ => start(false),
    }
}

/// The user's answer to a run that would delete many files.
pub(crate) fn confirm_deletions(go_ahead: bool) {
    set(PendingDeletions(None));
    if go_ahead {
        start(true);
    }
}

/// The open note was saved: it syncs once editing stops for a while.
pub(super) fn edited() {
    EDITED.set(Some(Instant::now()));
}

/// Syncs on its own, when it is on and did not just sync.
fn auto_sync() {
    let recent = LAST_RUN.get().is_some_and(|t| t.elapsed() < AGAIN_AFTER);
    if untracked(sync_config).auto && !recent {
        start(false);
    }
}

/// The timers: after edits, every few minutes on the desktops, on coming
/// back to the app; and the progress of a run.
pub(super) fn start_sync_timers() {
    set_interval(Duration::from_secs(1), || {
        if RUNNING.get() {
            let [done, total] = PROGRESS.each_ref().map(|n| n.load(Ordering::Relaxed));
            let status = SyncStatus::Running(done, total);
            if untracked(sync_status) != status {
                set(status);
            }
        }
        let config = untracked(sync_config);
        let interval = Duration::from_secs(u64::from(config.interval) * 60);
        let edits = EDITED.get().is_some_and(|t| t.elapsed() >= AFTER_EDITS);
        let due = is_desktop()
            && config.interval > 0
            && LAST_RUN.get().is_some_and(|t| t.elapsed() >= interval);
        if config.auto && (edits || due) {
            start(false);
        }
    });
    effect(|| {
        if lifecycle() == Lifecycle::Active {
            untracked(auto_sync);
        }
    });
}

/// Starts a run, unless one is running or sync is off; `confirmed` lets it
/// delete many files.
fn start(confirmed: bool) {
    let config = untracked(sync_config);
    let Some(vault) = untracked(vault) else {
        return;
    };
    if RUNNING.get() || config.provider == Provider::Off {
        return;
    }
    // What is typed so far goes with it.
    save();
    EDITED.set(None);
    RUNNING.set(true);
    for n in &PROGRESS {
        n.store(0, Ordering::Relaxed);
    }
    set(SyncStatus::Running(0, 0));
    let account = account(&vault);
    spawn_task(async move {
        let run = background(async move {
            let secret = secrets::get(&account).unwrap_or_default();
            calliope_sync::sync(&vault, &config, &secret, confirmed, |done, total| {
                PROGRESS[0].store(done, Ordering::Relaxed);
                PROGRESS[1].store(total, Ordering::Relaxed);
            })
            .await
        });
        let result = run.await;
        RUNNING.set(false);
        LAST_RUN.set(Some(Instant::now()));
        match result {
            Ok(Ok(report)) => {
                let time = jiff::Zoned::now().strftime("%H:%M").to_string();
                set(SyncStatus::Done(time));
                if !report.conflicts.is_empty() {
                    notify(strings::sync_conflicts(&report.conflicts));
                }
            }
            Ok(Err(Error::TooManyDeletions(n))) => {
                set(SyncStatus::Idle);
                set(PendingDeletions(Some(n)));
            }
            Ok(Err(e)) => {
                log::warn!("sync failed: {e:?}");
                set(SyncStatus::Failed(e.to_string()));
                notify(strings::sync_failed(&e));
            }
            Err(e) => notify(e),
        }
        // What it downloaded or deleted shows.
        refresh();
    });
}

/// This device's name in conflict copies: the computer's name on the
/// desktops (Linux and Windows), the system's elsewhere. The user can
/// change it in the settings.
fn device_name() -> String {
    let name = if cfg!(target_os = "windows") {
        std::env::var("COMPUTERNAME").ok()
    } else if cfg!(target_os = "linux") {
        fs::read_to_string("/proc/sys/kernel/hostname").ok()
    } else {
        None
    };
    let system = if cfg!(target_os = "macos") {
        "Mac"
    } else if cfg!(target_os = "ios") {
        "iPhone"
    } else if cfg!(target_os = "android") {
        "Android"
    } else {
        "Computer"
    };
    (name.map(|n| n.trim().to_owned()))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| system.to_owned())
}
