//! rclone crypt's standard filename mode and SecretBox file format.

use aes::{
    Aes256,
    cipher::{Block, BlockCipherDecrypt, BlockCipherEncrypt, KeyInit},
};
use crypto_secretbox::{
    XSalsa20Poly1305,
    aead::{Aead, KeyInit as SecretBoxKeyInit},
};
use data_encoding::BASE32HEX_NOPAD;
use zeroize::Zeroizing;

use crate::{Error, Result};

const SALT: [u8; 16] = [
    0xa8, 0x0d, 0xf4, 0x3a, 0x8f, 0xbd, 0x03, 0x08, 0xa7, 0xca, 0xb8, 0x3e, 0x58, 0x1f, 0x86, 0xb1,
];
const MAGIC: &[u8; 8] = b"RCLONE\0\0";
const CHUNK: usize = 64 * 1024;
const TAG: usize = 16;

pub(crate) struct Crypt {
    data: XSalsa20Poly1305,
    names: Aes256,
    tweak: Zeroizing<u128>,
}

impl Crypt {
    pub(crate) fn new(passphrase: &str) -> Result<Self> {
        if passphrase.is_empty() {
            return Err(Error::MissingPassphrase);
        }
        // rclone's default salt allows recovery with just the passphrase.
        let mut key = Zeroizing::new([0; 80]);
        let params = scrypt::Params::new(14, 8, 1, 32).expect("rclone scrypt parameters");
        scrypt::scrypt(passphrase.as_bytes(), &SALT, &params, &mut *key)
            .expect("80 bytes of rclone key material");
        Ok(Self {
            data: XSalsa20Poly1305::new_from_slice(&key[..32]).unwrap(),
            names: Aes256::new_from_slice(&key[32..64]).unwrap(),
            tweak: Zeroizing::new(u128::from_le_bytes(key[64..].try_into().unwrap())),
        })
    }

    pub(crate) fn encode_path(&self, path: &str) -> Result<String> {
        path.split('/')
            .map(|name| {
                let mut bytes = name.as_bytes().to_vec();
                let pad = 16 - bytes.len() % 16;
                bytes.resize(bytes.len() + pad, pad as u8);
                self.eme(&mut bytes, true)?;
                Ok(BASE32HEX_NOPAD.encode(&bytes).to_ascii_lowercase())
            })
            .collect::<Result<Vec<_>>>()
            .map(|parts| parts.join("/"))
    }

    pub(crate) fn decode_path(&self, path: &str) -> Result<String> {
        path.split('/')
            .map(|name| {
                let mut bytes = BASE32HEX_NOPAD
                    .decode(name.to_ascii_uppercase().as_bytes())
                    .map_err(|_| Error::InvalidCiphertext)?;
                self.eme(&mut bytes, false)?;
                let pad = usize::from(*bytes.last().unwrap());
                if !(1..=16).contains(&pad)
                    || !bytes[bytes.len() - pad..]
                        .iter()
                        .all(|&b| usize::from(b) == pad)
                {
                    return Err(Error::InvalidCiphertext);
                }
                bytes.truncate(bytes.len() - pad);
                let part = String::from_utf8(bytes).map_err(|_| Error::InvalidCiphertext)?;
                // An encoded component must remain a single relative path component.
                if matches!(part.as_str(), "" | "." | "..")
                    || part.contains(['/', '\0'])
                    || (cfg!(windows) && part.contains(['\\', ':']))
                {
                    return Err(Error::InvalidCiphertext);
                }
                Ok(part)
            })
            .collect::<Result<Vec<_>>>()
            .map(|parts| parts.join("/"))
    }

