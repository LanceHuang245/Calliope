//! Sync runs between devices, each a vault in a temporary folder, all on
//! one remote: a folder through OpenDAL's file system service (its memory
//! service gives no versions), then the WebDAV and S3 servers
//! that `CALLIOPE_TEST_WEBDAV` (`endpoint user secret`) and
//! `CALLIOPE_TEST_S3` (`endpoint bucket user secret`) name, when set (CI
//! runs them in Docker).

mod encrypted;

use std::{collections::BTreeMap, fs};

use calliope_core::vault::Vault;
use opendal::{Operator, services::Fs};

use crate::{Config, Error, Provider, Report, engine::run, local, remote, sync_remote::Remote};

/// A device's vault, empty, for `test`.
fn device(test: &str, name: &str) -> Vault {
    Vault::open(folder(test, name)).unwrap()
}

/// An empty temporary folder for `test`.
fn folder(test: &str, name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "calliope-sync-{test}-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path
}

const PASSPHRASE: &str = "Phase 5 compatibility test";

/// Run the same sync scenarios on plain and encrypted remotes, both on disk
/// and against the servers CI supplies.
async fn remotes(test: &str) -> Vec<Remote> {
    let mut remotes = Vec::new();
    for encrypted in [false, true] {
        let mode = if encrypted { "crypt" } else { "plain" };
        let folder = folder(test, mode);
        let op = Operator::new(Fs::default().root(&folder.to_string_lossy())).unwrap();
        remotes.push(if encrypted {
            Remote::encrypted_fs(op, PASSPHRASE).await.unwrap()
        } else {
            Remote::plain(op)
        });

        for (mut config, secret) in server_configs(&format!("{test}-{mode}")) {
            config.encrypted = encrypted;
            let op = remote::operator(&config, &secret).await.unwrap();
            op.delete_with("").recursive(true).await.unwrap();
            remotes.push(
                Remote::connect(&config, &secret, PASSPHRASE, true, true)
                    .await
                    .unwrap(),
            );
        }
    }
    remotes
}

fn server_configs(test: &str) -> Vec<(Config, String)> {
    let root = format!("calliope-{test}-{}", std::process::id());
    let mut configs = Vec::new();
    if let Ok(env) = std::env::var("CALLIOPE_TEST_WEBDAV") {
        let [endpoint, user, secret] = env.split(' ').collect::<Vec<_>>()[..] else {
            panic!("CALLIOPE_TEST_WEBDAV: endpoint user secret");
        };
        configs.push((
            Config {
                provider: Provider::WebDav,
                endpoint: endpoint.into(),
                user: user.into(),
                root: root.clone(),
                ..Config::default()
            },
            secret.into(),
        ));
    }
    if let Ok(env) = std::env::var("CALLIOPE_TEST_S3") {
        let [endpoint, bucket, user, secret] = env.split(' ').collect::<Vec<_>>()[..] else {
            panic!("CALLIOPE_TEST_S3: endpoint bucket user secret");
        };
        configs.push((
            Config {
                provider: Provider::S3,
                endpoint: endpoint.into(),
                bucket: bucket.into(),
                user: user.into(),
                root,
                ..Config::default()
            },
            secret.into(),
        ));
    }
    configs
}

/// A run, its deletions confirmed unless a test asks otherwise.
async fn sync(vault: &Vault, remote: &Remote) -> Report {
    run(
        vault,
        remote,
        remote.identity().as_str(),
        "Test",
        true,
        |_, _| {},
    )
    .await
    .unwrap()
}

/// The content of every file of `vault` that syncs, by path.
fn files(vault: &Vault) -> BTreeMap<String, String> {
    (local::list(vault).unwrap().into_keys())
        .map(|path| (path.clone(), vault.read(&path).unwrap()))
        .collect()
}

fn write(vault: &Vault, path: &str, text: &str) {
    vault.write(path, text).unwrap();
}

/// Writes `text` (as many bytes as before) to the file at `path`, which
/// keeps its modification time: a second write in the same tick of the
/// clock (on Windows, 926 of 1000 quick rewrites).
fn rewrite_in_the_same_tick(path: &std::path::Path, text: &str) {
    let modified = fs::metadata(path).unwrap().modified().unwrap();
    fs::write(path, text).unwrap();
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(modified).unwrap();
}

