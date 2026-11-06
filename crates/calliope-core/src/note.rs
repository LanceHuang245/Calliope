//! A note: its path in the vault, its front matter, title and Markdown body.

use std::path::{Path, PathBuf};

/// A note's title: its file name without `.md`.
pub fn title(rel: &str) -> &str {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    name.strip_suffix(".md").unwrap_or(name)
}

/// Splits a note into its YAML front matter (between `---` lines at the
/// very top) and the Markdown after it; `None` when it has none.
pub fn split_front_matter(text: &str) -> (Option<&str>, &str) {
    let Some(rest) = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))
    else {
        return (None, text);
    };
    // The closing `---` on a line of its own.
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (Some(&rest[..offset]), &rest[offset + line.len()..]);
        }
        offset += line.len();
    }
    (None, text)
}

/// The file a note's link or image `src` points at, relative to the
/// folder of the note at `note` (absolute): URL-escaped (`my%20image.png`)
/// or not (`my image.png`, `图片.png`). `None` for a URL with a scheme
/// (`https:`, `data:`), which is not a file of the vault.
pub fn resolve_file(note: &Path, src: &str) -> Option<PathBuf> {
    if url::Url::parse(src).is_ok() {
        return None;
    }
    let base = url::Url::from_file_path(note).ok()?;
    base.join(src).ok()?.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_file_names() {
        assert_eq!(title("a/b/读书笔记.md"), "读书笔记");
        assert_eq!(title("Note.md"), "Note");
    }

    #[test]
    fn resolves_files_next_to_the_note() {
        let note = std::env::temp_dir().join("vault").join("dir").join("a.md");
        let dir = note.parent().unwrap();
        assert_eq!(
            resolve_file(&note, "图片 1.png"),
            Some(dir.join("图片 1.png"))
        );
        assert_eq!(
            resolve_file(&note, "img/my%20cat.png"),
            Some(dir.join("img").join("my cat.png"))
        );
        assert_eq!(
            resolve_file(&note, "../b.png"),
            Some(dir.parent().unwrap().join("b.png"))
        );
        assert_eq!(resolve_file(&note, "https://example.com/a.png"), None);
    }

    #[test]
    fn splits_front_matter() {
        assert_eq!(
            split_front_matter("---\ntags: [a]\n---\n# Hi\n"),
            (Some("tags: [a]\n"), "# Hi\n")
        );
        assert_eq!(split_front_matter("# Hi\n---\n"), (None, "# Hi\n---\n"));
        // Unclosed: no front matter.
        assert_eq!(split_front_matter("---\nx\n"), (None, "---\nx\n"));
    }
}