    pub(crate) fn seal(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0; 24];
        getrandom::fill(&mut nonce).map_err(|_| Error::EncryptionRandomness)?;
        let mut encrypted = Vec::with_capacity(32 + data.len() + data.len().div_ceil(CHUNK) * TAG);
        encrypted.extend_from_slice(MAGIC);
        encrypted.extend_from_slice(&nonce);
        for chunk in data.chunks(CHUNK) {
            let block = self
                .data
                .encrypt((&nonce).into(), chunk)
                .map_err(|_| Error::InvalidCiphertext)?;
            encrypted.extend_from_slice(&block);
            increment(&mut nonce);
        }
        Ok(encrypted)
    }

    pub(crate) fn open(&self, encrypted: &[u8]) -> Result<Vec<u8>> {
        if !encrypted.starts_with(MAGIC) || encrypted.len() < 32 {
            return Err(Error::InvalidCiphertext);
        }
        let mut nonce: [u8; 24] = encrypted[8..32].try_into().unwrap();
        let mut data = Vec::with_capacity(encrypted.len() - 32);
        for chunk in encrypted[32..].chunks(CHUNK + TAG) {
            if chunk.len() <= TAG {
                return Err(Error::InvalidCiphertext);
            }
            let block = self
                .data
                .decrypt((&nonce).into(), chunk)
                .map_err(|_| Error::InvalidCiphertext)?;
            data.extend_from_slice(&block);
            increment(&mut nonce);
        }
        Ok(data)
    }

    // Halevi–Rogaway EME uses two AES passes and a mix between them.
    fn eme(&self, bytes: &mut [u8], encrypt: bool) -> Result<()> {
        if bytes.is_empty() || !bytes.len().is_multiple_of(16) || bytes.len() > 128 * 16 {
            return Err(Error::InvalidCiphertext);
        }
        let transform = |value: u128| {
            let mut block = Block::<Aes256>::from(value.to_le_bytes());
            if encrypt {
                self.names.encrypt_block(&mut block);
            } else {
                self.names.decrypt_block(&mut block);
            }
            u128::from_le_bytes(block.into())
        };
        let mut zero = Block::<Aes256>::default();
        self.names.encrypt_block(&mut zero);
        let initial = double(u128::from_le_bytes(zero.into()));
        let mut mask = initial;
        let mut sum = *self.tweak;
        let (blocks, _) = bytes.as_chunks_mut::<16>();
        for block in &mut *blocks {
            let value = transform(u128::from_le_bytes(*block) ^ mask);
            *block = value.to_le_bytes();
            sum ^= value;
            mask = double(mask);
        }

        let mixed = transform(sum);
        let mut delta = sum ^ mixed;
        let mut first = mixed ^ *self.tweak;
        for block in &mut blocks[1..] {
            delta = double(delta);
            let value = u128::from_le_bytes(*block) ^ delta;
            first ^= value;
            *block = value.to_le_bytes();
        }
        blocks[0] = first.to_le_bytes();

        mask = initial;
        for block in blocks {
            let value = transform(u128::from_le_bytes(*block)) ^ mask;
            *block = value.to_le_bytes();
            mask = double(mask);
        }
        Ok(())
    }
}

fn double(value: u128) -> u128 {
    (value << 1) ^ (0x87 & 0_u128.wrapping_sub(value >> 127))
}

// rclone increments the nonce as a little-endian 192-bit integer.
fn increment(nonce: &mut [u8; 24]) {
    for byte in nonce {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_paths_and_bytes_written_by_rclone() {
        let crypt = Crypt::new("Phase 5 compatibility test").unwrap();
        let name = "资料/a longer filename.md";
        let stored =
            "u275tdg65toph6fh7ho0e4ijk4/kpk7ofpvqidum7b1jh7u93p55mm2iqh8pb7l6f355696uq5t84sg";
        assert_eq!(crypt.encode_path(name).unwrap(), stored);
        assert_eq!(crypt.decode_path(stored).unwrap(), name);
        let data = data_encoding::HEXLOWER.decode(
            b"52434c4f4e45000023e1fa9e2f94af61014dfe233ddf39b4cebaa30c606799a52841ce1d97224f7d7d1bd21e53537b0e2e4ce13b42614d4c4b",
        ).unwrap();
        assert_eq!(crypt.open(&data).unwrap(), "# 你好\n".as_bytes());
    }

    #[test]
    fn round_trips_empty_and_chunked_attachments_and_rejects_tampering() {
        let crypt = Crypt::new("Phase 5 compatibility test").unwrap();
        for size in [0, CHUNK, CHUNK + 1, 2 * CHUNK + 5] {
            let original = vec![0xa5; size];
            let mut sealed = crypt.seal(&original).unwrap();
            assert_eq!(crypt.open(&sealed).unwrap(), original);
            if size > 0 {
                *sealed.last_mut().unwrap() ^= 1;
                assert!(matches!(crypt.open(&sealed), Err(Error::InvalidCiphertext)));
            }
        }
    }
}
