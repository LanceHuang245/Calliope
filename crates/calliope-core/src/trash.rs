//! The vault's trash, where there is no system trash (the phones, and
//! sync): a file deleted on a day goes to that day's folder at the path it
//! had (`.trash/2026-11-19/Journal/Day.md`), so the trash tells when it was
//! deleted and where from, and puts it back there. Moving keeps a file's
//! modification time, so only the folder tells the day. Anything else in
//! `.trash/` (Obsidian keeps its own trash there) is left alone.

use std::{fs, io, path::Path};

use jiff::civil::Date;

use crate::vault::{Vault, join};

const TRASH: &str = ".trash";

/// A file in the trash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trashed {
    /// Its path in the vault, inside the trash.
    pub path: String,
    /// The path it had, where restoring puts it back.
    pub origin: String,
    /// The day it was deleted.
    pub deleted: Date,
}

impl Vault {
    /// Moves the file or folder at `rel` into today's folder of the trash,
    /// at the path it had; a file the trash already has there is numbered
    /// (`Day 1.md`).
    pub fn trash(&self, rel: &str) -> io::Result<()> {
        let day = format!("{TRASH}/{}", jiff::Zoned::now().date());
        let path = self.abs(rel);
        if !path.is_dir() {
            return self.move_to(rel, &self.free(&join(&day, rel)));
        }
        // File by file, so each keeps its own path even where the trash has
        // the folder already (a note of it deleted earlier that day).
        let mut files = Vec::new();
        list_files(&path, rel, &mut files)?;
        for file in files {
            self.move_to(&file, &self.free(&join(&day, &file)))?;
        }
        fs::remove_dir_all(path)
    }

    /// Every file in the trash: the latest deleted first, then by the path
    /// they had.
    pub fn trashed(&self) -> io::Result<Vec<Trashed>> {
        let mut trashed = Vec::new();
        for (deleted, day) in self.trash_days() {
            let mut origins = Vec::new();
            list_files(&self.abs(&day), "", &mut origins)?;
            trashed.extend(origins.into_iter().map(|origin| Trashed {
                path: join(&day, &origin),
                origin,
                deleted,
            }));
        }
        trashed.sort_by(|a, b| (b.deleted.cmp(&a.deleted)).then_with(|| a.origin.cmp(&b.origin)));
        Ok(trashed)
    }

    /// Moves `item` back to where it was, making its folder again; numbered
    /// when the name is taken there. Its path.
    pub fn restore(&self, item: &Trashed) -> io::Result<String> {
        let to = self.free(&item.origin);
        self.move_to(&item.path, &to)?;
        Ok(to)
    }

    /// Deletes `item` from the trash for good.
    pub fn delete_forever(&self, item: &Trashed) -> io::Result<()> {
        fs::remove_file(self.abs(&item.path))
    }

    /// Deletes what went to the trash more than `days` days ago: the day
    /// folders before then.
    pub fn empty_trash(&self, days: i64) -> io::Result<()> {
        let oldest = jiff::Zoned::now()
            .date()
            .saturating_sub(jiff::Span::new().days(days));
        for (day, dir) in self.trash_days() {
            if day < oldest {
                fs::remove_dir_all(self.abs(&dir))?;
            }
        }
        Ok(())
    }

    /// Deletes everything the trash keeps: every day folder.
    pub fn clear_trash(&self) -> io::Result<()> {
        for (_, dir) in self.trash_days() {
            fs::remove_dir_all(self.abs(&dir))?;
        }
        Ok(())
    }

    /// The trash's day folders, with their days.
    fn trash_days(&self) -> Vec<(Date, String)> {
        let Ok(entries) = fs::read_dir(self.abs(TRASH)) else {
            return Vec::new();
        };
        (entries.flatten())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                Some((name.parse().ok()?, join(TRASH, &name)))
            })
            .collect()
    }

    /// Moves the file at `from` to `to`, making `to`'s folder.
    fn move_to(&self, from: &str, to: &str) -> io::Result<()> {
        let to = self.abs(to);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir)?;
        }
        fs::rename(self.abs(from), to)
    }
}

