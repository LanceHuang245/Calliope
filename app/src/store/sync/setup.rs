//! Checks a remote before saving encrypted sync settings or joining it.

use super::*;

pub(crate) fn save_sync(mut config: Config, secret: String, passphrase: String, new: bool) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    let saved = untracked(sync_config);
    if changes_mode(&saved, &config) {
        return notify(strings::REMOTE_MODE_FIXED);
    }
    if untracked(connecting) || RUNNING.get() {
        return;
    }
    if config.provider == Provider::Off {
        config = Config {
            provider: Provider::Off,
            ..saved.clone()
        };
    }
    if config.provider == Provider::Off || (!config.encrypted && !saved.encrypted) {
        if keep_sync(config, &secret) {
            inform(strings::SYNC_SAVED);
        }
        return;
    }

    // Setup may initialize an empty remote, while testing a connection is read-only.
    set(Connecting(true));
    let account = account(&vault);
    let checked = config.clone();
    let submitted = secret.clone();
    spawn_task(async move {
        let checked = background(async move {
            let (secret, passphrase) = credentials(&account, &checked, submitted, passphrase);
            calliope_sync::test(&checked, &secret, &passphrase, new).await?;
            if checked.encrypted
                && !secrets::set_passphrase(&crypt_account(&account, &checked), &passphrase)
            {
                return Err(std::io::Error::other(strings::PASSPHRASE_STORAGE).into());
            }
            if new {
                calliope_sync::prepare(&checked, &secret, &passphrase, true).await?;
            }
            Ok(())
        })
        .await;
        set(Connecting(false));
        if report_check(checked)
            && untracked(super::vault).as_ref() == Some(&vault)
            && keep_sync(config, &secret)
        {
            inform(strings::SYNC_SAVED);
        }
    });
}

/// Turning sync off retains the stored mode. Changing mode requires a new root.
fn changes_mode(saved: &Config, config: &Config) -> bool {
    config.provider != Provider::Off
        && saved.encrypted != config.encrypted
        && !saved.endpoint.is_empty()
        && (saved.provider == config.provider || saved.provider == Provider::Off)
        && saved.endpoint.trim_end_matches('/') == config.endpoint.trim_end_matches('/')
        && saved.bucket == config.bucket
        && saved.root.trim_matches('/') == config.root.trim_matches('/')
}

/// Joining from the welcome screen checks the passphrase before creating a vault.
pub(crate) fn create_from_sync(name: String, config: Config, secret: String, passphrase: String) {
    if name.trim().is_empty() || untracked(connecting) {
        return;
    }
    let Some(root) = crate::paths::sandbox_vault(name.trim()) else {
        return;
    };
    if config.encrypted && root.exists() {
        return notify(strings::SYNC_VAULT_EXISTS);
    }
    set(Connecting(true));
    let (checked, key, path) = (config.clone(), secret.clone(), root.clone());
    spawn_task(async move {
        let tested = background(async move {
            calliope_sync::test(&checked, &key, &passphrase, false).await?;
            if checked.encrypted
                && !secrets::set_passphrase(
                    &crypt_account(&path.to_string_lossy(), &checked),
                    &passphrase,
                )
            {
                return Err(std::io::Error::other(strings::PASSPHRASE_STORAGE).into());
            }
            Ok(())
        })
        .await;
        set(Connecting(false));
        if report_check(tested) {
            create_vault(&name);
            if untracked(vault).is_some_and(|vault| vault.root() == root)
                && keep_sync(config, &secret)
            {
                start(false);
            }
        }
    });
}

pub(crate) fn test_sync(config: Config, secret: String, passphrase: String, new: bool) {
    let Some(vault) = untracked(vault) else {
        return;
    };
    let account = account(&vault);
    spawn_task(async move {
        let tested = background(async move {
            let (secret, passphrase) = credentials(&account, &config, secret, passphrase);
            calliope_sync::test(&config, &secret, &passphrase, new).await
        });
        if report_check(tested.await) {
            inform(strings::SYNC_WORKS);
        }
    });
}

fn credentials(
    account: &str,
    config: &Config,
    mut secret: String,
    mut passphrase: String,
) -> (String, String) {
    if secret.is_empty() {
        secret = secrets::get(account).unwrap_or_default();
    }
    if config.encrypted && passphrase.is_empty() {
        passphrase = secrets::passphrase(&crypt_account(account, config)).unwrap_or_default();
    }
    (secret, passphrase)
}

fn report_check<E: std::fmt::Display>(tested: Result<calliope_sync::Result<()>, E>) -> bool {
    match tested {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            notify(strings::sync_failed(&e));
            false
        }
        Err(e) => {
            notify(e.to_string());
            false
        }
    }
}
