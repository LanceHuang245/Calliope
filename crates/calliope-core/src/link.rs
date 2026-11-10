//! The wiki links of a note's Markdown: `[[Note]]`, `[[Note|alias]]`,
//! `[[Note#Heading]]` and `[[Folder/Note]]`.

/// A wiki link as written in a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// The note it names: a title, or a path ending in one (`Folder/Note`).
    pub target: String,
    pub heading: Option<String>,
    pub alias: Option<String>,
    /// The line it is on, trimmed: what a backlink shows around it.
    pub context: String,
}

/// The parts of what is between `[[` and `]]`: the target, the heading
/// after `#` and the alias after `|`. `None` for an empty target.
pub fn parse(inner: &str) -> Option<(&str, Option<&str>, Option<&str>)> {
    let (rest, alias) = match inner.split_once('|') {
        Some((rest, alias)) => (rest, Some(alias.trim())),
        None => (inner, None),
    };
    let (target, heading) = match rest.split_once('#') {
        Some((target, heading)) => (target, Some(heading.trim())),
        None => (rest, None),
    };
    let target = target.trim();
    (!target.is_empty()).then_some((target, heading, alias))
}

/// The wiki links of `text`, in order. Code (fenced blocks and inline
/// spans) holds none.
pub fn links(text: &str) -> Vec<Link> {
    let mut found = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        // Outside inline code: every other piece between backticks.
        for prose in line.split('`').step_by(2) {
            let mut rest = prose;
            while let Some(open) = rest.find("[[") {
                rest = &rest[open + 2..];
                let Some(close) = rest.find("]]") else {
                    break;
                };
                if let Some((target, heading, alias)) = parse(&rest[..close]) {
                    found.push(Link {
                        target: target.to_owned(),
                        heading: heading.map(str::to_owned),
                        alias: alias.map(str::to_owned),
                        context: trimmed.to_owned(),
                    });
                }
                rest = &rest[close + 2..];
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn targets(text: &str) -> Vec<String> {
        links(text).into_iter().map(|l| l.target).collect()
    }

    #[test]
    fn reads_every_form() {
        let found = links("See [[笔记]], [[B|the b]] and [[C#Intro]].\n[[dir/D#h|d]]");
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].target, "笔记");
        assert_eq!(
            found[0].context,
            "See [[笔记]], [[B|the b]] and [[C#Intro]]."
        );
        assert_eq!(found[1].alias.as_deref(), Some("the b"));
        assert_eq!(found[2].heading.as_deref(), Some("Intro"));
        assert_eq!(
            (
                found[3].target.as_str(),
                found[3].heading.as_deref(),
                found[3].alias.as_deref()
            ),
            ("dir/D", Some("h"), Some("d"))
        );
    }

    #[test]
    fn code_holds_no_links() {
        assert_eq!(targets("`[[A]]` [[B]]\n```\n[[C]]\n```\n[[D]]"), ["B", "D"]);
    }

    #[test]
    fn skips_empty_and_unclosed() {
        assert_eq!(targets("[[]] [[ |x]] [[A"), Vec::<String>::new());
    }
}
