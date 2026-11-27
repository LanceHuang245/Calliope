//! Run Harper on the Markdown body and locate its findings in the source editor.

use std::ops::Range;

use calliope_core::note::split_front_matter;
use harper_core::{
    Dialect, Document, Token, TokenKind,
    linting::{LintGroup, Linter, Suggestion},
    parsers::{Markdown, Parser as HarperParser},
    spell::FstDictionary,
};
use nsh_base::editor::{
    EditorDiagnostic, EditorDiagnosticSeverity, EditorEdit, EditorPosition, EditorRange,
};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

pub(super) struct Issue {
    pub(super) mark: EditorDiagnostic,
    pub(super) fixes: Vec<(String, EditorEdit)>,
}

pub(super) fn checker() -> LintGroup {
    LintGroup::new_curated(FstDictionary::curated(), Dialect::American)
}

pub(super) fn check(linter: &mut LintGroup, source: &str) -> Vec<Issue> {
    let (_, body) = split_front_matter(source);
    let prefix = &source[..source.len() - body.len()];
    let positions = positions(prefix, body);
    let (prose, wiki_targets) = prose_ranges(body);
    let doc = Document::new_curated(
        body,
        &MarkdownProse {
            positions: &positions,
            wiki_targets,
        },
    );
    let urls: Vec<_> = doc
        .tokens()
        .filter(|t| {
            matches!(
                t.kind,
                TokenKind::Url | TokenKind::Hostname | TokenKind::EmailAddress
            )
        })
        .filter_map(|t| Some(positions.get(t.span.start)?.0..positions.get(t.span.end)?.0))
        .collect();

    let mut issues: Vec<_> = linter
        .lint(&doc)
        .into_iter()
        .filter_map(|lint| {
            let &(start_byte, start) = positions.get(lint.span.start)?;
            let &(end_byte, end) = positions.get(lint.span.end)?;
            let first = containing(&prose, start_byte)?;
            let last = containing(
                &prose,
                end_byte.saturating_sub(usize::from(start_byte < end_byte)),
            )?;
            if urls
                .get(urls.partition_point(|r| r.end <= start_byte))
                .is_some_and(|r| r.start < end_byte.max(start_byte + 1))
            {
                return None;
            }

            let range = EditorRange::new(start, end);
            let mut mark = EditorDiagnostic::default();
            mark.range = range;
            mark.severity = EditorDiagnosticSeverity::Warning;
            mark.message = lint.message.into();
            mark.source = Some("Harper".into());
            mark.code = Some(lint.lint_kind.to_string().into());

            // A suggestion can edit only one contiguous run of visible source.
            let fixes = if first == last {
                lint.suggestions
                    .into_iter()
                    .map(|suggestion| {
                        let label = suggestion.to_string();
                        let edit = match suggestion {
                            Suggestion::ReplaceWith(chars) => {
                                EditorEdit::new(range, chars.into_iter().collect::<String>())
                            }
                            Suggestion::InsertAfter(chars) => EditorEdit::new(
                                EditorRange::new(end, end),
                                chars.into_iter().collect::<String>(),
                            ),
                            Suggestion::Remove => EditorEdit::new(range, ""),
                        };
                        (label, edit)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Some(Issue { mark, fixes })
        })
        .collect();
    issues.sort_by_key(|issue| issue.mark.range.start);
    issues
}

struct MarkdownProse<'a> {
    positions: &'a [(usize, EditorPosition)],
    wiki_targets: Vec<Range<usize>>,
}

impl HarperParser for MarkdownProse<'_> {
    fn parse(&self, source: &[char]) -> Vec<Token> {
        let mut tokens = Markdown::default().parse(source);
        // Harper already skips an aliased link's target. Ignore unaliased
        // targets as context too, so they cannot cause a lint next to the link.
        for token in &mut tokens {
            if !token.span.is_empty()
                && let Some((start, _)) = self.positions.get(token.span.start)
                && containing(&self.wiki_targets, *start).is_some()
            {
                token.kind = TokenKind::Unlintable;
            }
        }
        tokens
    }
}

// Use Markdown source offsets to keep URLs and wiki destinations out of edits,
// including a wiki link whose only text is its destination.
fn prose_ranges(body: &str) -> (Vec<Range<usize>>, Vec<Range<usize>>) {
    let options = Options::all().difference(Options::ENABLE_SMART_PUNCTUATION);
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut wiki_targets = Vec::new();
    let mut hidden = false;
    let mut code = false;
    for (event, range) in Parser::new_ext(body, options).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => code = true,
            Event::End(TagEnd::CodeBlock) => code = false,
            Event::Start(Tag::Link { link_type, .. }) => {
                if matches!(link_type, LinkType::WikiLink { has_pothole: false }) {
                    wiki_targets.push(range);
                }
                hidden = matches!(
                    link_type,
                    LinkType::Autolink
                        | LinkType::Email
                        | LinkType::WikiLink { has_pothole: false }
                );
            }
            Event::End(TagEnd::Link) => hidden = false,
            Event::Text(text) if !hidden && !code && text.as_ref() == &body[range.clone()] => {
                if let Some(last) = ranges.last_mut()
                    && last.end == range.start
                {
                    last.end = range.end;
                } else {
                    ranges.push(range);
                }
            }
            _ => {}
        }
    }
    (ranges, wiki_targets)
}