#[tokio::test]
async fn edits_in_the_same_tick_are_seen() {
    // A remote of its own in a folder, to rewrite its files the same way.
    let folder = folder("tick", "remote");
    fs::create_dir_all(&folder).unwrap();
    let op = Operator::new(Fs::default().root(&folder.to_string_lossy())).unwrap();
    let op = Remote::plain(op);
    let (a, b) = (device("tick", "a"), device("tick", "b"));
    write(&a, "Note.md", "hello");
    sync(&a, &op).await;
    // A local edit with the stamp the record has: uploaded.
    rewrite_in_the_same_tick(&a.abs("Note.md"), "HELLO");
    assert_eq!(sync(&a, &op).await.uploaded, 1);
    sync(&b, &op).await;
    // A remote edit with the version the record has: downloaded.
    rewrite_in_the_same_tick(&folder.join("Note.md"), "howdy");
    assert_eq!(sync(&b, &op).await.downloaded, 1);
    assert_eq!(b.read("Note.md").unwrap(), "howdy");
    // An edit between the listing and the download it would undo: kept
    // (`progress` hears of each file just before it is synced).
    rewrite_in_the_same_tick(&folder.join("Note.md"), "later");
    let edit = |_, _| rewrite_in_the_same_tick(&b.abs("Note.md"), "mine!");
    run(&b, &op, &op.identity(), "Test", true, edit)
        .await
        .unwrap();
    assert_eq!(b.read("Note.md").unwrap(), "mine!");
}

#[tokio::test]
async fn devices_follow_each_other() {
    for op in remotes("follow").await {
        let (a, b) = (device("follow", "a"), device("follow", "b"));
        write(&a, "Hello.md", "hello");
        write(&a, "Old.md", "old");
        write(&a, "Journal/Day.md", "day");
        write(&a, ".calliope/workspace.json", "{}");
        assert_eq!(sync(&a, &op).await.uploaded, 3);
        assert_eq!(sync(&b, &op).await.downloaded, 3);
        assert_eq!(files(&a), files(&b));
        assert!(!b.abs(".calliope/workspace.json").exists());

        write(&b, "Hello.md", "hello again");
        fs::remove_file(b.abs("Old.md")).unwrap();
        write(&b, "New.md", "new");
        sync(&b, &op).await;
        let report = sync(&a, &op).await;
        assert_eq!((report.downloaded, report.deleted), (2, 1));
        assert_eq!(files(&a), files(&b));
        // Deleted by the other device: in the trash.
        let today = jiff::ZonedDateTime::now().date();
        assert_eq!(a.read(&format!(".trash/{today}/Old.md")).unwrap(), "old");
        // Nothing left to do.
        let report = sync(&a, &op).await;
        assert_eq!(
            (report.uploaded, report.downloaded, report.deleted),
            (0, 0, 0)
        );
    }
}

#[tokio::test]
async fn conflicts_keep_both_sides() {
    for op in remotes("conflict").await {
        let (a, b) = (device("conflict", "a"), device("conflict", "b"));
        write(&a, "Plan.md", "plan");
        write(&a, "Gone.md", "gone");
        write(&a, "Restored.md", "original");
        sync(&a, &op).await;
        sync(&b, &op).await;

        write(&a, "Plan.md", "plan by a");
        write(&b, "Plan.md", "plan by b");
        // Deleted on one side, changed on the other: kept.
        fs::remove_file(a.abs("Gone.md")).unwrap();
        write(&b, "Gone.md", "kept by b");
        fs::remove_file(b.abs("Restored.md")).unwrap();
        write(&a, "Restored.md", "kept by a");
        sync(&a, &op).await;
        let report = sync(&b, &op).await;
        sync(&a, &op).await;

        assert_eq!(report.conflicts.len(), 1);
        let copy = &report.conflicts[0];
        assert!(copy.starts_with("Plan (conflict ") && copy.ends_with(" Test).md"));
        assert_eq!(b.read("Plan.md").unwrap(), "plan by b");
        assert_eq!(b.read(copy).unwrap(), "plan by a");
        assert_eq!(b.read("Gone.md").unwrap(), "kept by b");
        assert_eq!(b.read("Restored.md").unwrap(), "kept by a");
        assert_eq!(files(&a), files(&b));
    }
}

#[tokio::test]
async fn a_first_sync_merges_both_sides() {
    for op in remotes("merge").await {
        let (a, b) = (device("merge", "a"), device("merge", "b"));
        write(&a, "A.md", "a");
        write(&a, "Same.md", "same");
        write(&a, "Both.md", "from a");
        write(&b, "B.md", "b");
        write(&b, "Same.md", "same");
        write(&b, "Both.md", "from b");
        sync(&a, &op).await;
        let report = sync(&b, &op).await;
        sync(&a, &op).await;
        // Only the file that differs is a conflict.
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(files(&a).len(), 5);
        assert_eq!(files(&a), files(&b));
    }
}

