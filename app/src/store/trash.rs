//! The vault's trash as its page shows it (MVP phase 4): what sync, and
//! the phones, deleted; restored where it was, or deleted for good.

use std::io;

use calliope_core::trash::Trashed;

use super::*;

/// The files in the trash, as last read ([`load_trash`]).
#[derive(Clone, Default)]
struct Trash(Vec<Trashed>);

pub(crate) fn trashed() -> Vec<Trashed> {
    get::<Trash>().0
}

/// Reads the open vault's trash again.
pub(crate) fn load_trash() {
    let read = untracked(vault).map_or(Ok(Vec::new()), |v| v.trashed());
    match read {
        Ok(items) => set(Trash(items)),
        Err(e) => notify(e),
    }
}

/// Opens the trash, or returns to the note when it is already shown (the
/// desktops' sidebar button).
pub(crate) fn toggle_trash() {
    match untracked(route) {
        Route::Trash => back_to_note(),
        _ => set_route(Route::Trash),
    }
}

/// Moves `item` back where it was, and says where.
pub(crate) fn restore(item: Trashed) {
    change_trash(|vault| {
        let path = vault.restore(&item)?;
        inform(strings::restored(&path));
        refresh();
        Ok(())
    });
}

/// Asks, then deletes `item` for good.
pub(crate) fn delete_forever(item: Trashed) {
    let ask = for_good(strings::DELETE_FOREVER_TITLE);
    confirm(ask, move || {
        change_trash(|vault| vault.delete_forever(&item))
    });
}

/// Asks, then deletes everything in the trash.
pub(crate) fn clear_trash() {
    let ask = for_good(strings::EMPTY_TRASH_TITLE);
    confirm(ask, || change_trash(Vault::clear_trash));
}

/// Makes `change` to the open vault's trash, then reads it again.
fn change_trash(change: impl FnOnce(&Vault) -> io::Result<()>) {
    if let Some(vault) = untracked(vault)
        && let Err(e) = change(&vault)
    {
        notify(e);
    }
    load_trash();
}

/// Asking `title` before deleting for good, which cannot be undone.
fn for_good(title: &'static str) -> Ask {
    Ask {
        title: title.to_owned(),
        description: Some(strings::CANNOT_UNDO),
        action: strings::DELETE,
        destructive: true,
    }
}
