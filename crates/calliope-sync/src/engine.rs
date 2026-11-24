//! A sync run (MVP 8.3–8.5): lists both sides, plans every file, then
//! carries the plan out file by file. The records are written as it goes
//! and when it ends, done or not, so a run cut short goes on where it
//! stopped; a record lost with it only costs comparing a file's content
//! again.

use std::{
    collections::{BTreeSet, HashMap},
    fs,
};

use calliope_core::vault::Vault;
use opendal::ErrorKind;

use crate::{
    Config, Result,
    local::{self, Stamp},
    plan::{self, Action, Side},
    remote::Version,
    state::{self, Record, State},
    sync_remote::Remote,
};

/// What a run did.
#[derive(Debug, Default)]
pub struct Report {
    pub uploaded: usize,
    pub downloaded: usize,
    pub deleted: usize,
    /// The conflict copies it made.
    pub conflicts: Vec<String>,
}

/// Checks that the remote `config` names can be listed with `secret`.
pub async fn test(config: &Config, secret: &str, passphrase: &str, allow_new: bool) -> Result<()> {
    Remote::connect(config, secret, passphrase, allow_new, false)
        .await?
        .list()
        .await
        .map(drop)
}

/// Prepare a configured remote, creating its verifier only on explicit setup.
pub async fn prepare(
    config: &Config,
    secret: &str,
    passphrase: &str,
    allow_new: bool,
) -> Result<()> {
    Remote::connect(config, secret, passphrase, allow_new, true)
        .await?
        .list()
        .await
        .map(drop)
}

/// Syncs `vault` with the remote `config` names, signed in with `secret`.
/// `progress` hears how many of the files to sync are done, of how many. A
/// run that would delete many files stops with
/// [`crate::Error::TooManyDeletions`] unless `confirmed`.
pub async fn sync(
    vault: &Vault,
    config: &Config,
    secret: &str,
    passphrase: &str,
    confirmed: bool,
    progress: impl Fn(usize, usize) + Sync,
) -> Result<Report> {
    let remote = Remote::connect(config, secret, passphrase, false, false).await?;
    run(
        vault,
        &remote,
        &config.remote_id(),
        &config.device,
        confirmed,
        progress,
    )
    .await
}

pub(crate) async fn run(
    vault: &Vault,
    store: &Remote,
    remote_id: &str,
    device: &str,
    confirmed: bool,
    progress: impl Fn(usize, usize) + Sync,
) -> Result<Report> {
    let remote = store.list().await?;
    let local = local::list(vault)?;
    let mut state = state::load(vault, remote_id);
    // One side empty and the other not (a new device, a remote folder
    // emptied or replaced): merged as a first sync, so the empty side
    // deletes nothing on the other (as Remotely Save does).
    if local.is_empty() != remote.is_empty() {
        state.clear();
    }
    let paths: BTreeSet<String> = (local.keys().chain(remote.keys()).chain(state.keys()))
        .cloned()
        .collect();
    let mut planned = Vec::new();
    for path in &paths {
        let mine = local_side(vault, path, local.get(path), state.get_mut(path))?;
        let theirs = match (remote.get(path), state.get_mut(path)) {
            (None, _) => Side::Gone,
            (Some(v), Some(r)) if v.same(&r.remote) => Side::Same,
            // Recorded too soon after it was written: its content tells.
            (Some(v), Some(r)) if r.unsure() => {
                let data = store.read(path).await?;
                match local::hash(&data) == r.hash {
                    true => {
                        r.remote = state::kept_version(v.clone());
                        Side::Same
                    }
                    false => Side::Changed,
                }
            }
            _ => Side::Changed,
        };
        if let Some(action) = plan::action(mine, theirs) {
            planned.push((path, action));
        }
    }
    plan::check_deletions(planned.iter().map(|(_, a)| *a), paths.len(), confirmed)?;

    let mut run = Run {
        vault,
        store,
        device,
        local,
        remote,
        state,
        report: Report::default(),
    };
    let mut result = Ok(());
    for (done, (path, action)) in planned.iter().enumerate() {
        progress(done, planned.len());
        result = run.apply(path, *action).await;
        if let Err(e) = &result {
            log::warn!("sync: {path}: {e}");
            break;
        }
        if done % 20 == 19 {
            state::save(vault, remote_id, &run.state)?;
        }
    }
    state::save(vault, remote_id, &run.state)?;
    result.map(|()| run.report)
}

