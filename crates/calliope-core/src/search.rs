//! The full-text search (MVP phase 6): the lines of the notes that hold a
//! query, each with the line's text around the match, as the results show
//! it.

use std::io::{self, BufRead, Read};

/// A line of a note that holds the query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// The note's path in the vault.
    pub path: String,
    /// The line, counted from 1.
    pub line: u32,
    /// The line's text around the match, short enough for a result's row.
    pub context: String,
}

/// The maximum number of characters in a searchable line.
const MAX_LINE: usize = 1000;
/// How much of the line a hit's context shows.
const CONTEXT: usize = 120;
/// How much of that comes before the match: the keyword stays in view
/// along with where its line leads.
const LEAD: usize = CONTEXT / 4;

/// Read one disk line without allocating for text that cannot be searched.
pub(crate) fn read_line(reader: &mut impl BufRead, line: &mut Vec<u8>) -> io::Result<usize> {
    const MAX_BYTES: usize = MAX_LINE * 4 + 2;
    line.clear();
    let read = (&mut *reader)
        .take(MAX_BYTES as u64)
        .read_until(b'\n', line)?;
    if read == MAX_BYTES && !line.ends_with(b"\n") {
        reader.skip_until(b'\n')?;
        line.clear();
    }
    Ok(read)
}

/// State reused across all lines in a query.
pub(crate) struct Search {
    query: String,
    folded: String,
    limit: usize,
    pub hits: Vec<Hit>,
}

impl Search {
    pub(crate) fn new(query: &str, limit: usize) -> Option<Self> {
        let query = query.trim();
        (!query.is_empty() && limit > 0).then(|| Self {
            query: query.to_lowercase(),
            folded: String::new(),
            limit,
            hits: Vec::new(),
        })
    }

    /// Scan ordinary ASCII notes together before inspecting individual lines.
    pub(crate) fn may_match(&mut self, text: &str) -> bool {
        if text.len() > 64 * 1024 || !text.is_ascii() {
            return true;
        }
        self.folded.clear();
        self.folded.push_str(text);
        self.folded.make_ascii_lowercase();
        self.folded.contains(&self.query)
    }

    /// Adds a matching line and returns whether the result limit was reached.
    pub(crate) fn add(&mut self, path: &str, number: u32, line: &str) -> bool {
        if let Some(context) = context(line, &self.query, &mut self.folded) {
            self.hits.push(Hit {
                path: path.to_owned(),
                line: number,
                context,
            });
        }
        self.hits.len() == self.limit
    }
}

/// The context of `line` if it holds a lowercase query.
fn context(line: &str, query: &str, folded: &mut String) -> Option<String> {
    // Short lines cannot exceed the character limit even in UTF-8.
    if line.len() > MAX_LINE && line.chars().nth(MAX_LINE).is_some() {
        return None;
    }
    let line = line.trim();
    Some(window(line, locate(line, query, folded)?))
}

/// Where `query` starts in `line`, ignoring case.
fn locate(line: &str, query: &str, folded: &mut String) -> Option<usize> {
    if line.is_ascii() {
        if !line.bytes().any(|byte| byte.is_ascii_uppercase()) {
            return line.find(query);
        }
        folded.clear();
        folded.push_str(line);
        folded.make_ascii_lowercase();
        return folded.find(query);
    }
    // Keep contextual Unicode casing and map expansions back to the source.
    if line.chars().all(|ch| ch.to_lowercase().eq([ch])) {
        return line.find(query);
    }
    let folded = line.to_lowercase();
    let at = folded.find(query)?;
    let mut end = 0;
    line.char_indices().find_map(|(index, ch)| {
        end += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
        (at < end).then_some(index)
    })
}

/// `line` from just before `at` to [`CONTEXT`] characters on, an ellipsis
/// where an end was cut.
fn window(line: &str, at: usize) -> String {
    let start = line[..at]
        .char_indices()
        .rev()
        .nth(LEAD - 1)
        .map_or(0, |(i, _)| i);
    let shown = &line[start..];
    let mut text = String::new();
    if start > 0 {
        text.push('…');
    }
    let room = CONTEXT - usize::from(start > 0);
    if shown.chars().nth(room).is_some() {
        text.extend(shown.chars().take(room - 1));
        text.push('…');
    } else {
        text.push_str(shown);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snippet(line: &str, query: &str) -> Option<String> {
        context(line, query, &mut String::new())
    }

    #[test]
    fn finds_lines_ignoring_case() {
        assert_eq!(
            snippet("  Some **Bold** text.  ", "bold"),
            Some("Some **Bold** text.".into())
        );
        // Chinese has no case: the query matches as it is.
        assert_eq!(
            snippet("笔记里的[[链接]]", "链接"),
            Some("笔记里的[[链接]]".into())
        );
        assert_eq!(snippet("nothing here", "bold"), None);
        assert_eq!(snippet("", "bold"), None);
    }

    #[test]
    fn keeps_the_match_in_a_long_lines_context() {
        let line = format!("{} needle {}", "a".repeat(400), "b".repeat(400));
        let found = snippet(&line, "needle").unwrap();
        assert!(found.contains("needle"), "{found}");
        assert!(found.starts_with('…') && found.ends_with('…'), "{found}");
        // The ellipses on both sides and the line's own text.
        assert_eq!(found.chars().count(), CONTEXT);
        // A line too long to be prose is not searched at all.
        assert_eq!(snippet(&"a".repeat(MAX_LINE + 1), "a"), None);
        let line = format!("{}针", "字".repeat(MAX_LINE - 1));
        assert!(snippet(&line, "针").unwrap().ends_with('针'));
        assert_eq!(snippet(&(line + "字"), "针"), None);
    }

    #[test]
    fn maps_case_expansions_to_the_source() {
        let line = format!("{} needle {}", "İ".repeat(200), "字".repeat(200));
        assert!(snippet(&line, "needle").unwrap().contains("needle"));
    }
}
