use super::*;

#[tokio::test]
async fn wrong_passphrase_keeps_both_sides_intact() {
    for (mut config, secret) in server_configs("wrong-passphrase") {
        config.encrypted = true;
        let op = remote::operator(&config, &secret).await.unwrap();
        op.delete_with("").recursive(true).await.unwrap();
        crate::test(&config, &secret, PASSPHRASE, true)
            .await
            .unwrap();
        assert!(remote::entries(&op).await.unwrap().is_empty());
        crate::prepare(&config, &secret, PASSPHRASE, true)
            .await
            .unwrap();
        assert!(matches!(
            crate::test(&config, &secret, "incorrect", false).await,
            Err(Error::WrongPassphrase)
        ));
        let a = device("wrong-passphrase", "a");
        write(&a, "资料/Note.md", "before");
        write(&a, "资料/Empty.md", "");
        a.write_bytes("资料/attachment.bin", &vec![0xa5; 131_077])
            .unwrap();
        crate::sync(&a, &config, &secret, PASSPHRASE, true, |_, _| {})
            .await
            .unwrap();

        write(&a, "资料/Note.md", "new local edit");
        let state = a.read(".calliope/sync/state.json").unwrap();
        let remote = remote::entries(&op).await.unwrap();
        let mut before = BTreeMap::new();
        for entry in remote.into_iter().filter(|e| e.metadata().is_file()) {
            assert!(!entry.path().contains("资料") && !entry.path().contains("Note"));
            let data = op.read(entry.path()).await.unwrap().to_vec();
            assert!(data.starts_with(b"RCLONE\0\0"));
            before.insert(entry.path().to_owned(), data);
        }
        assert!(matches!(
            crate::sync(&a, &config, &secret, "incorrect", true, |_, _| {}).await,
            Err(Error::WrongPassphrase)
        ));
        assert_eq!(a.read("资料/Note.md").unwrap(), "new local edit");
        assert_eq!(a.read(".calliope/sync/state.json").unwrap(), state);
        assert_eq!(
            remote::entries(&op)
                .await
                .unwrap()
                .into_iter()
                .filter(|e| e.metadata().is_file())
                .count(),
            before.len()
        );
        for (path, bytes) in before {
            assert_eq!(op.read(&path).await.unwrap().to_vec(), bytes);
        }

        config.encrypted = false;
        assert!(matches!(
            crate::test(&config, &secret, "", false).await,
            Err(Error::EncryptedRemote)
        ));
    }
}

#[tokio::test]
async fn competing_setup_cannot_replace_a_verifier() {
    for (mut config, secret) in server_configs("competing-setup") {
        config.encrypted = true;
        let other = "another prospective passphrase";
        let (first, second) = tokio::join!(
            crate::prepare(&config, &secret, PASSPHRASE, true),
            crate::prepare(&config, &secret, other, true),
        );
        assert_ne!(first.is_ok(), second.is_ok(), "{first:?} / {second:?}");
        let (winner, loser) = if first.is_ok() {
            (PASSPHRASE, other)
        } else {
            (other, PASSPHRASE)
        };
        crate::test(&config, &secret, winner, false).await.unwrap();
        assert!(matches!(
            crate::test(&config, &secret, loser, false).await,
            Err(Error::WrongPassphrase)
        ));
    }
}