/// How the local file at `path` (`stamp`, `None` when not there) differs
/// from its `record`. A file touched but holding the same content is the
/// same, and its record takes the new stamp (unless still too recent to
/// vouch for it: [`state::kept`]).
fn local_side(
    vault: &Vault,
    path: &str,
    stamp: Option<&Stamp>,
    record: Option<&mut Record>,
) -> Result<Side> {
    Ok(match (stamp, record) {
        (None, _) => Side::Gone,
        (Some(_), None) => Side::Changed,
        (Some(s), Some(r)) if *s == r.local => Side::Same,
        (Some(s), Some(r)) => match local::hash(&fs::read(vault.abs(path))?) == r.hash {
            true => {
                r.local = state::kept(*s);
                Side::Same
            }
            false => Side::Changed,
        },
    })
}

struct Run<'a> {
    vault: &'a Vault,
    store: &'a Remote,
    device: &'a str,
    /// Both sides as listed.
    local: HashMap<String, Stamp>,
    remote: HashMap<String, Version>,
    state: State,
    report: Report,
}

impl Run<'_> {
    async fn apply(&mut self, path: &str, action: Action) -> Result<()> {
        log::info!("sync: {action:?} {path}");
        match action {
            Action::Upload => self.upload(path).await?,
            // A file edited since the listing is left to the next run,
            // which sees it changed: never overwritten or deleted unseen.
            Action::Download | Action::DeleteLocal if self.touched(path) => {}
            Action::Download => {
                let data = self.store.read(path).await?;
                self.vault.write_bytes(path, &data)?;
                self.record(path, &data, self.remote[path].clone());
                self.report.downloaded += 1;
            }
            Action::DeleteLocal => {
                self.vault.trash(path)?;
                self.state.remove(path);
                self.report.deleted += 1;
            }
            Action::DeleteRemote => {
                // Likewise a remote file changed since the listing: the next
                // run downloads it.
                match self.store.version(path).await {
                    Ok(version) if !version.same(&self.remote[path]) => return Ok(()),
                    Err(crate::Error::Remote(e)) if e.kind() == ErrorKind::NotFound => {
                        self.store.delete(path).await?;
                    }
                    Err(e) => return Err(e),
                    _ => self.store.delete(path).await?,
                }
                self.state.remove(path);
                self.report.deleted += 1;
            }
            Action::Conflict => self.conflict(path).await?,
            Action::Forget => drop(self.state.remove(path)),
        }
        Ok(())
    }

    /// Whether the local file at `path` changed since the listing: its
    /// stamp did, or the stamp is too recent to vouch for it and the content
    /// is no longer the recorded one (unreadable: taken as changed).
    fn touched(&self, path: &str) -> bool {
        let stamp = local::stamp(self.vault, path);
        if stamp != self.local.get(path).copied() {
            return true;
        }
        match (stamp, self.state.get(path)) {
            (Some(s), Some(r)) if local::racy(s.modified) => {
                (fs::read(self.vault.abs(path))).map_or(true, |data| local::hash(&data) != r.hash)
            }
            _ => false,
        }
    }

    async fn upload(&mut self, path: &str) -> Result<()> {
        // Gone since the listing: the next run sees it gone.
        let Some(stamp) = local::stamp(self.vault, path) else {
            return Ok(());
        };
        let data = fs::read(self.vault.abs(path))?;
        let hash = local::hash(&data);
        let remote = self.store.write(path, data).await?;
        let record = Record::new(hash, stamp, remote);
        self.state.insert(path.to_owned(), record);
        self.report.uploaded += 1;
        Ok(())
    }

    /// Changed on both sides (or new on both): the same content is only
    /// recorded; else the remote file is kept as a conflict copy, named
    /// with the time and this device (`Note (conflict 2026-11-06 1530
    /// Pixel 9).md`), and the local one takes its place on the remote.
    async fn conflict(&mut self, path: &str) -> Result<()> {
        let theirs = self.store.read(path).await?;
        let mine = fs::read(self.vault.abs(path))?;
        let mut remote = self.remote[path].clone();
        if theirs != mine {
            let now = jiff::ZonedDateTime::now().strftime("%Y-%m-%d %H%M");
            let tag = format!("conflict {now} {}", self.device);
            let copy = self.vault.save_copy(path, tag.trim(), &theirs)?;
            self.upload(&copy).await?;
            remote = self.store.write(path, mine.clone()).await?;
            self.report.conflicts.push(copy);
        }
        self.record(path, &mine, remote);
        Ok(())
    }

    /// Records the local file at `path`, holding `data`, as `remote`.
    fn record(&mut self, path: &str, data: &[u8], remote: Version) {
        if let Some(local) = local::stamp(self.vault, path) {
            let record = Record::new(local::hash(data), local, remote);
            self.state.insert(path.to_owned(), record);
        }
    }
}
