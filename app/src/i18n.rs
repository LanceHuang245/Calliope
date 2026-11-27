//! The interface's language (MVP phase 6): the texts are the app's
//! `locales/` files (rust-i18n's format, which `nsh`'s build reads), and this
//! module says which language shows them.
//!
//! The running language is fixed at startup. Settings show the choice saved
//! for the next start, including on pages opened before the app restarts.

use std::{collections::HashMap, fmt::Display};

use nsh_core::{
    i18n::Translations,
    platform::Locale,
    prelude::*,
    storage::{Storage, StorageFull},
};

/// The app's name, the same in every language.
pub(crate) const APP_NAME: &str = "Calliope";

/// The languages the interface speaks; English is the default.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Language {
    #[default]
    English,
    SimplifiedChinese,
}

impl Language {
    /// Both, in the order the settings list them.
    pub(crate) const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    /// Its name in its own language: what the settings' list shows.
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::SimplifiedChinese => "简体中文",
        }
    }

    /// Its tag in `locales/` and in the device's storage (`en`,
    /// `zh-Hans`).
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-Hans",
        }
    }

    /// The language `code` names; `None` for one this build has not.
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.code() == code)
    }
}

/// The item of the device's storage the choice is kept in, as the theme's.
const LANGUAGE: &str = "language";

#[derive(Clone, Copy, Default)]
struct Languages {
    running: Language,
    saved: Language,
}

pub(crate) fn saved_language() -> Language {
    global::<Languages>().get().saved
}

/// Reads the language saved on this device: called once, at start.
pub(crate) fn restore_language() {
    let language = Storage::get_item(LANGUAGE)
        .and_then(|code| Language::from_code(&code))
        .unwrap_or_default();
    global::<Languages>().set(Languages {
        running: language,
        saved: language,
    });
    // Framework-owned text uses the same startup language.
    nsh_core::i18n::set_locale(Locale::parse(language.code()));
}

/// Saves `language` for this device: the next start speaks it.
pub(crate) fn save_language(language: Language) -> Result<(), StorageFull> {
    Storage::set_item(LANGUAGE, language.code())?;
    global::<Languages>().update(|languages| languages.saved = language);
    Ok(())
}

/// The app's translations: its `locales/` files, as `nsh`'s build reads
/// them. `cargo run` and the tests take them from here; a build of `nsh run`
/// or `nsh package` compiles the same files in.
pub fn translations() -> Translations {
    let mut translations: HashMap<String, HashMap<String, String>> = HashMap::new();
    for language in Language::ALL {
        let json = match language {
            Language::English => include_str!("../locales/en.json"),
            Language::SimplifiedChinese => include_str!("../locales/zh-Hans.json"),
        };
        let texts: HashMap<String, String> = serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("locales/{}: {e}", language.code()));
        translations.insert(language.code().to_owned(), texts);
    }
    Translations::from_map(translations)
}

/// The text of `key` in the language this device keeps: `t("new_note")`.
pub(crate) fn t(key: &str) -> String {
    translate(key, &[])
}

/// The text of `key` with its `%{name}`s filled from `args` (nsh's own
/// `i18n::translate`, given the language this device keeps).
pub(crate) fn translate(key: &str, args: &[(&str, &dyn Display)]) -> String {
    let running = untracked(|| global::<Languages>().get().running);
    nsh_core::i18n::translate(key, Some(running.code()), args)
}

/// What the desktops do with a file the user deletes: each names its own
/// place for them.
pub(crate) fn delete_to_system() -> String {
    t(match cfg!(target_os = "windows") {
        true => "delete_to_system_recycle_bin",
        false => "delete_to_system_trash",
    })
}

/// What the desktops' trash keeps, by those same names.
pub(crate) fn trash_keeps_desktop() -> String {
    t(match cfg!(target_os = "windows") {
        true => "trash_keeps_desktop_recycle_bin",
        false => "trash_keeps_desktop_trash",
    })
}

pub(crate) fn deleted_from(origin: &str, day: &str) -> String {
    translate("deleted_from", &[("origin", &origin), ("day", &day)])
}

pub(crate) fn restored(path: &str) -> String {
    translate("restored", &[("path", &path)])
}

pub(crate) fn deleting(count: usize) -> String {
    translate("deleting", &[("count", &count)])
}

pub(crate) fn syncing(done: usize, total: usize) -> String {
    translate("syncing", &[("done", &done), ("total", &total)])
}

pub(crate) fn synced_at(time: &str) -> String {
    translate("synced_at", &[("time", &time)])
}

/// English tells one conflict from many; Chinese needs no telling.
pub(crate) fn synced_conflicts(time: &str, count: usize) -> String {
    match count {
        1 => translate(
            "synced_conflicts_one",
            &[("time", &time), ("count", &count)],
        ),
        _ => translate(
            "synced_conflicts_many",
            &[("time", &time), ("count", &count)],
        ),
    }
}

pub(crate) fn sync_error(error: &dyn Display) -> String {
    translate("sync_error", &[("error", &error)])
}

pub(crate) fn conflict_copies(copies: &[String]) -> String {
    let separator = match untracked(|| global::<Languages>().get().running) {
        Language::English => ", ",
        Language::SimplifiedChinese => "、",
    };
    translate("conflict_copies", &[("copies", &copies.join(separator))])
}

pub(crate) fn create_note(name: &str) -> String {
    translate("create_note", &[("name", &name)])
}

pub(crate) fn version() -> String {
    translate("version", &[("version", &env!("CARGO_PKG_VERSION"))])
}

pub(crate) fn saved_copy(path: &str) -> String {
    translate("saved_copy", &[("path", &path)])
}

/// The tag of the copy a command writes beside a note
/// (`Note (copy 2026-11-27 1530).md`). A file name, so it stays the same in
/// every language: the same note keeps its name whatever the interface
/// speaks.
pub(crate) fn copy_tag() -> String {
    format!(
        "copy {}",
        jiff::ZonedDateTime::now().strftime("%Y-%m-%d %H%M")
    )
}
