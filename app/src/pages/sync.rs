//! Sync in the settings (MVP 8.7): the service and its settings, checking
//! the connection, syncing on its own, and how the last run went.

use calliope_sync::{Config, Provider};
use nsh_core::prelude::*;

use crate::{
    components::{ButtonVariant, button, field, labeled, muted, section, segmented},
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
        // Only show Save when sync is off; Test connection and Sync now apply to active providers.
        .child(
            view()
                .flex_row()
                .flex_wrap()
                .items_center()
                .gap_2()
                .pt_2()
                .child(button(ButtonVariant::Primary, strings::SAVE, move || {
                    store::save_sync(edited(), &secret.get())
                }))
                .child(dynamic(move || {
                    (provider.get() != Provider::Off).then(|| {
                        view()
                            .flex_row()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(button(
                                ButtonVariant::Secondary,
                                strings::TEST_CONNECTION,
                                move || store::test_sync(edited(), secret.get()),
                            ))
                            .child(button(
                                ButtonVariant::Secondary,
                                strings::SYNC_NOW,
                                store::sync_now,
                            ))
                    })
                })),
        )
        .child(dynamic(move || {
            (provider.get() != Provider::Off).then(|| {
                muted(move || match store::sync_status() {
                    SyncStatus::Idle => strings::NOT_SYNCED.to_owned(),
                    SyncStatus::Running(done, total) => strings::syncing(done, total),
                    SyncStatus::Done(time, 0) => strings::synced_at(&time),
                    SyncStatus::Done(time, n) => strings::synced_conflicts(&time, n),
                    SyncStatus::Failed(error) => strings::sync_failed(&error),
                })
            })
        }))
        .child(dynamic(|| {
            store::secret_in_file().then(|| muted(strings::SECRET_IN_FILE))
        }))
}

/// The settings of the service `provider`, typed into `draft`; the password
/// or secret key into `secret`. A phone's vault made from sync asks for
/// them too (`welcome`).
pub(crate) fn fields(
    provider: Provider,
    draft: Signal<Config>,
    secret: Signal<String>,
) -> impl IntoElement {
    let c = draft.get();
    let s3 = provider == Provider::S3;
    let entry = move |label, required, value: &str, set: fn(&mut Config, String)| {
        labeled(
            label,
            required,
            input(label, value, move |typed| draft.update(|c| set(c, typed))),
        )
    };
    let secret_label = match s3 {
        true => strings::SECRET_KEY,
        false => strings::PASSWORD,
    };
    // A saved password stays while its field is left empty.
    let kept = untracked(store::sync_config).provider != Provider::Off;
    // Memoize so typing in any field does not rebuild the interval input.
    let show_interval = computed(move || is_desktop() && draft.get().auto);
    view()
        .w_full()
        .pt_2()
        .gap_3()
        .items_start()
        // Required: the server, the bucket, who signs in, and the password
        // or secret key until one is saved. S3's region and where its bucket
        // goes (path or host) are found out from the endpoint (calliope-sync).
        .child(match s3 {
            true => entry(strings::ENDPOINT, true, &c.endpoint, |c, t| c.endpoint = t),
            false => entry(strings::SERVER_ADDRESS, true, &c.endpoint, |c, t| {
                c.endpoint = t
            }),
        })
        .child(s3.then(|| entry(strings::BUCKET, true, &c.bucket, |c, t| c.bucket = t)))
        .child(match s3 {
            true => entry(strings::ACCESS_KEY, true, &c.user, |c, t| c.user = t),
            false => entry(strings::USER_NAME, true, &c.user, |c, t| c.user = t),
        })
        .child(
            labeled(
                secret_label,
                !kept,
                input(secret_label, "", move |typed| secret.set(typed)).masked(true),
            )
            .child(kept.then(|| muted(strings::SECRET_KEPT))),
        )
        .child(match s3 {
            true => entry(strings::PATH_PREFIX, false, &c.root, |c, t| c.root = t),
            false => entry(strings::REMOTE_FOLDER, false, &c.root, |c, t| c.root = t),
        })
        // A server with a certificate of its own (a NAS): trusted by the
        // fingerprint a failed connection shows.
        .child(
            entry(strings::CERTIFICATE, false, &c.certificate, |c, t| {
                c.certificate = t
            })
            .child(muted(strings::CERTIFICATE_HINT)),
        )
        .child(entry(strings::DEVICE_NAME, false, &c.device, |c, t| {
            c.device = t
        }))
        .child(labeled(
            strings::AUTO_SYNC,
            false,
            segmented(
                vec![(true, strings::ON), (false, strings::OFF)],
                move || draft.get().auto,
                move |on| draft.update(|c| c.auto = on),
            ),
        ))
        // The desktops also sync every few minutes while auto-sync is on; 0 never.
        .child(dynamic(move || {
            let show = show_interval.get();
            untracked(|| {
                show.then(|| {
                    let current = draft.get().interval.to_string();
                    let minutes = input(strings::SYNC_MINUTES, &current, move |t| {
                        let trimmed = t.trim();
                        if trimmed.is_empty() {
                            draft.update(|c| c.interval = 0);
                        } else if let Ok(minutes) = trimmed.parse() {
                            draft.update(|c| c.interval = minutes);
                        }
                    });
                    labeled(strings::SYNC_MINUTES, false, minutes)
                })
            })
        }))
}

/// A field holding `value` at first; `typed` hears each change.
fn input(label: &'static str, value: &str, typed: impl Fn(String) + 'static) -> nsh_base::Input {
    field(label)
        .label(label)
        .default_value(value.to_owned())
        .on_change(move |text: &str| typed(text.to_owned()))
}
