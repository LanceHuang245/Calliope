//! What a sync does with each file (MVP 8.3): a three-way comparison of the
//! local file and the remote one with their record from the last sync.

use crate::{Error, Result};

/// How one side of a file differs from its record. A file without a record
/// is `Changed` where it is (new) and `Gone` where it is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Same,
    Changed,
    Gone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Upload,
    Download,
    DeleteRemote,
    /// Moves the local file to the vault's `.trash/`.
    DeleteLocal,
    /// Changed on both sides: the local file stays, the remote one becomes a
    /// conflict copy beside it (unless both hold the same).
    Conflict,
    /// Gone from both sides: only its record goes.
    Forget,
}

/// What to do with a file, from how its local and remote sides changed.
pub(crate) fn action(local: Side, remote: Side) -> Option<Action> {
    use Side::*;
    Some(match (local, remote) {
        (Same, Same) => return None,
        (Changed, Same | Gone) => Action::Upload,
        (Same | Gone, Changed) => Action::Download,
        (Changed, Changed) => Action::Conflict,
        (Gone, Same) => Action::DeleteRemote,
        (Same, Gone) => Action::DeleteLocal,
        (Gone, Gone) => Action::Forget,
    })
}

/// Stops a run that would delete more than 20 files, or more than a tenth
/// of all `files`, unless the user `confirmed` it (MVP 8.5).
pub(crate) fn check_deletions(actions: &[Action], files: usize, confirmed: bool) -> Result<()> {
    let deletions = (actions.iter())
        .filter(|a| matches!(a, Action::DeleteRemote | Action::DeleteLocal))
        .count();
    match !confirmed && (deletions > 20 || deletions * 10 > files) {
        true => Err(Error::TooManyDeletions(deletions)),
        false => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Action::*, Side::*, *};

    #[test]
    fn follows_the_table_of_the_mvp() {
        // Local, remote, action: each row of MVP 8.3, then a file new on
        // both sides and one gone from both.
        let table = [
            (Same, Same, None),
            (Changed, Same, Some(Upload)),
            (Same, Changed, Some(Download)),
            (Changed, Changed, Some(Conflict)),
            (Gone, Same, Some(DeleteRemote)),
            (Same, Gone, Some(DeleteLocal)),
            (Gone, Changed, Some(Download)),
            (Changed, Gone, Some(Upload)),
            (Gone, Gone, Some(Forget)),
        ];
        for (local, remote, expected) in table {
            assert_eq!(action(local, remote), expected, "{local:?} {remote:?}");
        }
    }

    #[test]
    fn asks_before_deleting_many() {
        assert!(check_deletions(&[DeleteLocal; 2], 30, false).is_ok());
        assert!(check_deletions(&[DeleteLocal; 4], 30, false).is_err());
        assert!(check_deletions(&[DeleteRemote; 21], 1000, false).is_err());
        assert!(check_deletions(&[DeleteRemote; 21], 1000, true).is_ok());
    }
}
