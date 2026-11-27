//! The vault's link index: each note's links, what they resolve to and the
//! backlinks of each note, kept up to date as files change. It keeps the
//! notes' text too, which the full-text search reads ([`Index::search`]).

use std::{
    collections::{BTreeMap, HashMap},
    io::{self, BufReader, Read},
    sync::Arc,
    time::SystemTime,
};

use crate::{
    link::{self, Link},
    note,
    search::{self, Hit, Search},
    vault::{Entry, EntryKind, Vault, open_regular},
};

/// Keep ordinary vaults searchable from memory; read uncached notes as needed.
const TEXT_BUDGET: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
struct Note {
    /// The file's modification time and size when its links were read.
    stamp: (SystemTime, u64),
    links: Vec<Link>,
    /// What the note holds, for the full-text search.
    text: Option<Box<str>>,
}

/// A link to a note from another: the note it is in and its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    pub source: String,
    pub context: String,
}

#[derive(Debug, Clone, Default)]
pub struct Index {
    vault: Option<Vault>,
    /// The notes by path (`Arc`-wrapped so cloning the index is cheap).
    notes: BTreeMap<String, Arc<Note>>,
    /// The paths of the notes of each title.
    titles: HashMap<String, Vec<String>>,
    /// Precomputed backlinks keyed by target note path for O(1) lookup.
    backlinks: HashMap<String, Vec<Backlink>>,
}

impl Index {
    /// Brings the index up to date with the vault's files, reading only
    /// the notes that are new or changed; whether anything changed.
    pub fn sync(&mut self, vault: &Vault) -> io::Result<bool> {
        let root = vault.directory();
        let mut paths = Vec::new();
        note_paths(&vault.scan()?, &mut paths);
        let mut remaining = TEXT_BUDGET;
        let mut old = std::mem::take(&mut self.notes);
        let mut changed = false;
        if self
            .vault
            .as_ref()
            .is_some_and(|previous| !std::ptr::eq(previous.directory(), root))
        {
            changed = !old.is_empty();
            old.clear();
        }
        for path in paths {
            let Ok(metadata) = root.symlink_metadata(&path) else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let stamp = (modified.into_std(), metadata.len());
            let note = match old.remove(&path) {
                Some(note) if note.stamp == stamp => {
                    if note
                        .text
                        .as_ref()
                        .is_some_and(|text| text.len() > remaining)
                    {
                        Arc::new(Note {
                            stamp,
                            links: note.links.clone(),
                            text: None,
                        })
                    } else {
                        note
                    }
                }
                previous => {
                    let mut text = String::new();
                    if open_regular(root, &path)
                        .and_then(|mut file| file.read_to_string(&mut text))
                        .is_err()
                    {
                        changed |= previous.is_some();
                        continue;
                    }
                    changed = true;
                    let links = link::links(&text);
                    let text = (text.len() <= remaining).then(|| text.into_boxed_str());
                    Arc::new(Note { stamp, links, text })
                }
            };
            remaining -= note.text.as_ref().map_or(0, |text| text.len());
            self.notes.insert(path, note);
        }
        self.vault = Some(vault.clone());
        // What is left of the old notes was deleted.
        changed |= !old.is_empty();
        if changed {
            self.titles.clear();
            for path in self.notes.keys() {
                let title = note::title(path).to_owned();
                self.titles.entry(title).or_default().push(path.clone());
            }
            // Rebuild the reverse index once per change so backlinks lookup is O(1).
            let mut backlinks: HashMap<String, Vec<Backlink>> = HashMap::new();
            for (source, from) in &self.notes {
                for link in &from.links {
                    if let Some(target) = self.resolve(&link.target)
                        && target != source
                    {
                        backlinks
                            .entry(target.to_owned())
                            .or_default()
                            .push(Backlink {
                                source: source.clone(),
                                context: link.context.clone(),
                            });
                    }
                }
            }
            self.backlinks = backlinks;
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
        self.backlinks.get(note).cloned().unwrap_or_default()
    }

    /// The lines of the notes that hold `query`, by note path, at most
    /// `limit` of them.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        self.search_until(query, limit, || false)
    }

