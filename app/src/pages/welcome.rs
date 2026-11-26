//! The welcome page, without a vault: the desktops open a folder or create
//! one; a phone names the vault it creates in its sandbox, empty or made
//! from sync.

use calliope_sync::{Config, Provider};
use nsh_base::theme;
use nsh_core::{element::AnyElement, prelude::*};

use super::sync::{encryption_fields, fields, secret_fields};
use crate::i18n::t;

use crate::{
    components::{ButtonVariant, action, button, field, labeled, muted, segmented},
    i18n, store,
};

pub(crate) fn welcome_page() -> impl IntoElement {
    if is_desktop() {
        let actions = view()
            .flex_row()
            .gap_2()
            .child(action(t("open_folder"), || store::pick_vault(false)))
            .child(action(t("new_vault"), || store::pick_vault(true)));
        return welcome(t("no_vault"), actions.into_element()).into_element();
    }
    // A phone's two ways in (MVP phase 4): an empty vault, or the form of
    // one made from sync.
    let from_sync = signal(false);
    dynamic(move || match from_sync.get() {
        false => untracked(|| welcome(t("name_vault"), new_vault(from_sync))).into_element(),
        true => untracked(|| sync_form(from_sync)).into_element(),
    })
    .into_element()
}

/// The app's name over `message`, and `actions` under them, in the middle.
fn welcome(message: impl Into<SharedString>, actions: AnyElement) -> impl IntoElement {
    let message: SharedString = message.into();
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_3()
        .child(
            text(i18n::APP_NAME)
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
            field(t("vault_name"))
                .label(t("vault_name"))
                .on_change(move |text: &str| name.set(text.to_owned()))
                .on_submit(|text: &str| store::create_vault(text)),
        )
        .child(button(ButtonVariant::Primary, t("create"), move || {
            store::create_vault(&name.get())
        }))
        .child(button(
            ButtonVariant::Secondary,
            t("create_from_sync"),
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
            text(t("create_from_sync"))
                .text_2xl()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(move || theme().foreground),
        )
        .child(muted(t("from_sync_hint")))
        .child(labeled(
            t("vault_name"),
            true,
            field(t("vault_name"))
                .label(t("vault_name"))
                .on_change(move |text: &str| name.set(text.to_owned())),
        ))
        .child(segmented(
            vec![(Provider::WebDav, t("webdav")), (Provider::S3, t("s3"))],
            move || provider.get(),
            move |p| provider.set(p),
        ))
        // The service's fields again when it changes, as in the settings.
        .child(dynamic(move || {
            let provider = provider.get();
            untracked(|| fields(provider, draft, secret, false))
        }))
        .child(encryption_fields(mode, false))
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .pt_2()
                .child(button(ButtonVariant::Secondary, t("cancel"), back))
                .child(button(ButtonVariant::Primary, t("create"), create))
                .child(dynamic(|| {
                    store::connecting().then(|| muted(t("connecting")))
                })),
        );
    scroll().flex_1().child(form)
}
