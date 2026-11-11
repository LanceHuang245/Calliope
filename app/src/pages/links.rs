//! Links between notes: the wiki links of the rendered note and the
//! backlinks of the open note.

use calliope_core::{index::Backlink, link, note};
use nsh_base::{MarkdownExtension, MarkdownPlugin, text_view::CustomNode, theme};
use nsh_core::prelude::*;

use crate::{store, strings};

/// The name of the custom nodes wiki links are read as.
const WIKI_LINK: &str = "wikilink";

/// Reads `[[…]]` in a note's text as a wiki link, outside code.
pub(crate) fn wiki_links() -> MarkdownExtension {
    MarkdownExtension::pattern(WIKI_LINK, r"\[\[([^\[\]]+)\]\]")
}

/// Shows the wiki links as text of their line: a custom node shown by the
/// text view's `render_custom` is a view in the line, which the Linux
/// backend does not draw.
pub(crate) struct WikiLinks;

impl MarkdownPlugin for WikiLinks {
    fn name(&self) -> &str {
        WIKI_LINK
    }

    fn render(&self, node: &CustomNode) -> Option<AnyElement> {
        (node.name == WIKI_LINK).then(|| wiki_link(node))
    }
}

/// A wiki link of the rendered note: its alias, else what it names,
/// underlined (dashed while no note has that name). A press follows it.
fn wiki_link(node: &CustomNode) -> AnyElement {
    let inner = node.attributes.get("1").map_or("", String::as_str);
    let Some((target, _, alias)) = link::parse(inner) else {
        return text(node.text.clone()).into_element();
    };
    let label = alias.unwrap_or(inner.trim()).to_owned();
    let target = target.to_owned();
    let missing = {
        let target = target.clone();
        move || !store::link_exists(&target)
    };
    text(label)
        .underline()
        .text_color(move || theme().primary)
        .style_when(missing, |s| s.text_decoration_dashed())
        .role(AccessibilityRole::Link)
        .on_press(move || store::open_link(target.clone()))
        .into_element()
}

/// The notes linking to the open one, each with the line of its link; a
/// press opens it.
pub(crate) fn backlinks_panel() -> impl IntoElement {
    view()
        .flex_1()
        .gap_2()
        .p_3()
        .child(
            text(strings::BACKLINKS)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().muted_foreground),
        )
        .child(scroll().flex_1().child(dynamic(|| {
            let links = store::backlinks();
            match links.is_empty() {
                true => text(strings::NO_BACKLINKS)
                    .text_sm()
                    .text_color(move || theme().muted_foreground)
                    .into_element(),
                false => view()
                    .gap_1()
                    .child(links.into_iter().map(backlink))
                    .into_element(),
            }
        })))
}

fn backlink(link: Backlink) -> impl IntoElement {
    let title = note::title(&link.source).to_owned();
    view()
        .p_2()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .on_press(move || store::select(&link.source.clone().into()))
        .child(text(title).text_sm().text_color(move || theme().foreground))
        .child(
            text(link.context)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}
