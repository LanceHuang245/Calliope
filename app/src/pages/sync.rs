//! Sync in the settings (MVP 8.7): the service and its settings, checking
//! the connection, syncing on its own, and how the last run went.

use calliope_sync::{Config, Provider};
use nsh_core::prelude::*;

use super::settings::settings_segmented;

use crate::{
    components::{ButtonVariant, button, field, labeled, muted, segmented},
    store::{self, SyncStatus},
    strings,
};

pub(crate) fn sync_settings() -> impl IntoElement {
    // The settings as edited, kept by Save. The service is a signal of its
    // own: changing it shows other fields, typing in one does not.
    let saved = untracked(store::sync_config);
    let provider = signal(saved.provider);
    let mode = secret_fields(saved.encrypted);
    let draft = signal(saved);
    let secret = signal(String::new());
    let edited = move || Config {
        provider: provider.get(),
        encrypted: mode.encrypted(),
        ..draft.get()
    };
    view()
        .w_full()
        .gap_2()
        .items_start()
        .child(labeled(
            strings::SYNC_PROVIDER,
            false,
            settings_segmented(
                vec![
                    (Provider::Off, strings::OFF),
                    (Provider::WebDav, strings::WEBDAV),
                    (Provider::S3, strings::S3),
                ],
                move || provider.get(),
                move |p| provider.set(p),
            ),
        ))
        .child(dynamic(move || {
            let on = provider.get() != Provider::Off;
            untracked(|| {
                on.then(|| {
                    view()
                        .w_full()
                        .gap_3()
                        .child(fields(provider.get(), draft, secret, true))
                        .child(encryption_fields(mode, true))
                })
            })
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
                    let config = edited();
                    if config.provider == Provider::Off {
                        store::save_sync(config, secret.get(), String::new(), false);
                    } else if let Some((passphrase, new)) = mode.submitted() {
                        store::save_sync(config, secret.get(), passphrase, new);
                    }
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
                                move || {
                                    let (passphrase, new) = mode.for_test();
                                    store::test_sync(edited(), secret.get(), passphrase, new);
                                },
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
            store::connecting().then(|| muted(strings::CONNECTING))
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
    in_settings: bool,
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
    let choices = vec![(true, strings::ON), (false, strings::OFF)];
    let auto = move || draft.get().auto;
    let set_auto = move |on| draft.update(|c| c.auto = on);
    let auto_picker = if in_settings {
        settings_segmented(choices, auto, set_auto)
    } else {
        segmented(choices, auto, set_auto).into_element()
    };
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
        .child(labeled(strings::AUTO_SYNC, false, auto_picker))
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum RemoteMode {
    Plaintext,
    New,
    Join,
}

#[derive(Clone, Copy)]
pub(crate) struct EncryptionFields {
    mode: Signal<RemoteMode>,
    passphrase: Signal<String>,
    confirmation: Signal<String>,
}

pub(crate) fn secret_fields(encrypted: bool) -> EncryptionFields {
    EncryptionFields {
        mode: signal(if encrypted {
            RemoteMode::Join
        } else {
            RemoteMode::Plaintext
        }),
        passphrase: signal(String::new()),
        confirmation: signal(String::new()),
    }
}

impl EncryptionFields {
    pub(crate) fn encrypted(self) -> bool {
        self.mode.get() != RemoteMode::Plaintext
    }

    pub(crate) fn for_test(self) -> (String, bool) {
        let mode = self.mode.get();
        let passphrase = if self.encrypted() {
            self.passphrase.get()
        } else {
            String::new()
        };
        (passphrase, mode == RemoteMode::New)
    }

    fn submitted(self) -> Option<(String, bool)> {
        let (passphrase, new) = self.for_test();
        if new && (passphrase.is_empty() || passphrase != self.confirmation.get()) {
            store::notify(strings::PASSPHRASE_MISMATCH);
            return None;
        }
        Some((passphrase, new))
    }
}

/// New remotes confirm the passphrase. Joining an existing remote asks once.
pub(crate) fn encryption_fields(fields: EncryptionFields, allow_new: bool) -> impl IntoElement {
    let mut modes = vec![(RemoteMode::Plaintext, strings::PLAINTEXT_REMOTE)];
    if allow_new {
        modes.push((RemoteMode::New, strings::NEW_ENCRYPTED_REMOTE));
    }
    modes.push((RemoteMode::Join, strings::JOIN_ENCRYPTED_REMOTE));
    let current = move || fields.mode.get();
    let choose = move |mode| {
        fields.passphrase.set(String::new());
        fields.confirmation.set(String::new());
        fields.mode.set(mode);
    };
    let picker = if allow_new {
        settings_segmented(modes, current, choose)
    } else {
        segmented(modes, current, choose).into_element()
    };
    view()
        .gap_2()
        .items_start()
        .child(muted(strings::REMOTE_MODE))
        .child(picker)
        .child(dynamic(move || {
            let mode = fields.mode.get();
            (mode != RemoteMode::Plaintext).then(|| {
                view()
                    .gap_2()
                    .child(labeled(
                        strings::ENCRYPTION_PASSPHRASE,
                        true,
                        input(strings::ENCRYPTION_PASSPHRASE, "", move |typed| {
                            fields.passphrase.set(typed);
                        })
                        .masked(true),
                    ))
                    .child((mode == RemoteMode::New).then(|| {
                        view()
                            .gap_2()
                            .child(muted(strings::PASSPHRASE_LOSS))
                            .child(labeled(
                                strings::CONFIRM_PASSPHRASE,
                                true,
                                input(strings::CONFIRM_PASSPHRASE, "", move |typed| {
                                    fields.confirmation.set(typed);
                                })
                                .masked(true),
                            ))
                    }))
            })
        }))
}