/// Adds every file under `dir` to `files`, by its path from there after
/// `rel`.
fn list_files(dir: &Path, rel: &str, files: &mut Vec<String>) -> io::Result<()> {
    for item in fs::read_dir(dir)? {
        let item = item?;
        let path = join(rel, &item.file_name().to_string_lossy());
        match item.file_type()?.is_dir() {
            true => list_files(&item.path(), &path, files)?,
            false => files.push(path),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(name: &str) -> Vault {
        let root =
            std::env::temp_dir().join(format!("calliope-trash-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Vault::open(root).unwrap()
    }

    #[test]
    fn keeps_the_day_and_the_path() {
        let v = vault("keep");
        v.write("Hello.md", "hello").unwrap();
        v.write("Journal/Day.md", "day").unwrap();
        v.write("Ideas/One.md", "one").unwrap();
        v.trash("Hello.md").unwrap();
        v.trash("Journal/Day.md").unwrap();
        // A folder, file by file; the same path again, numbered.
        v.trash("Ideas").unwrap();
        v.write("Hello.md", "again").unwrap();
        v.trash("Hello.md").unwrap();
        assert!(v.scan().unwrap().iter().all(|e| e.name == "Journal"));
        let today = jiff::Zoned::now().date();
        let origins: Vec<_> = (v.trashed().unwrap().into_iter())
            .map(|t| (t.origin, t.deleted))
            .collect();
        let expected = ["Hello 1.md", "Hello.md", "Ideas/One.md", "Journal/Day.md"];
        assert_eq!(origins, expected.map(|o| (o.to_owned(), today)));
        assert_eq!(
            v.read(&format!(".trash/{today}/Journal/Day.md")).unwrap(),
            "day"
        );
    }

    #[test]
    fn restores_where_it_was() {
        let v = vault("restore");
        v.write("Journal/Day.md", "day").unwrap();
        v.trash("Journal").unwrap();
        let item = &v.trashed().unwrap()[0];
        assert_eq!(v.restore(item).unwrap(), "Journal/Day.md");
        assert_eq!(v.read("Journal/Day.md").unwrap(), "day");
        assert!(v.trashed().unwrap().is_empty());
        // A name taken there: numbered.
        v.trash("Journal/Day.md").unwrap();
        v.write("Journal/Day.md", "new").unwrap();
        let item = &v.trashed().unwrap()[0];
        assert_eq!(v.restore(item).unwrap(), "Journal/Day 1.md");
        assert_eq!(v.read("Journal/Day 1.md").unwrap(), "day");
    }

    #[test]
    fn deletes_for_good_and_empties() {
        let v = vault("empty");
        v.write("A.md", "a").unwrap();
        v.write("B.md", "b").unwrap();
        v.trash("A.md").unwrap();
        v.trash("B.md").unwrap();
        v.delete_forever(&v.trashed().unwrap()[0]).unwrap();
        let left: Vec<_> = (v.trashed().unwrap().into_iter())
            .map(|t| t.origin)
            .collect();
        assert_eq!(left, ["B.md"]);
        // Emptying after 15 days keeps today; emptying now keeps only what
        // is not a day folder (Obsidian's own trash).
        v.write(".trash/2020-01-01/Old.md", "old").unwrap();
        v.write(".trash/Obsidian.md", "kept").unwrap();
        v.empty_trash(15).unwrap();
        assert!(!v.abs(".trash/2020-01-01").exists());
        assert_eq!(v.trashed().unwrap().len(), 1);
        v.clear_trash().unwrap();
        assert!(v.trashed().unwrap().is_empty());
        assert!(v.abs(".trash/Obsidian.md").exists());
    }
}
