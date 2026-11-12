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
use opendal::{ErrorKind, Operator};

use crate::{
    Config, Result,
    local::{self, Stamp},
    plan::{self, Action, Side},
    remote::{self, Version},
    state::{self, Record, State},
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
pub async fn test(config: &Config, secret: &str) -> Result<()> {
    remote::list(&remote::operator(config, secret)?)
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
    confirmed: bool,
    progress: impl Fn(usize, usize) + Sync,
) -> Result<Report> {
    let op = remote::operator(config, secret)?;
    run(vault, &op, &config.device, confirmed, progress).await
}

pub(crate) async fn run(
    vault: &Vault,
    op: &Operator,
    device: &str,
    confirmed: bool,
    progress: impl Fn(usize, usize) + Sync,
) -> Result<Report> {
    let remote = remote::list(op).await?;
    let local = local::list(vault)?;
    let mut state = state::load(vault);
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
        let theirs = match (remote.get(path), state.get(path)) {
            (None, _) => Side::Gone,
            (Some(v), Some(r)) if v.same(&r.remote) => Side::Same,
            _ => Side::Changed,
        };
        if let Some(action) = plan::action(mine, theirs) {
            planned.push((path, action));
        }
    }
    let actions: Vec<Action> = planned.iter().map(|(_, a)| *a).collect();
    plan::check_deletions(&actions, paths.len(), confirmed)?;

    let mut run = Run {
        vault,
        op,
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
            state::save(vault, &run.state)?;
        }
    }
    state::save(vault, &run.state)?;
    result.map(|()| run.report)
}

/// How the local file at `path` (`stamp`, `None` when not there) differs
/// from its `record`. A file touched but holding the same content is the
/// same, and its record takes the new stamp.
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
                r.local = *s;
                Side::Same
            }
            false => Side::Changed,
        },
    })
}

struct Run<'a> {
    vault: &'a Vault,
    op: &'a Operator,
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
                let data = self.op.read(path).await?.to_vec();
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
                match self.op.stat(path).await {
                    Ok(meta) if !Version::of(&meta).same(&self.remote[path]) => return Ok(()),
                    Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
                    _ => self.op.delete(path).await?,
                }
                self.state.remove(path);
                self.report.deleted += 1;
            }
            Action::Conflict => self.conflict(path).await?,
            Action::Forget => drop(self.state.remove(path)),
        }
        Ok(())
    }

    /// Whether the local file at `path` changed since the listing.
    fn touched(&self, path: &str) -> bool {
        local::stamp(self.vault, path) != self.local.get(path).copied()
    }

    async fn upload(&mut self, path: &str) -> Result<()> {
        // Gone since the listing: the next run sees it gone.
        let Some(stamp) = local::stamp(self.vault, path) else {
            return Ok(());
        };
        let data = fs::read(self.vault.abs(path))?;
        let hash = local::hash(&data);
        self.op.write(path, data).await?;
        let remote = Version::of(&self.op.stat(path).await?);
        let record = Record {
            hash,
            local: stamp,
            remote,
        };
        self.state.insert(path.to_owned(), record);
        self.report.uploaded += 1;
        Ok(())
    }

    /// Changed on both sides (or new on both): the same content is only
    /// recorded; else the remote file is kept as a conflict copy, named
    /// with the time and this device (`Note (conflict 2026-11-06 1530
    /// Pixel 9).md`), and the local one takes its place on the remote.
    async fn conflict(&mut self, path: &str) -> Result<()> {
        let theirs = self.op.read(path).await?.to_vec();
        let mine = fs::read(self.vault.abs(path))?;
        let mut remote = self.remote[path].clone();
        if theirs != mine {
            let now = jiff::Zoned::now().strftime("%Y-%m-%d %H%M");
            let tag = format!("conflict {now} {}", self.device);
            let copy = self.vault.save_copy(path, tag.trim(), &theirs)?;
            self.upload(&copy).await?;
            self.op.write(path, mine.clone()).await?;
            remote = Version::of(&self.op.stat(path).await?);
            self.report.conflicts.push(copy);
        }
        self.record(path, &mine, remote);
        Ok(())
    }

    /// Records the local file at `path`, holding `data`, as `remote`.
    fn record(&mut self, path: &str, data: &[u8], remote: Version) {
        if let Some(local) = local::stamp(self.vault, path) {
            let hash = local::hash(data);
            (self.state).insert(
                path.to_owned(),
                Record {
                    hash,
                    local,
                    remote,
                },
            );
        }
    }
}
