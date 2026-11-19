//! Links between notes: the wiki links of the rendered note and the
//! backlinks of the open note (a panel on the desktops, a sheet on the
//! phones).

use calliope_core::{index::Backlink, link, note};
use nsh_base::{
    MarkdownExtension, MarkdownPlugin, SheetDetent, SheetEvent, sheet, text_view::CustomNode, theme,
};
use nsh_core::prelude::*;

use crate::{
    components::{
        card,
        icons::{self, icon},
        muted,
    },
    store, strings,
};

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

/// The phones' backlinks: a sheet over the note, at half its height and
/// dragged up to all of it, each note linking here a card with the line of
/// its link; a press opens that note.
pub(crate) fn backlinks_sheet() -> impl IntoElement {
    let list = dynamic(|| {
        let links = store::backlinks();
        match links.is_empty() {
            true => view()
                .items_center()
                .gap_2()
                .py_8()
                .child(
                    icon(icons::LINK)
                        .w(px(24.0))
                        .h(px(24.0))
                        .text_color(move || theme().muted_foreground),
                )
                .child(muted(strings::NO_BACKLINKS))
                .into_element(),
            false => view()
                .gap_2()
                .child(links.into_iter().map(link_card))
                .into_element(),
        }
    });
    sheet()
        .open(move || Some(store::backlinks_shown()))
        .on_open_change(|e: &SheetEvent| {
            if let SheetEvent::OpenChange(open, _) = e {
                store::set_backlinks_shown(*open);
            }
        })
        .detents(vec![SheetDetent::Medium, SheetDetent::Large])
        .default_detent(SheetDetent::Medium)
        .grabber(true)
        .grabber_class("mx-auto my-2 h-1 w-9 rounded-full bg-muted-foreground/40")
        .popup_style(|s| s.w_full().rounded_t(px(24.0)).bg(theme().sidebar))
        .child(
            view()
                .flex_1()
                .px_4()
                .pb_safe_area()
                .gap_3()
                .child(
                    view()
                        .flex_row()
                        .items_center()
                        .child(
                            text(strings::BACKLINKS)
                                .flex_1()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(move || theme().foreground),
                        )
                        .child(muted(|| store::backlinks().len().to_string())),
                )
                .child(scroll().flex_1().child(list)),
        )
}

/// A note linking to the open one, on a card: its title, and the line of
/// its link with the wiki links in the theme's `primary`.
fn link_card(link: Backlink) -> impl IntoElement {
    let title = note::title(&link.source).to_owned();
    let source = link.source.clone();
    card()
        .flex_row()
        .items_center()
        .gap_3()
        .p_3()
        .active(|s| s.opacity(0.7))
        .on_press(move || {
            store::set_backlinks_shown(false);
            store::select(&source.clone().into());
        })
        .child(
            view()
                .flex_1()
                .gap_1()
                .child(
                    text(title)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(highlighted(&link.context)),
        )
        .child(icon(icons::CHEVRON_RIGHT).text_color(move || theme().muted_foreground))
}

/// `line` in muted text, its `[[wiki links]]` in the theme's `primary`.
fn highlighted(line: &str) -> impl IntoElement {
    let mut spans = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("[[") {
        let Some(end) = rest[start..].find("]]").map(|e| start + e + 2) else {
            break;
        };
        spans.push((rest[..start].to_owned(), false));
        spans.push((rest[start..end].to_owned(), true));
        rest = &rest[end..];
    }
    spans.push((rest.to_owned(), false));
    text("")
        .text_sm()
        .text_color(move || theme().muted_foreground)
        .children(
            spans
                .into_iter()
                .filter(|(t, _)| !t.is_empty())
                .map(|(t, link)| {
                    text(t).text_color(move || match link {
                        true => theme().primary,
                        false => theme().muted_foreground,
                    })
                }),
        )
}