/// A vault pointed at another remote, one with files of its own, merges
/// with it as on a first sync: the records of the old remote overwrite and
/// delete nothing (MVP 8.5).
#[tokio::test]
async fn a_new_remote_is_merged() {
    let fs_remote = |name| {
        let root = folder("switch", name).to_string_lossy().into_owned();
        Remote::plain(Operator::new(Fs::default().root(&root)).unwrap())
    };
    let (old, new) = (fs_remote("old"), fs_remote("new"));
    let (a, b) = (device("switch", "a"), device("switch", "b"));
    write(&a, "Note.md", "mine");
    write(&a, "Only here.md", "here");
    sync(&a, &old).await;
    write(&b, "Note.md", "theirs");
    sync(&b, &new).await;

    let report = sync(&a, &new).await;
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(a.read("Note.md").unwrap(), "mine");
    assert_eq!(a.read(&report.conflicts[0]).unwrap(), "theirs");
    assert_eq!(a.read("Only here.md").unwrap(), "here");
}

#[tokio::test]
async fn an_emptied_remote_deletes_nothing() {
    for op in remotes("emptied").await {
        let a = device("emptied", "a");
        write(&a, "Note.md", "note");
        sync(&a, &op).await;
        op.delete("Note.md").await.unwrap();
        assert_eq!(sync(&a, &op).await.uploaded, 1);
        assert_eq!(a.read("Note.md").unwrap(), "note");
    }
}

#[tokio::test]
async fn asks_before_deleting_many() {
    for op in remotes("many").await {
        let (a, b) = (device("many", "a"), device("many", "b"));
        for i in 0..30 {
            write(&a, &format!("{i}.md"), "");
        }
        sync(&a, &op).await;
        sync(&b, &op).await;
        for i in 0..25 {
            fs::remove_file(a.abs(&format!("{i}.md"))).unwrap();
        }
        let refused = run(&a, &op, &op.identity(), "Test", false, |_, _| {}).await;
        assert!(matches!(refused, Err(Error::TooManyDeletions(25))));
        assert_eq!(files(&b).len(), 30);
        run(&a, &op, &op.identity(), "Test", true, |_, _| {})
            .await
            .unwrap();
        sync(&b, &op).await;
        assert_eq!(files(&b).len(), 5);
    }
}

#[tokio::test]
async fn resumes_a_run_cut_short() {
    for op in remotes("resume").await {
        let (a, b) = (device("resume", "a"), device("resume", "b"));
        write(&a, "One.md", "one");
        sync(&a, &op).await;
        sync(&b, &op).await;
        write(&a, "One.md", "one again");
        write(&a, "Two.md", "two");
        // Cut short after its uploads, before writing its records.
        let state = a.read(".calliope/sync/state.json").unwrap();
        sync(&a, &op).await;
        write(&a, ".calliope/sync/state.json", &state);

        let report = sync(&a, &op).await;
        assert!(report.conflicts.is_empty());
        sync(&b, &op).await;
        assert_eq!(files(&a), files(&b));
    }
}

/// Two devices taking turns 100 times: each changes, renames and deletes
/// some notes, then both sync. Both end the same, and every change still
/// on its device when it synced is kept, in its file or a conflict copy.
#[tokio::test]
async fn takes_turns_without_losing_anything() {
    for op in remotes("turns").await {
        let devices = [device("turns", "a"), device("turns", "b")];
        let names = ["a.md", "b.md", "c.md", "d/e.md", "d/f.md"];
        let mut seed = 1_u64;
        let mut next = |n: u64| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) % n
        };
        let mut count = 0;
        for _ in 0..100 {
            let mut written = Vec::new();
            for vault in &devices {
                for _ in 0..next(3) {
                    let name = names[next(5) as usize];
                    let to = names[next(5) as usize];
                    match next(4) {
                        0 => drop(fs::remove_file(vault.abs(name))),
                        1 if !vault.abs(to).exists() => {
                            drop(fs::rename(vault.abs(name), vault.abs(to)))
                        }
                        _ => {
                            count += 1;
                            written.push(format!("change {count}"));
                            write(vault, name, &format!("change {count}"));
                        }
                    }
                }
            }
            let kept: Vec<String> = (devices.iter().flat_map(|v| files(v).into_values()))
                .filter(|text| written.contains(text))
                .collect();
            sync(&devices[0], &op).await;
            sync(&devices[1], &op).await;
            sync(&devices[0], &op).await;
            let synced = files(&devices[0]);
            assert_eq!(synced, files(&devices[1]));
            for text in kept {
                assert!(synced.values().any(|t| *t == text), "{text} was lost");
            }
        }
    }
}
