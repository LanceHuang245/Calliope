//! Sync in the settings (MVP 8.7): the service and its settings, checking
//! the connection, syncing on its own, and how the last run went; and the
//! dialog asking before a run deletes many files.

use calliope_sync::{Config, Provider};
use nsh_base::{dialog, theme};
use nsh_core::prelude::*;

use super::settings::muted;
use crate::{
    components::{action, card, field, overlay, primary, section, segmented},
    store::{self, SyncStatus},
    strings,
};

pub(crate) fn sync_settings() -> impl IntoElement {
    // The settings as edited, kept by Save. The service is a signal of its
    // own: changing it shows other fields, typing in one does not.
    let saved = untracked(store::sync_config);
    let provider = signal(saved.provider);
    let draft = signal(saved);
    let secret = signal(String::new());
    let edited = move || Config {
        provider: provider.get(),
        ..draft.get()
    };
    view()
        .w_full()
        .gap_2()
        .items_start()
        .child(section(strings::SYNC))
        .child(segmented(
            vec![
                (Provider::Off, strings::OFF),
                (Provider::WebDav, strings::WEBDAV),
                (Provider::S3, strings::S3),
            ],
            move || provider.get(),
            move |p| provider.set(p),
        ))
        .child(dynamic(move || {
            let on = provider.get() != Provider::Off;
            untracked(|| on.then(|| fields(provider.get(), draft, secret)))
        }))
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(primary(strings::SAVE, move || {
                    store::save_sync(edited(), &secret.get())
                }))
                .child(action(strings::TEST_CONNECTION, move || {
                    store::test_sync(edited(), secret.get())
                }))
                .child(action(strings::SYNC_NOW, store::sync_now)),
        )
        .child(muted(move || match store::sync_status() {
            SyncStatus::Idle => strings::NOT_SYNCED.to_owned(),
            SyncStatus::Running(done, total) => strings::syncing(done, total),
            SyncStatus::Done(time) => strings::synced_at(&time),
            SyncStatus::Failed(error) => strings::sync_failed(&error),
        }))
        .child(dynamic(|| {
            store::secret_in_file().then(|| muted(strings::SECRET_IN_FILE))
        }))
}

/// The settings of the service `provider`, typed into `draft`; the password
/// or secret key into `secret`.
fn fields(provider: Provider, draft: Signal<Config>, secret: Signal<String>) -> impl IntoElement {
    let c = draft.get();
    let s3 = provider == Provider::S3;
    let entry = move |label, value: &str, set: fn(&mut Config, String)| {
        labeled(
            label,
            input(label, value, move |typed| draft.update(|c| set(c, typed))),
        )
    };
    let secret_label = match s3 {
        true => strings::SECRET_KEY,
        false => strings::PASSWORD,
    };
    // A saved password stays while its field is left empty.
    let kept = untracked(store::sync_config).provider != Provider::Off;
    view()
        .w_full()
        .gap_2()
        .items_start()
        .child(match s3 {
            true => entry(strings::ENDPOINT, &c.endpoint, |c, t| c.endpoint = t),
            false => entry(strings::SERVER_ADDRESS, &c.endpoint, |c, t| c.endpoint = t),
        })
        .child(s3.then(|| entry(strings::REGION, &c.region, |c, t| c.region = t)))
        .child(s3.then(|| entry(strings::BUCKET, &c.bucket, |c, t| c.bucket = t)))
        .child(match s3 {
            true => entry(strings::ACCESS_KEY, &c.user, |c, t| c.user = t),
            false => entry(strings::USER_NAME, &c.user, |c, t| c.user = t),
        })
        .child(
            labeled(
                secret_label,
                input(secret_label, "", move |typed| secret.set(typed)).masked(true),
            )
            .child(kept.then(|| muted(strings::SECRET_KEPT))),
        )
        .child(match s3 {
            true => entry(strings::PATH_PREFIX, &c.root, |c, t| c.root = t),
            false => entry(strings::REMOTE_FOLDER, &c.root, |c, t| c.root = t),
        })
        .child(s3.then(|| {
            segmented(
                vec![(true, strings::PATH_STYLE), (false, strings::VIRTUAL_HOST)],
                move || draft.get().path_style,
                move |on| draft.update(|c| c.path_style = on),
            )
        }))
        // A server with a certificate of its own (a NAS): trusted by the
        // fingerprint a failed connection shows.
        .child(
            entry(strings::CERTIFICATE, &c.certificate, |c, t| {
                c.certificate = t
            })
            .child(muted(strings::CERTIFICATE_HINT)),
        )
        .child(entry(strings::DEVICE_NAME, &c.device, |c, t| c.device = t))
        .child(muted(strings::AUTO_SYNC))
        .child(segmented(
            vec![(true, strings::ON), (false, strings::OFF)],
            move || draft.get().auto,
            move |on| draft.update(|c| c.auto = on),
        ))
        // The desktops also sync every few minutes; 0 never.
        .child(is_desktop().then(|| {
            let minutes = input(strings::SYNC_MINUTES, &c.interval.to_string(), move |t| {
                if let Ok(minutes) = t.trim().parse() {
                    draft.update(|c| c.interval = minutes);
                }
            });
            labeled(strings::SYNC_MINUTES, minutes)
        }))
}

/// A field holding `value` at first; `typed` hears each change.
fn input(label: &'static str, value: &str, typed: impl Fn(String) + 'static) -> nsh_base::Input {
    field(label)
        .label(label)
        .default_value(value.to_owned())
        .on_change(move |text: &str| typed(text.to_owned()))
}

/// `input` with its `label` over it.
fn labeled(label: &'static str, input: impl IntoElement) -> View {
    view()
        .w_full()
        .max_w(px(420.0))
        .gap_1()
        .child(muted(label))
        .child(input)
}

/// Asks before a run deletes `count` files: Delete runs it, Cancel, Escape
/// or the backdrop do not.
pub(crate) fn deletions_dialog(count: usize) -> impl IntoElement {
    let cancel = || store::confirm_deletions(false);
    dialog()
        .animation(ModalAnimation::Fade)
        .on_cancel(cancel)
        .flex_1()
        .child(view().absolute().inset_0().bg(overlay).on_press(cancel))
        .child(
            card()
                .m(auto())
                .w_80()
                .gap_3()
                .p_5()
                .shadow_lg()
                .child(
                    text(strings::DELETE_FILES)
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(move || theme().foreground),
                )
                .child(muted(strings::deleting(count)))
                .child(
                    view()
                        .flex_row()
                        .justify_end()
                        .gap_2()
                        .child(action(strings::CANCEL, cancel))
                        .child(primary(strings::DELETE, || store::confirm_deletions(true))),
                ),
        )
}
