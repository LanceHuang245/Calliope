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
use nsh_core::{notifications::Notification, platform::Lifecycle};

use super::{note::save, *};
use crate::secrets;

mod setup;
pub(crate) use setup::{create_from_sync, save_sync, test_sync};

/// Edits sync this long after the last one.
const AFTER_EDITS: Duration = Duration::from_secs(30);
/// Opening the vault or coming back to the app syncs, unless it synced
/// this recently (a desktop's window comes back often).
const AGAIN_AFTER: Duration = Duration::from_secs(60);

#[derive(Clone, Default)]
struct SyncConfig(Config);
/// The password went to a file, for want of secure storage.
#[derive(Clone, Default)]
struct SecretInFile(bool);
/// A vault made from sync is checking its connection.
#[derive(Clone, Default)]
struct Connecting(bool);

/// What the UI shows of syncing.
#[derive(Clone, Default, PartialEq)]
pub(crate) enum SyncStatus {
    #[default]
    Idle,
    /// Files done, of how many.
    Running(usize, usize),
    /// When the last run ended (`15:30`), and how many conflict copies it made.
    Done(String, usize),
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

pub(crate) fn secret_in_file() -> bool {
    get::<SecretInFile>().0
}

pub(crate) fn connecting() -> bool {
    get::<Connecting>().0
}

/// The vault's account in the secrets: its folder.
fn account(vault: &Vault) -> String {
    vault.root().display().to_string()
}

/// Scope the passphrase to the vault and destination so a failed setup for
/// another remote cannot replace the credential used by an existing one.
fn crypt_account(account: &str, config: &Config) -> String {
    format!(
        "{account}
{:?}
{}
{}
{}",
        config.provider,
        config.endpoint.trim_end_matches('/'),
        config.bucket,
        config.root.trim_matches('/')
    )
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

/// Persists the sync settings and reports whether they were kept.
fn keep_sync(mut config: Config, secret: &str) -> bool {
    let Some(vault) = untracked(vault) else {
        return false;
    };
    if let Err(e) = config.save(&vault) {
        notify(e);
        return false;
    }
    if !secret.is_empty() {
        if let Err(e) = secrets::set(&account(&vault), secret) {
            notify(e);
        }
        show_secret_in_file(&vault);
    }
    // Left empty, the device name is this device's own, as on opening the
    // vault: conflict copies are named after it (the file keeps it empty).
    if config.device.is_empty() {
        config.device = device_name();
    }
    set(SyncConfig(config));
    true
}

/// The sync button; it opens the settings while sync is off.
pub(crate) fn sync_now() {
    match untracked(sync_config).provider {
        Provider::Off => set_route(Route::Settings),
        _ => start(false),
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
        // Never synced yet (sync just turned on): due at once, then counted
        // from that run.
        let due = is_desktop()
            && config.interval > 0
            && LAST_RUN.get().is_none_or(|t| t.elapsed() >= interval);
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
    if RUNNING.get() || untracked(connecting) || config.provider == Provider::Off {
        return;
    }
    // Encrypted sync checks the passphrase before writing vault files.
    // Its pending editor changes are still handled by the normal autosave.
    if !config.encrypted {
        save();
    }
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
            let passphrase = if config.encrypted {
                secrets::passphrase(&crypt_account(&account, &config)).unwrap_or_default()
            } else {
                String::new()
            };
            calliope_sync::sync(
                &vault,
                &config,
                &secret,
                &passphrase,
                confirmed,
                |done, total| {
                    PROGRESS[0].store(done, Ordering::Relaxed);
                    PROGRESS[1].store(total, Ordering::Relaxed);
                },
            )
            .await
        });
        let result = run.await;
        RUNNING.set(false);
        LAST_RUN.set(Some(Instant::now()));
        match result {
            Ok(Ok(report)) => {
                let time = jiff::ZonedDateTime::now().strftime("%H:%M").to_string();
                set(SyncStatus::Done(time, report.conflicts.len()));
                if !report.conflicts.is_empty() {
                    notify(strings::sync_conflicts(&report.conflicts));
                    spawn_task(notify_system(report.conflicts));
                }
            }
            // A run that would delete many files asks first, then runs
            // again letting it.
            Ok(Err(Error::TooManyDeletions(n))) => {
                set(SyncStatus::Idle);
                let ask = Ask {
                    title: strings::DELETE_FILES.to_owned(),
                    description: Some(strings::deleting(n).into()),
                    action: strings::DELETE,
                    destructive: true,
                };
                confirm(ask, || start(true));
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

/// Tells of the conflict `copies` a run made in a system notification too
/// (MVP 8.4), asking to show notifications the first time. Refused, the
/// notice in the app is left.
async fn notify_system(copies: Vec<String>) {
    request_notification_permission().await;
    let body = strings::sync_conflicts(&copies);
    let notification = Notification::new("sync-conflicts", strings::SYNC_CONFLICTS, body);
    show_notification(notification).await;
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
