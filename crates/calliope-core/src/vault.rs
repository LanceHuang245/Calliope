//! A vault: a folder of notes (`*.md`) and attachments (images, PDFs), with
//! Calliope's own settings in `.calliope/`. Opening, listing, reading,
//! writing and renaming the files.
//!
//! Paths inside a vault are relative, `/`-separated strings (`"a/b.md"`):
//! the same on every platform, and usable as ids by the UI.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "svg"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Folder,
    Note,
    Image,
    Other,
}

impl EntryKind {
    pub fn of_file(path: &str) -> Self {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if ext == "md" {
            EntryKind::Note
        } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            EntryKind::Image
        } else {
            EntryKind::Other
        }
    }
}

/// A file or folder of the vault, with its children for a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The path relative to the vault's root, `/`-separated.
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    pub children: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Opens the folder at `root` as a vault, creating it when missing:
    /// opening and creating a vault are one.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn name(&self) -> String {
        self.root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    pub fn abs(&self, rel: &str) -> PathBuf {
        rel.split('/')
            .filter(|p| !p.is_empty())
            .fold(self.root.clone(), |path, part| path.join(part))
    }

    /// Every file and folder, folders first then by name; names starting
    /// with `.` (`.git/`, `.calliope/`, `.trash/`) are left out.
    pub fn scan(&self) -> io::Result<Vec<Entry>> {
        scan_dir(&self.root, "")
    }

    pub fn read(&self, rel: &str) -> io::Result<String> {
        fs::read_to_string(self.abs(rel))
    }

    /// Writes `text` to `rel` atomically: a hidden temporary file next to
    /// it, flushed to disk, then renamed over it, so a crash leaves either
    /// the old file or the new one.
    pub fn write(&self, rel: &str, text: &str) -> io::Result<()> {
        let path = self.abs(rel);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let tmp = path.with_file_name(format!(".{name}.calliope-tmp"));
        {
            let mut file = fs::File::create(&tmp)?;
            io::Write::write_all(&mut file, text.as_bytes())?;
            file.sync_all()?;
        }
        fs::rename(&tmp, &path)
    }

    /// Creates an empty note in folder `dir` (`""` for the root) named
    /// `Untitled.md`, or `Untitled 1.md`, ... when taken; its path.
    pub fn create_note(&self, dir: &str) -> io::Result<String> {
        let rel = self.unique(dir, "Untitled", ".md");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.abs(&rel))?;
        Ok(rel)
    }

    /// Creates a folder in `dir` named `New folder` (numbered when taken);
    /// its path.
    pub fn create_folder(&self, dir: &str) -> io::Result<String> {
        let rel = self.unique(dir, "New folder", "");
        fs::create_dir(self.abs(&rel))?;
        Ok(rel)
    }

    /// Writes `text` as a new note next to `rel`, named after it with
    /// ` (conflict)` (numbered when taken); its path.
    pub fn save_copy(&self, rel: &str, text: &str) -> io::Result<String> {
        let (dir, name) = split(rel);
        let stem = name.strip_suffix(".md").unwrap_or(name);
        let copy = self.unique(dir, &format!("{stem} (conflict)"), ".md");
        self.write(&copy, text)?;
        Ok(copy)
    }

    /// Renames the file or folder at `rel` to `name` in the same folder; a
    /// note keeps its `.md` when `name` leaves it out. Its new path; an
    /// `AlreadyExists` error when the name is taken.
    pub fn rename(&self, rel: &str, name: &str) -> io::Result<String> {
        let (dir, old) = split(rel);
        let mut name = name.trim().to_owned();
        if old.ends_with(".md") && !name.ends_with(".md") {
            name.push_str(".md");
        }
        let new = join(dir, &name);
        if new == rel {
            return Ok(new);
        }
        if self.abs(&new).exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{name} already exists"),
            ));
        }
        fs::rename(self.abs(rel), self.abs(&new))?;
        Ok(new)
    }

    /// Moves the file or folder at `rel` into the vault's `.trash/` (where
    /// there is no system trash: the phones).
    pub fn trash(&self, rel: &str) -> io::Result<()> {
        fs::create_dir_all(self.root.join(".trash"))?;
        let target = self.unique(".trash", split(rel).1, "");
        fs::rename(self.abs(rel), self.abs(&target))
    }

    /// `dir/stem ext`, or `dir/stem N ext` with the first N free.
    fn unique(&self, dir: &str, stem: &str, ext: &str) -> String {
        (0..)
            .map(|n| match n {
                0 => join(dir, &format!("{stem}{ext}")),
                n => join(dir, &format!("{stem} {n}{ext}")),
            })
            .find(|rel| !self.abs(rel).exists())
            .expect("a free name")
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

fn split(rel: &str) -> (&str, &str) {
    rel.rsplit_once('/').unwrap_or(("", rel))
}

fn scan_dir(dir: &Path, rel: &str) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for item in fs::read_dir(dir)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let path = join(rel, &name);
        let entry = if item.file_type()?.is_dir() {
            Entry {
                children: scan_dir(&item.path(), &path)?,
                kind: EntryKind::Folder,
                path,
                name,
            }
        } else {
            Entry {
                kind: EntryKind::of_file(&path),
                children: Vec::new(),
                path,
                name,
            }
        };
        entries.push(entry);
    }
    entries.sort_by_cached_key(|e| (e.kind != EntryKind::Folder, e.name.to_lowercase()));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(name: &str) -> Vault {
        let root =
            std::env::temp_dir().join(format!("calliope-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Vault::open(root).unwrap()
    }

    #[test]
    fn scans_folders_first_and_skips_dot_files() {
        let v = vault("scan");
        fs::create_dir_all(v.abs("b/.git")).unwrap();
        fs::write(v.abs("a.md"), "").unwrap();
        fs::write(v.abs("b/c.png"), "").unwrap();
        fs::write(v.abs(".hidden.md"), "").unwrap();
        let entries = v.scan().unwrap();
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["b", "a.md"]);
        assert_eq!(entries[0].children.len(), 1);
        assert_eq!(entries[0].children[0].path, "b/c.png");
        assert_eq!(entries[0].children[0].kind, EntryKind::Image);
    }

    #[test]
    fn writes_reads_and_names_new_notes() {
        let v = vault("write");
        let first = v.create_note("").unwrap();
        let second = v.create_note("").unwrap();
        assert_eq!(
            (first.as_str(), second.as_str()),
            ("Untitled.md", "Untitled 1.md")
        );
        v.write(&first, "# 你好").unwrap();
        assert_eq!(v.read(&first).unwrap(), "# 你好");
        // No temporary file is left behind.
        assert_eq!(v.scan().unwrap().len(), 2);
    }

    #[test]
    fn renames_keeping_the_extension_and_refuses_taken_names() {
        let v = vault("rename");
        let dir = v.create_folder("").unwrap();
        let note = v.create_note(&dir).unwrap();
        let renamed = v.rename(&note, "Ideas").unwrap();
        assert_eq!(renamed, "New folder/Ideas.md");
        let other = v.create_note(&dir).unwrap();
        let err = v.rename(&other, "Ideas").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn trashes_into_the_vault_and_copies_conflicts() {
        let v = vault("trash");
        let note = v.create_note("").unwrap();
        v.trash(&note).unwrap();
        assert!(v.scan().unwrap().is_empty());
        assert!(v.abs(".trash/Untitled.md").exists());
        let copy = v.save_copy("Plan.md", "mine").unwrap();
        assert_eq!(copy, "Plan (conflict).md");
        assert_eq!(v.read(&copy).unwrap(), "mine");
    }
}
