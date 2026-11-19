//! Asking before what cannot be undone (deleting) or what makes something
//! (the note a link names): Calliope's own dialog on the desktops, which
//! the shell shows while [`asking`]; the system's alert on the phones (MVP
//! 5.4).

use std::rc::Rc;

use nsh_base::alert_dialog;

use super::*;

/// A question to the user.
#[derive(Clone)]
pub(crate) struct Ask {
    pub title: String,
    /// What it means, under the title.
    pub description: Option<&'static str>,
    /// The button that goes on.
    pub action: &'static str,
    /// Going on deletes: the system's alert draws the button so.
    pub destructive: bool,
}

/// What the desktops are asking, and what to do when the user goes on.
#[derive(Clone, Default)]
struct Asking(Option<(Ask, Rc<dyn Fn()>)>);

/// Asks `ask`, then runs `then` if the user goes on.
pub(crate) fn confirm(ask: Ask, then: impl Fn() + 'static) {
    if is_desktop() {
        return set(Asking(Some((ask, Rc::new(then)))));
    }
    spawn_task(async move {
        let mut alert = alert_dialog()
            .title(ask.title)
            .cancel("cancel", strings::CANCEL);
        if let Some(description) = ask.description {
            alert = alert.description(description);
        }
        alert = match ask.destructive {
            true => alert.destructive("go", ask.action),
            false => alert.action("go", ask.action),
        };
        if matches!(alert.show().await, Ok(r) if r.action.as_deref() == Some("go")) {
            then();
        }
    });
}

/// What the desktops are asking, for the dialog.
pub(crate) fn asking() -> Option<Ask> {
    get::<Asking>().0.map(|(ask, _)| ask)
}

/// The user's answer to what [`asking`] asks: `go` on, or not.
pub(crate) fn answer(go: bool) {
    let asked = untracked(|| get::<Asking>().0);
    set(Asking(None));
    if let (true, Some((_, then))) = (go, asked) {
        then();
    }
}
