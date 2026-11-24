//! The welcome page, without a vault: the desktops open a folder or create
//! one; a phone names the vault it creates in its sandbox, empty or made
//! from sync.

use calliope_sync::{Config, Provider};
use nsh_base::theme;
use nsh_core::{element::AnyElement, prelude::*};

use super::sync::{encryption_fields, fields, secret_fields};
use crate::{
    components::{ButtonVariant, action, button, field, labeled, muted, segmented},
    store, strings,
};

pub(crate) fn welcome_page() -> impl IntoElement {
    if is_desktop() {
        let actions = view()
            .flex_row()
            .gap_2()
            .child(action(strings::OPEN_FOLDER, || store::pick_vault(false)))
            .child(action(strings::NEW_VAULT, || store::pick_vault(true)));
        return welcome(strings::NO_VAULT, actions.into_element()).into_element();
    }
    // A phone's two ways in (MVP phase 4): an empty vault, or the form of
    // one made from sync.
    let from_sync = signal(false);
    dynamic(move || match from_sync.get() {
        false => untracked(|| welcome(strings::NAME_VAULT, new_vault(from_sync))).into_element(),
        true => untracked(|| sync_form(from_sync)).into_element(),
    })
    .into_element()
}

/// The app's name over `message`, and `actions` under them, in the middle.
fn welcome(message: &'static str, actions: AnyElement) -> impl IntoElement {
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            text(strings::APP_NAME)
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(text(message).text_color(move || theme().muted_foreground))
        .child(actions)
}

/// A phone's empty vault, by its name; or the way to one made from sync.
fn new_vault(from_sync: Signal<bool>) -> AnyElement {
    let name = signal(String::new());
    view()
        .w_full()
        .px_6()
        .gap_3()
        .child(
            field(strings::VAULT_NAME)
                .label(strings::VAULT_NAME)
                .on_change(move |text: &str| name.set(text.to_owned()))
                .on_submit(|text: &str| store::create_vault(text)),
        )
        .child(button(ButtonVariant::Primary, strings::CREATE, move || {
            store::create_vault(&name.get())
        }))
        .child(button(
            ButtonVariant::Secondary,
            strings::CREATE_FROM_SYNC,
            move || from_sync.set(true),
        ))
        .into_element()
}

/// A phone's vault made from sync: its name, the service and its settings
/// (the sync settings' own fields). Create checks the connection (saying
/// so while it does), then makes the vault and downloads it; Cancel and
/// the system's back return to the two ways in.
fn sync_form(from_sync: Signal<bool>) -> impl IntoElement {
    let back = move || from_sync.set(false);
    on_back_press(move || {
        back();
        true
    });
    let name = signal(String::new());
    let provider = signal(Provider::WebDav);
    let draft = signal(Config::default());
    let secret = signal(String::new());
    let mode = secret_fields(false);
    let create = move || {
        let config = Config {
            provider: provider.get(),
            encrypted: mode.encrypted(),
            ..draft.get()
        };
        store::create_from_sync(name.get(), config, secret.get(), mode.for_test().0)
    };
    let form = view()
        .p_6()
        .gap_3()
        .items_start()
        .child(
            text(strings::CREATE_FROM_SYNC)
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(muted(strings::FROM_SYNC_HINT))
        .child(labeled(
            strings::VAULT_NAME,
            true,
            field(strings::VAULT_NAME)
                .label(strings::VAULT_NAME)
                .on_change(move |text: &str| name.set(text.to_owned())),
        ))
        .child(segmented(
            vec![
                (Provider::WebDav, strings::WEBDAV),
                (Provider::S3, strings::S3),
            ],
            move || provider.get(),
            move |p| provider.set(p),
        ))
        // The service's fields again when it changes, as in the settings.
        .child(dynamic(move || {
            let provider = provider.get();
            untracked(|| fields(provider, draft, secret))
        }))
        .child(encryption_fields(mode, false))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .pt_2()
                .child(button(ButtonVariant::Secondary, strings::CANCEL, back))
                .child(button(ButtonVariant::Primary, strings::CREATE, create))
                .child(dynamic(|| {
                    store::connecting().then(|| muted(strings::CONNECTING))
                })),
        );
    scroll().flex_1().child(form)
}
