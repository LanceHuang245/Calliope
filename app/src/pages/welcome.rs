//! The welcome page, without a vault: the desktops open a folder or create
//! one; a phone names the vault it creates in its sandbox.

use nsh_base::theme;
use nsh_core::prelude::*;

use crate::{
    components::{action, field, primary},
    store, strings,
};

pub(crate) fn welcome_page() -> impl IntoElement {
    let (message, actions) = match is_desktop() {
        true => (
            strings::NO_VAULT,
            view()
                .flex_row()
                .gap_2()
                .child(action(strings::OPEN_FOLDER, || store::pick_vault(false)))
                .child(action(strings::NEW_VAULT, || store::pick_vault(true)))
                .into_element(),
        ),
        false => {
            let name = signal(String::new());
            let form = view()
                .w_full()
                .px_6()
                .gap_3()
                .child(
                    field(strings::VAULT_NAME)
                        .label(strings::VAULT_NAME)
                        .on_change(move |text: &str| name.set(text.to_owned()))
                        .on_submit(|text: &str| store::create_vault(text)),
                )
                .child(primary(strings::CREATE, move || {
                    store::create_vault(&name.get())
                }));
            (strings::NAME_VAULT, form.into_element())
        }
    };
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
