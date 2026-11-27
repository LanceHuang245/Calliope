//! The note's panels: the backlinks, the outline and the outlinks, each
//! shown when asked (a toolbar button on the phones, a command anywhere).
//! The desktops show them beside the note, the phones in a sheet over it.

use calliope_core::{
    link,
    note::{self, Heading},
};

use super::*;

/// A panel: what it lists of the open note.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Panel {
    /// The notes linking to the open one.
    Backlinks,
    /// The open note's headings.
    Outline,
    /// The notes the open one links to.
    Outlinks,
}

impl Panel {
    /// All of them, in the order they sit.
    pub(crate) const ALL: [Self; 3] = [Self::Backlinks, Self::Outline, Self::Outlinks];

    /// Its key among the texts (`t!`): what its title and its toolbar
    /// button say.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Backlinks => "backlinks",
            Self::Outline => "outline",
            Self::Outlinks => "outlinks",
        }
    }

    /// Its id in the tests (`panel-outline`).
    pub(crate) fn test_id(self) -> &'static str {
        match self {
            Self::Backlinks => "panel-backlinks",
            Self::Outline => "panel-outline",
            Self::Outlinks => "panel-outlinks",
        }
    }

    /// Its bit in [`Shown`].
    fn bit(self) -> u8 {
        match self {
            Self::Backlinks => 1,
            Self::Outline => 2,
            Self::Outlinks => 4,
        }
    }
}

/// A note the open one links to: the note itself once it exists.
pub(crate) struct Outlink {
    pub target: String,
    /// Whether a note has that name (an uncreated one is listed too).
    pub exists: bool,
}

/// The panels shown.
#[derive(Clone)]
struct Shown(u8);

/// The desktops start with the backlinks beside the note, the phones with
/// none (their sheet covers the note).
impl Default for Shown {
    fn default() -> Self {
        Self(match is_desktop() {
            true => Panel::Backlinks.bit(),
            false => 0,
        })
    }
}

/// Whether `panel` shows: asked for, and beside an open note (not the
/// settings, the trash, the graph or the search).
pub(crate) fn panel_shown(panel: Panel) -> bool {
    global::<Shown>().with(|shown| shown.0 & panel.bit() != 0) && beside_note()
}

fn beside_note() -> bool {
    opened().is_some()
        && !matches!(
            route(),
            Route::Settings
                | Route::SettingsDetail(_)
                | Route::Trash
                | Route::Graph
                | Route::Search
        )
}

/// The panel a phone's sheet shows; a phone shows one at a time.
pub(crate) fn open_panel() -> Option<Panel> {
    Panel::ALL.into_iter().find(|panel| panel_shown(*panel))
}

/// Shows or hides `panel`: a phone shows one at a time (its sheet covers
/// the note), a desktop each on its own.
pub(crate) fn toggle_panel(panel: Panel) {
    global::<Shown>().update(|shown| {
        let bit = panel.bit();
        shown.0 = match shown.0 & bit != 0 {
            true => shown.0 & !bit,
            false if is_desktop() => shown.0 | bit,
            false => bit,
        };
    });
}

/// Shows `panel` or folds it away (a card's close button).
pub(crate) fn set_panel_shown(panel: Panel, shown: bool) {
    global::<Shown>().update(|open| match shown {
        true => open.0 |= panel.bit(),
        false => open.0 &= !panel.bit(),
    });
}

/// Folds every panel away: the phones' sheet, swiped closed.
pub(crate) fn close_panels() {
    set(Shown(0));
}

/// Each layout starts with its own panels (a tablet turned, or put beside
/// another app).
pub(crate) fn reset_panels(desktop: bool) {
    set(Shown(match desktop {
        true => Panel::Backlinks.bit(),
        false => 0,
    }));
}

/// The open note's headings, in order; none without a note.
pub(crate) fn outline() -> Vec<Heading> {
    match opened().is_some() {
        true => note::outline(&text()),
        false => Vec::new(),
    }
}

/// The notes the open note links to, once each, in the order it writes
/// them; the ones no note has yet too.
pub(crate) fn outlinks() -> Vec<Outlink> {
    if opened().is_none() {
        return Vec::new();
    }
    let mut found: Vec<Outlink> = Vec::new();
    for link in link::links(&text()) {
        if !found.iter().any(|out| out.target == link.target) {
            let exists = super::link_exists(&link.target);
            found.push(Outlink {
                target: link.target,
                exists,
            });
        }
    }
    found
}
