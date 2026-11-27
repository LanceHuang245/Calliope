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

/// A heading of a note's Markdown: its level (1 for `#`), its text and the
/// line it is on (from 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub line: usize,
}

/// The headings of a note's Markdown, in order: `#` to `######` with a space
/// after them, outside the front matter and the code.
pub fn outline(text: &str) -> Vec<Heading> {
    let (front, body) = split_front_matter(text);
    // The body starts after the `---` lines the front matter is between.
    let offset = front.map_or(0, |front| front.lines().count() + 2);
    let mut headings = Vec::new();
    let mut fenced = false;
    for (index, line) in body.lines().enumerate() {
        let line = line.trim_end();
        if line.starts_with("```") || line.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        let level = line.chars().take_while(|c| *c == '#').count();
        // Seven are no heading, and one starts in the first column.
        if fenced || level == 0 || level > 6 {
            continue;
        }
        let Some(text) = line[level..].strip_prefix(' ') else {
            continue;
        };
        // A closing run of `#` marks the end, as in `## Title ##`.
        let text = text.trim_end_matches('#').trim();
        if !text.is_empty() {
            headings.push(Heading {
                level: level as u8,
                text: text.to_owned(),
                line: offset + index + 1,
            });
        }
    }
    headings
}

/// How many characters a note holds, as the status bar counts them: its
/// body (the front matter is not text the reader sees), whitespace aside —
/// a space and a line break are no character of either language's text.
pub fn characters(text: &str) -> usize {
    let (_, body) = split_front_matter(text);
    body.chars().filter(|c| !c.is_whitespace()).count()
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
    fn lists_headings_with_their_levels_and_lines() {
        let text = "---\ntags: [a]\n---\n# One\n\ntext\n## Two ##\n###\n####### Seven\n```\n# Code\n```\n#NoSpace\n# 中文标题\n";
        assert_eq!(
            outline(text),
            [
                Heading {
                    level: 1,
                    text: "One".to_owned(),
                    line: 4
                },
                Heading {
                    level: 2,
                    text: "Two".to_owned(),
                    line: 7
                },
                Heading {
                    level: 1,
                    text: "中文标题".to_owned(),
                    line: 14
                },
            ]
        );
        // A note without front matter counts from its first line.
        assert_eq!(outline("# Hi\n")[0].line, 1);
    }

    #[test]
    fn counts_characters_of_both_languages() {
        // A Chinese character, a Latin letter and an emoji are one each.
        assert_eq!(characters("中文😀 abc"), 6);
        // Spaces, tabs and line breaks are none, and the front matter is not
        // the note's text.
        assert_eq!(characters("---\ntags: [a]\n---\none\ttwo\nthree\n"), 11);
        assert_eq!(characters(""), 0);
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