    /// Searches a snapshot, stopping early if the caller no longer needs it.
    pub fn search_until(
        &self,
        query: &str,
        limit: usize,
        cancelled: impl Fn() -> bool,
    ) -> Vec<Hit> {
        let (Some(mut search), Some(vault)) = (Search::new(query, limit), &self.vault) else {
            return Vec::new();
        };
        let mut line = Vec::new();
        for (path, note) in &self.notes {
            if cancelled() {
                break;
            }
            if let Some(text) = &note.text {
                if !search.may_match(text) {
                    continue;
                }
                for (index, line) in text.lines().enumerate() {
                    if cancelled() || search.add(path, index as u32 + 1, line) {
                        return search.hits;
                    }
                }
            } else if let Ok(file) = open_regular(vault.directory(), path) {
                let mut reader = BufReader::new(file);
                let mut number = 1;
                // Reuse the capped line buffer for notes outside the cache.
                while !cancelled()
                    && search::read_line(&mut reader, &mut line).is_ok_and(|len| len > 0)
                {
                    if let Ok(line) = str::from_utf8(&line)
                        && let Some(text) = line.lines().next()
                        && search.add(path, number, text)
                    {
                        return search.hits;
                    }
                    number += 1;
                }
            }
        }
        search.hits
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
    use std::fs;

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
    fn finds_the_lines_that_hold_a_query() {
        let v = vault(
            "search",
            &[
                ("One.md", "# Notes\n\nThe Quick brown fox\n"),
                ("Folder/Two.md", "第一行\n这里提到链接和别的\n"),
            ],
        );
        let mut index = Index::default();
        index.sync(&v).unwrap();
        assert_eq!(
            index.search("quick", 10),
            [Hit {
                path: "One.md".into(),
                line: 3,
                context: "The Quick brown fox".into()
            }]
        );
        assert_eq!(index.search(" 链接 ", 10)[0].line, 2);
        assert_eq!(index.search("链接", 10)[0].path, "Folder/Two.md");
        assert!(index.search("fox", 1).len() == 1);
        assert!(index.search("missing", 10).is_empty());
        assert!(index.search("   ", 10).is_empty());

        // An edited note is read again: its old lines are gone.
        fs::write(v.abs("One.md"), "nothing to see\n").unwrap();
        index.sync(&v).unwrap();
        assert!(index.search("quick", 10).is_empty());
    }
    #[test]
    fn searches_notes_that_do_not_fit_in_the_cache() {
        let text = format!("{}\nNeedle\n", "x".repeat(TEXT_BUDGET));
        let vault = vault("uncached", &[("Large.md", &text)]);
        let mut index = Index::default();
        index.sync(&vault).unwrap();

        assert!(index.notes["Large.md"].text.is_none());
        assert_eq!(index.search("needle", 1)[0].line, 2);
        assert!(index.search("needle", 0).is_empty());
        assert!(index.search_until("needle", 1, || true).is_empty());
        drop(index);
        let root = vault.root().to_owned();
        drop(vault);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn excludes_file_symlinks_from_the_index() {
        let target = vault("linked-target", &[("Target.md", "only in the other vault")]);
        let vault = vault("linked-file", &[("Own.md", "local note")]);
        std::os::unix::fs::symlink(target.abs("Target.md"), vault.abs("Linked.md")).unwrap();
        let mut index = Index::default();
        index.sync(&vault).unwrap();

        assert_eq!(index.notes().collect::<Vec<_>>(), ["Own.md"]);
        assert!(index.search("other vault", 1).is_empty());
        assert!(vault.read("Linked.md").is_err());
        fs::remove_dir_all(vault.root()).unwrap();
        fs::remove_dir_all(target.root()).unwrap();
    }
}