fn containing(ranges: &[Range<usize>], offset: usize) -> Option<usize> {
    let index = ranges.partition_point(|r| r.end <= offset);
    ranges
        .get(index)
        .filter(|r| r.contains(&offset))
        .map(|_| index)
}

fn positions(prefix: &str, body: &str) -> Vec<(usize, EditorPosition)> {
    let mut line = prefix.bytes().filter(|&b| b == b'\n').count() as u32;
    let mut column = 0;
    let mut offsets = Vec::new();
    // Harper counts Unicode scalar values. nsh counts UTF-16 units per line.
    for (byte, ch) in body.char_indices() {
        offsets.push((byte, EditorPosition::new(line, column)));
        match ch {
            '\n' => {
                line += 1;
                column = 0;
            }
            '\r' if body.as_bytes().get(byte + 1) == Some(&b'\n') => {}
            _ => column += ch.len_utf16() as u32,
        }
    }
    offsets.push((body.len(), EditorPosition::new(line, column)));
    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_markdown_metadata_and_destinations_but_checks_link_labels() {
        let source = concat!(
            "---\nsummary: an apple apple\n---\n",
            "```\nThis is an test.\n```\n\n",
            "`This is an test.`\n\n$This is an test.$\n\n",
            "https://example.org/teh <https://example.org/teh>\n\n",
            "[[This is an test]]\n\n",
            "[This is an test](https://example.org/teh).\n\n",
            "[[This is an test|This is an test]].\n",
        );
        let issues = check(&mut checker(), source);
        assert!(!issues.is_empty());
        assert!(
            issues
                .iter()
                .all(|issue| matches!(issue.mark.range.start.line, 15 | 17))
        );
        for line in [15, 17] {
            assert!(issues.iter().any(|issue| {
                issue.mark.range.start.line == line
                    && issue
                        .fixes
                        .iter()
                        .any(|(_, edit)| edit.text.as_str() == "a")
            }));
        }
    }

    #[test]
    fn a_wiki_target_cannot_trigger_a_fix_in_nearby_prose() {
        let issues = check(&mut checker(), "This is an [[test]].");
        assert!(!issues.iter().any(|issue| {
            issue
                .fixes
                .iter()
                .any(|(_, edit)| edit.text.as_str() == "a")
        }));
    }

    #[test]
    fn maps_a_suggestion_after_chinese_and_emoji() {
        let source = "---\r\ntitle: Test\r\n---\r\n中文😀 This is an test.";
        let issues = check(&mut checker(), source);
        let edit = issues
            .iter()
            .flat_map(|issue| &issue.fixes)
            .map(|(_, edit)| edit)
            .find(|edit| edit.text.as_str() == "a")
            .unwrap();
        assert_eq!(edit.range.start, EditorPosition::new(3, 13));
        assert_eq!(edit.range.end, EditorPosition::new(3, 15));
    }
}
