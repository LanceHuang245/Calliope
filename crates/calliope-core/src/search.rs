//! The full-text search (MVP phase 6): the lines of the notes that hold a
//! query, each with the line's text around the match, as the results show
//! it.

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

/// The longest line searched: a minified or machine-made one holds nothing
/// a reader looks for, and its row would be all punctuation.
const MAX_LINE: usize = 1000;
/// How much of the line a hit's context shows.
const CONTEXT: usize = 120;
/// How much of that comes before the match: the keyword stays in view
/// along with where its line leads.
const LEAD: usize = CONTEXT / 4;

/// The context of `line` if it holds `query`, in lower case.
pub(crate) fn context(line: &str, query: &str) -> Option<String> {
    if line.len() > MAX_LINE {
        return None;
    }
    let line = line.trim();
    Some(window(line, locate(line, query)?))
}

/// Where `query` starts in `line`, ignoring case.
fn locate(line: &str, query: &str) -> Option<usize> {
    let folded = line.to_lowercase();
    let at = folded.find(query)?;
    // Lower casing keeps every character's place, except for a few
    // scripts' letters, which split in two and so move the window a
    // little: count the characters for the offset in the line.
    let index = folded[..at].chars().count();
    Some(
        line.char_indices()
            .nth(index)
            .map_or(line.len(), |(i, _)| i),
    )
}

/// `line` from just before `at` to [`CONTEXT`] characters on, an ellipsis
/// where an end was cut.
fn window(line: &str, at: usize) -> String {
    let start = line[..at]
        .char_indices()
        .rev()
        .nth(LEAD)
        .map_or(0, |(i, _)| i);
    let shown = &line[start..];
    let mut text = String::new();
    if start > 0 {
        text.push('…');
    }
    text.extend(shown.chars().take(CONTEXT));
    if shown.chars().nth(CONTEXT).is_some() {
        text.push('…');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_lines_ignoring_case() {
        assert_eq!(
            context("  Some **Bold** text.  ", "bold"),
            Some("Some **Bold** text.".into())
        );
        // Chinese has no case: the query matches as it is.
        assert_eq!(
            context("笔记里的[[链接]]", "链接"),
            Some("笔记里的[[链接]]".into())
        );
        assert_eq!(context("nothing here", "bold"), None);
        assert_eq!(context("", "bold"), None);
    }

    #[test]
    fn keeps_the_match_in_a_long_lines_context() {
        let line = format!("{} needle {}", "a".repeat(400), "b".repeat(400));
        let found = context(&line, "needle").unwrap();
        assert!(found.contains("needle"), "{found}");
        assert!(found.starts_with('…') && found.ends_with('…'), "{found}");
        // The ellipses on both sides and the line's own text.
        assert_eq!(found.chars().count(), CONTEXT + 2);
        // A line too long to be prose is not searched at all.
        assert_eq!(context(&"a".repeat(MAX_LINE + 1), "a"), None);
    }
}
