use aes_gcm::aead::{consts::U12, Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

pub const KEY_BYTES: usize = 32;
pub const KEY_ID_BYTES: usize = 16;
pub const NONCE_PREFIX_BYTES: usize = 4;
pub const NONCE_BYTES: usize = 12;
pub const AUTH_TAG_BYTES: usize = 16;
pub const ENCRYPTION_SUITE_AES_256_GCM: u8 = 1;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption key file is not a regular file")]
    InvalidKeyFile,
    #[error("encryption key file must contain exactly 32 bytes")]
    InvalidKeyLength,
    #[error("encryption key file already exists: {0}")]
    KeyFileExists(String),
    #[error("encryption authentication failed")]
    AuthenticationFailed,
    #[error("encryption key or nonce is invalid")]
    InvalidParameters,
    #[error("randomness provider failed")]
    RandomnessFailed,
    #[error("key-file I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct EncryptionKey([u8; KEY_BYTES]);

impl std::fmt::Debug for EncryptionKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("EncryptionKey(REDACTED)")
    }
}

impl EncryptionKey {
    pub fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn load_file(path: &Path) -> Result<Self, CryptoError> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.file_type().is_file() {
            return Err(CryptoError::InvalidKeyFile);
        }
        let mut file = File::open(path)?;
        let mut bytes = [0u8; KEY_BYTES];
        file.read_exact(&mut bytes)
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        let mut extra = [0u8; 1];
        if file.read(&mut extra)? != 0 {
            return Err(CryptoError::InvalidKeyLength);
        }
        Ok(Self(bytes))
    }

    pub fn generate_file(path: &Path) -> Result<(), CryptoError> {
        if fs::symlink_metadata(path).is_ok() {
            return Err(CryptoError::KeyFileExists(path.display().to_string()));
        }
        let mut bytes = [0u8; KEY_BYTES];
        aes_gcm::aead::rand_core::RngCore::try_fill_bytes(&mut OsRng, &mut bytes)
            .map_err(|_| CryptoError::RandomnessFailed)?;
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(())
    }

    pub(crate) fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new_from_slice(&self.0).expect("EncryptionKey always has 32 bytes")
    }
}

pub fn random_key_id() -> Result<[u8; KEY_ID_BYTES], CryptoError> {
    let mut key_id = [0u8; KEY_ID_BYTES];
    aes_gcm::aead::rand_core::RngCore::try_fill_bytes(&mut OsRng, &mut key_id)
        .map_err(|_| CryptoError::RandomnessFailed)?;
    Ok(key_id)
}

pub fn random_nonce_prefix() -> Result<[u8; NONCE_PREFIX_BYTES], CryptoError> {
    let mut prefix = [0u8; NONCE_PREFIX_BYTES];
    aes_gcm::aead::rand_core::RngCore::try_fill_bytes(&mut OsRng, &mut prefix)
        .map_err(|_| CryptoError::RandomnessFailed)?;
    Ok(prefix)
}

fn nonce(prefix: &[u8; NONCE_PREFIX_BYTES], sequence: u64) -> Nonce<U12> {
    let mut bytes = [0u8; NONCE_BYTES];
    bytes[..NONCE_PREFIX_BYTES].copy_from_slice(prefix);
    bytes[NONCE_PREFIX_BYTES..].copy_from_slice(&sequence.to_be_bytes());
    *Nonce::from_slice(&bytes)
}

pub fn authenticate_header(
    key: &EncryptionKey,
    key_id: &[u8; KEY_ID_BYTES],
    nonce_prefix: &[u8; NONCE_PREFIX_BYTES],
    associated_data: &[u8],
) -> Result<[u8; AUTH_TAG_BYTES], CryptoError> {
    let mut aad = Vec::with_capacity(associated_data.len() + KEY_ID_BYTES);
    aad.extend_from_slice(associated_data);
    aad.extend_from_slice(key_id);
    let encrypted = key
        .cipher()
        .encrypt(
            &nonce(nonce_prefix, 0),
            aes_gcm::aead::Payload {
                msg: &[],
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::AuthenticationFailed)?;
    encrypted
        .try_into()
        .map_err(|_| CryptoError::InvalidParameters)
}

pub fn verify_header(
    key: &EncryptionKey,
    key_id: &[u8; KEY_ID_BYTES],
    nonce_prefix: &[u8; NONCE_PREFIX_BYTES],
    associated_data: &[u8],
    expected_tag: &[u8; AUTH_TAG_BYTES],
) -> Result<(), CryptoError> {
    let actual = authenticate_header(key, key_id, nonce_prefix, associated_data)?;
    if &actual == expected_tag {
        Ok(())
    } else {
        Err(CryptoError::AuthenticationFailed)
    }
}

pub fn encrypt_record(
    key: &EncryptionKey,
    key_id: &[u8; KEY_ID_BYTES],
    nonce_prefix: &[u8; NONCE_PREFIX_BYTES],
    sequence: u64,
    associated_data: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if sequence == 0 {
        return Err(CryptoError::InvalidParameters);
    }
    let mut aad = Vec::with_capacity(associated_data.len() + KEY_ID_BYTES);
    aad.extend_from_slice(associated_data);
    aad.extend_from_slice(key_id);
    key.cipher()
        .encrypt(
            &nonce(nonce_prefix, sequence),
            aes_gcm::aead::Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::AuthenticationFailed)
}

pub fn decrypt_record(
    key: &EncryptionKey,
    key_id: &[u8; KEY_ID_BYTES],
    nonce_prefix: &[u8; NONCE_PREFIX_BYTES],
    sequence: u64,
    associated_data: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if sequence == 0 {
        return Err(CryptoError::InvalidParameters);
    }
    let mut aad = Vec::with_capacity(associated_data.len() + KEY_ID_BYTES);
    aad.extend_from_slice(associated_data);
    aad.extend_from_slice(key_id);
    key.cipher()
        .decrypt(
            &nonce(nonce_prefix, sequence),
            aes_gcm::aead::Payload {
                msg: ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| CryptoError::AuthenticationFailed)
}
