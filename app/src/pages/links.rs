//! Links between notes: the wiki links of the rendered note and the note's
//! panels — its backlinks, its outline and its outlinks (beside the note on
//! the desktops, a sheet on the phones).

use calliope_core::{
    index::Backlink,
    link,
    note::{self, Heading},
};
use nsh_base::{
    MarkdownExtension, MarkdownPlugin, SheetDetent, SheetEvent, sheet, text_view::CustomNode, theme,
};
use nsh_core::prelude::*;

use crate::i18n::t;

use crate::{
    components::{
        desktop_layout, icon_button,
        icons::{self, icon},
        muted,
    },
    store::{self, Outlink, Panel},
};

/// The name of the custom nodes wiki links are read as.
const WIKI_LINK: &str = "wikilink";

/// The phones' sheet of panels, as the tests find it.
const SHEET: &str = "panels";

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

/// Closes the phones' sheet, which covers the note a row asks for; the
/// desktops' panels stay beside it.
fn leave_sheet() {
    if !desktop_layout() {
        store::close_panels();
    }
}

/// A panel beside the note (the desktops): its title, a button folding it
/// away, and what it lists.
pub(crate) fn panel_card(panel: Panel) -> impl IntoElement {
    view()
        .flex_1()
        .test_id(panel.test_id())
        .gap_2()
        .p_3()
        .child(
            view()
                .flex_row()
                .items_center()
                .child(
                    text(t(panel.title()))
                        .flex_1()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().muted_foreground),
                )
                .child(icon_button(icons::X, t("close"), move || {
                    store::set_panel_shown(panel, false)
                })),
        )
        .child(scroll().flex_1().child(dynamic(move || panel_body(panel))))
}

/// The phones' panels: a sheet over the note, at half its height and dragged
/// up to all of it, showing the one asked for.
pub(crate) fn panel_sheet() -> impl IntoElement {
    // The panel last shown: the sheet keeps its content while it slides away.
    let showing = signal(Panel::Backlinks);
    effect(move || {
        if let Some(panel) = store::open_panel() {
            showing.set(panel);
        }
    });
    let title = text(move || t(showing.get().title()))
        .flex_1()
        .text_lg()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(move || theme().foreground);
    sheet()
        .open(move || Some(store::open_panel().is_some()))
        .on_open_change(|e: &SheetEvent| {
            if let SheetEvent::OpenChange(open, _) = e
                && !*open
            {
                store::close_panels();
            }
        })
        .detents(vec![SheetDetent::Medium, SheetDetent::Large])
        .default_detent(SheetDetent::Medium)
        .grabber(true)
        .grabber_class("mx-auto my-2 h-1 w-9 rounded-full bg-muted-foreground/40")
        .popup_style(|s| s.w_full().rounded_t(px(24.0)).bg(theme().sidebar))
        .child(
            view()
                .test_id(SHEET)
                .flex_1()
                .px_4()
                .pb_safe_area()
                .gap_3()
                .child(view().flex_row().items_center().child(title))
                .child(
                    scroll()
                        .flex_1()
                        .child(dynamic(move || panel_body(showing.get()))),
                ),
        )
}

/// What `panel` lists, a row each; its icon and a word while there is
/// nothing.
fn panel_body(panel: Panel) -> AnyElement {
    match panel {
        Panel::Backlinks => match store::backlinks().is_empty() {
            true => nothing(panel, t("no_backlinks")),
            false => view()
                .gap_1()
                .child(store::backlinks().into_iter().map(backlink))
                .into_element(),
        },
        Panel::Outline => match store::outline().is_empty() {
            true => nothing(panel, t("no_outline")),
            false => view()
                .gap_1()
                .child(store::outline().into_iter().map(heading_row))
                .into_element(),
        },
        Panel::Outlinks => match store::outlinks().is_empty() {
            true => nothing(panel, t("no_outlinks")),
            false => view()
                .gap_1()
                .child(store::outlinks().into_iter().map(outlink_row))
                .into_element(),
        },
    }
}

pub(crate) fn panel_icon(panel: Panel) -> &'static str {
    match panel {
        Panel::Backlinks => icons::LINK,
        Panel::Outline => icons::LIST,
        Panel::Outlinks => icons::EXTERNAL_LINK,
    }
}

fn nothing(panel: Panel, message: String) -> AnyElement {
    view()
        .items_center()
        .gap_2()
        .py_8()
        .child(
            icon(panel_icon(panel))
                .w(px(24.0))
                .h(px(24.0))
                .text_color(move || theme().muted_foreground),
        )
        .child(muted(message))
        .into_element()
}

/// A note linking to the open one, with the line of its link; a press opens
/// it.
fn backlink(link: Backlink) -> impl IntoElement {
    let title = note::title(&link.source).to_owned();
    view()
        .p_2()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .on_press(move || {
            leave_sheet();
            store::select(&link.source.clone().into());
        })
        .child(text(title).text_sm().text_color(move || theme().foreground))
        .child(
            text(link.context)
                .text_xs()
                .text_color(move || theme().muted_foreground),
        )
}

/// A heading of the open note: as deep as its level; a press shows the note
/// in the source, scrolled to it.
fn heading_row(heading: Heading) -> impl IntoElement {
    let line = heading.line as u32;
    let indent = 8.0 + f32::from(heading.level.saturating_sub(1)) * 12.0;
    view()
        .py_1()
        .pr_2()
        .pl(px(indent))
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .on_press(move || {
            leave_sheet();
            store::jump_to(line);
        })
        .child(
            text(heading.text)
                .text_sm()
                .text_color(move || theme().foreground),
        )
}

/// A note the open note links to: muted while no note has that name, and
/// saying so; a press opens it, or asks to create it.
fn outlink_row(outlink: Outlink) -> impl IntoElement {
    let exists = outlink.exists;
    let target = outlink.target;
    let pressed = target.clone();
    view()
        .p_2()
        .rounded(move || theme().radius)
        .hover(|s| s.bg(theme().accent))
        .on_press(move || {
            leave_sheet();
            store::open_link(pressed.clone());
        })
        .child(text(target).text_sm().text_color(move || match exists {
            true => theme().foreground,
            false => theme().muted_foreground,
        }))
        .child(
            (!exists).then_some(
                text(t("missing_note"))
                    .text_xs()
                    .text_color(move || theme().muted_foreground),
            ),
        )
}
