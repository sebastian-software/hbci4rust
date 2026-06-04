use std::fs;
use std::path::Path;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{Key, XChaCha20Poly1305};
use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::passport::PinTanPassportData;

const FORMAT: &str = "hbci4rust-pintan-passport";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u8,
    kdf: KdfParams,
    aead: String,
    salt: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KdfParams {
    algorithm: String,
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
}

pub struct PassportStorage;

impl PassportStorage {
    pub fn save_to_vec(data: &PinTanPassportData, passphrase: &[u8]) -> HbciResult<Vec<u8>> {
        if passphrase.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "passport passphrase must not be empty",
            ));
        }

        let kdf = KdfParams {
            algorithm: "argon2id".to_owned(),
            memory_cost_kib: 19 * 1024,
            time_cost: 2,
            parallelism: 1,
        };

        let mut salt = [0_u8; SALT_LEN];
        OsRng.fill_bytes(&mut salt);

        let mut key_bytes = derive_key(passphrase, &salt, &kdf)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let plaintext = serde_json::to_vec(data).map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Storage,
                "failed to serialize PinTAN passport",
                err,
            )
        })?;
        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_slice())
            .map_err(|err| {
                HbciError::with_source(HbciErrorKind::Storage, "failed to encrypt passport", err)
            })?;
        key_bytes.zeroize();

        let envelope = Envelope {
            format: FORMAT.to_owned(),
            version: VERSION,
            kdf,
            aead: "xchacha20poly1305".to_owned(),
            salt: salt.to_vec(),
            nonce: nonce.to_vec(),
            ciphertext,
        };

        serde_json::to_vec_pretty(&envelope).map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Storage,
                "failed to serialize passport envelope",
                err,
            )
        })
    }

    pub fn load_from_slice(bytes: &[u8], passphrase: &[u8]) -> HbciResult<PinTanPassportData> {
        let envelope: Envelope = serde_json::from_slice(bytes).map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Storage,
                "failed to parse passport envelope",
                err,
            )
        })?;

        if envelope.format != FORMAT || envelope.version != VERSION {
            return Err(HbciError::new(
                HbciErrorKind::Storage,
                "unsupported passport envelope format",
            ));
        }

        let mut key_bytes = derive_key(passphrase, &envelope.salt, &envelope.kdf)?;
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&key_bytes));
        let plaintext = cipher
            .decrypt(
                envelope.nonce.as_slice().into(),
                envelope.ciphertext.as_slice(),
            )
            .map_err(|err| {
                HbciError::with_source(HbciErrorKind::Storage, "failed to decrypt passport", err)
            })?;
        key_bytes.zeroize();

        serde_json::from_slice(&plaintext).map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Storage,
                "failed to deserialize PinTAN passport",
                err,
            )
        })
    }

    pub fn save_file(
        path: impl AsRef<Path>,
        data: &PinTanPassportData,
        passphrase: &[u8],
    ) -> HbciResult<()> {
        let bytes = Self::save_to_vec(data, passphrase)?;
        fs::write(path, bytes).map_err(|err| {
            HbciError::with_source(HbciErrorKind::Storage, "failed to write passport file", err)
        })
    }

    pub fn load_file(path: impl AsRef<Path>, passphrase: &[u8]) -> HbciResult<PinTanPassportData> {
        let bytes = fs::read(path).map_err(|err| {
            HbciError::with_source(HbciErrorKind::Storage, "failed to read passport file", err)
        })?;
        Self::load_from_slice(&bytes, passphrase)
    }
}

fn derive_key(passphrase: &[u8], salt: &[u8], params: &KdfParams) -> HbciResult<[u8; KEY_LEN]> {
    if params.algorithm != "argon2id" {
        return Err(HbciError::new(
            HbciErrorKind::Storage,
            "unsupported passport KDF",
        ));
    }

    let params = Params::new(
        params.memory_cost_kib,
        params.time_cost,
        params.parallelism,
        Some(KEY_LEN),
    )
    .map_err(|err| {
        HbciError::with_source(HbciErrorKind::Storage, "invalid Argon2id parameters", err)
    })?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0_u8; KEY_LEN];
    argon2
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|err| HbciError::with_source(HbciErrorKind::Storage, "Argon2id failed", err))?;
    Ok(key)
}
