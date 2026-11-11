//! The vault's link index: each note's links, what they resolve to and the
//! backlinks of each note, kept up to date as files change.

use std::{
    collections::{BTreeMap, HashMap},
    fs, io,
    time::SystemTime,
};

use crate::{
    link::{self, Link},
    note,
    vault::{Entry, EntryKind, Vault},
};

#[derive(Debug, Clone)]
struct Note {
    /// The file's modification time and size when its links were read.
    stamp: (SystemTime, u64),
    links: Vec<Link>,
}

/// A link to a note from another: the note it is in and its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    pub source: String,
    pub context: String,
}

#[derive(Debug, Clone, Default)]
pub struct Index {
    /// The notes by path.
    notes: BTreeMap<String, Note>,
    /// The paths of the notes of each title.
    titles: HashMap<String, Vec<String>>,
}

impl Index {
    /// Brings the index up to date with the vault's files, reading only
    /// the notes that are new or changed; whether anything changed.
    pub fn sync(&mut self, vault: &Vault) -> io::Result<bool> {
        let mut paths = Vec::new();
        note_paths(&vault.scan()?, &mut paths);
        let mut old = std::mem::take(&mut self.notes);
        let mut changed = false;
        for path in paths {
            let stamp = fs::metadata(vault.abs(&path)).and_then(|m| Ok((m.modified()?, m.len())));
            let Ok(stamp) = stamp else {
                continue;
            };
            // An unchanged note keeps its links; a new or changed one is read.
            let note = match old.remove(&path) {
                Some(note) if note.stamp == stamp => note,
                _ => {
                    changed = true;
                    let links = link::links(&vault.read(&path).unwrap_or_default());
                    Note { stamp, links }
                }
            };
            self.notes.insert(path, note);
        }
        // What is left of the old notes was deleted.
        changed |= !old.is_empty();
        if changed {
            self.titles.clear();
            for path in self.notes.keys() {
                let title = note::title(path).to_owned();
                self.titles.entry(title).or_default().push(path.clone());
            }
        }
        Ok(changed)
    }

    /// Every note's path, in order.
    pub fn notes(&self) -> impl Iterator<Item = &str> {
        self.notes.keys().map(String::as_str)
    }

    /// The links written in `note`.
    pub fn links(&self, note: &str) -> &[Link] {
        self.notes.get(note).map_or(&[], |n| &n.links)
    }

    /// The note a link's `target` names: by title anywhere in the vault,
    /// or by a path ending (`Folder/Note`). Among several, the one nearest
    /// the root.
    pub fn resolve(&self, target: &str) -> Option<&str> {
        let target = target.strip_suffix(".md").unwrap_or(target);
        self.titles
            .get(note::title(target))?
            .iter()
            .filter(|path| {
                // The path without `.md` is the target, or ends in `/target`.
                let rest = path[..path.len() - ".md".len()].strip_suffix(target);
                rest.is_some_and(|rest| rest.is_empty() || rest.ends_with('/'))
            })
            .min_by_key(|path| (path.matches('/').count(), path.as_str()))
            .map(String::as_str)
    }

    /// The links to `note` from the other notes.
    pub fn backlinks(&self, note: &str) -> Vec<Backlink> {
        let mut found = Vec::new();
        for (source, from) in self.notes.iter().filter(|(path, _)| *path != note) {
            for link in &from.links {
                if self.resolve(&link.target) == Some(note) {
                    found.push(Backlink {
                        source: source.clone(),
                        context: link.context.clone(),
                    });
                }
            }
        }
        found
    }
}

/// The paths of the notes among `entries`.
fn note_paths(entries: &[Entry], paths: &mut Vec<String>) {
    for entry in entries {
        if entry.kind == EntryKind::Note {
            paths.push(entry.path.clone());
        }
        note_paths(&entry.children, paths);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(name: &str, notes: &[(&str, &str)]) -> Vault {
        let root =
            std::env::temp_dir().join(format!("calliope-index-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let vault = Vault::open(root).unwrap();
        for (path, text) in notes {
            let abs = vault.abs(path);
            fs::create_dir_all(abs.parent().unwrap()).unwrap();
            fs::write(abs, text).unwrap();
        }
        vault
    }

    #[test]
    fn resolves_by_title_nearest_the_root() {
        let v = vault(
            "resolve",
            &[("a/b/笔记.md", ""), ("a/笔记.md", ""), ("c/Other.md", "")],
        );
        let mut index = Index::default();
        assert!(index.sync(&v).unwrap());
        assert_eq!(index.resolve("笔记"), Some("a/笔记.md"));
        assert_eq!(index.resolve("b/笔记"), Some("a/b/笔记.md"));
        assert_eq!(index.resolve("a/b/笔记"), Some("a/b/笔记.md"));
        assert_eq!(index.resolve("Other"), Some("c/Other.md"));
        assert_eq!(index.resolve("x/Other"), None);
        assert_eq!(index.resolve("Missing"), None);
    }

    #[test]
    fn backlinks_follow_the_files() {
        let v = vault("backlinks", &[("A.md", "to [[B]] here"), ("B.md", "")]);
        let mut index = Index::default();
        index.sync(&v).unwrap();
        assert_eq!(
            index.backlinks("B.md"),
            [Backlink {
                source: "A.md".into(),
                context: "to [[B]] here".into()
            }]
        );
        // Nothing changed: nothing is read again.
        assert!(!index.sync(&v).unwrap());

        fs::write(v.abs("A.md"), "no link any more").unwrap();
        fs::write(v.abs("C.md"), "[[B|b]]").unwrap();
        assert!(index.sync(&v).unwrap());
        let sources: Vec<_> = index
            .backlinks("B.md")
            .into_iter()
            .map(|b| b.source)
            .collect();
        assert_eq!(sources, ["C.md"]);

        fs::remove_file(v.abs("C.md")).unwrap();
        assert!(index.sync(&v).unwrap());
        assert!(index.backlinks("B.md").is_empty());
    }

    #[test]
    fn indexes_5000_notes_within_3_seconds() {
        let notes: Vec<(String, String)> = (0..5000)
            .map(|i| {
                let text = format!("# Note {i}\n[[n{}]] and [[n{}]]\n", (i + 1) % 5000, i / 2);
                (format!("d{}/n{i}.md", i % 50), text)
            })
            .collect();
        let refs: Vec<(&str, &str)> = notes
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let v = vault("many", &refs);
        let started = std::time::Instant::now();
        let mut index = Index::default();
        index.sync(&v).unwrap();
        assert_eq!(index.backlinks("d0/n0.md").len(), 2);
        assert!(started.elapsed().as_secs() < 3, "{:?}", started.elapsed());
    }
}
