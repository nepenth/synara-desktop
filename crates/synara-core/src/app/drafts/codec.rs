//! Credential-free `in.synara.draft` envelope: XChaCha20-Poly1305 over a
//! padded JSON payload, bound to the account and room through the AAD.
//!
//! Design: `docs/design/cross-device-drafts.md`.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};
use hmac::{Hmac, Mac};
use matrix_sdk::ruma::serde::Base64;

type B64 = Base64;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::Zeroizing;

use super::{
    SynaraDraftsPayload, DRAFTS_ALGORITHM, DRAFTS_ENVELOPE_VERSION, DRAFT_PADDING_BUCKET_BYTES,
    MAX_DRAFT_ENVELOPE_BYTES, MAX_DRAFT_PAYLOAD_BYTES,
};

const AAD_DOMAIN: &[u8] = b"in.synara.draft.v1";
const KEY_ID_INFO: &[u8] = b"in.synara.draft.key_id";
pub const DRAFTS_KEY_BYTES: usize = 32;
const KEY_ID_BYTES: usize = 8;
const NONCE_BYTES: usize = 24;

/// The account's draft-sync key. Never serialized into logs or DTOs.
#[derive(Clone)]
pub struct DraftsKey(Zeroizing<[u8; DRAFTS_KEY_BYTES]>);

impl std::fmt::Debug for DraftsKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DraftsKey(..)")
    }
}

impl PartialEq for DraftsKey {
    fn eq(&self, other: &Self) -> bool {
        self.key_id() == other.key_id()
    }
}

impl DraftsKey {
    pub fn generate() -> Self {
        let mut bytes = Zeroizing::new([0u8; DRAFTS_KEY_BYTES]);
        getrandom::fill(bytes.as_mut()).expect("drafts key entropy");
        Self(bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let bytes: [u8; DRAFTS_KEY_BYTES] =
            bytes.try_into().map_err(|_| "drafts-key-invalid-length")?;
        Ok(Self(Zeroizing::new(bytes)))
    }

    /// Unpadded standard base64, the encoding SSSS secrets and to-device
    /// shares carry.
    pub fn from_base64(encoded: &str) -> Result<Self, &'static str> {
        let decoded = Zeroizing::new(
            B64::parse(encoded.trim())
                .map_err(|_| "drafts-key-invalid-encoding")?
                .into_inner(),
        );
        Self::from_bytes(&decoded)
    }

    pub(super) fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn to_base64(&self) -> Zeroizing<String> {
        Zeroizing::new(B64::new(self.0.to_vec()).encode())
    }

    /// Stable public identifier so a device can tell "sealed under a key I
    /// lack" from "corrupt". It reveals nothing about the key.
    pub fn key_id(&self) -> String {
        let mut mac = <Hmac<Sha256> as hmac::KeyInit>::new_from_slice(self.0.as_ref())
            .expect("HMAC accepts any key length");
        mac.update(KEY_ID_INFO);
        let digest = mac.finalize().into_bytes();
        B64::new(digest[..KEY_ID_BYTES].to_vec()).encode()
    }

    fn cipher(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new_from_slice(self.0.as_ref()).expect("32-byte key")
    }
}

/// What the homeserver stores. Everything but `v`/`alg`/`key_id` is opaque.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftEnvelope {
    pub v: u32,
    pub alg: String,
    pub key_id: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftOpenError {
    /// Sealed under a different key (another identity, or one we lack).
    UnknownKey,
    /// Version or algorithm this build does not understand.
    Unsupported,
    /// Tampered, moved to another room/account, or malformed.
    Invalid,
}

impl DraftOpenError {
    pub fn diagnostic(self) -> &'static str {
        match self {
            Self::UnknownKey => "drafts-envelope-unknown-key",
            Self::Unsupported => "drafts-envelope-unsupported",
            Self::Invalid => "drafts-envelope-invalid",
        }
    }
}

fn aad(user_id: &str, room_id: &str) -> Vec<u8> {
    let mut aad = Vec::with_capacity(AAD_DOMAIN.len() + user_id.len() + room_id.len() + 2);
    aad.extend_from_slice(AAD_DOMAIN);
    aad.push(0);
    aad.extend_from_slice(user_id.as_bytes());
    aad.push(0);
    aad.extend_from_slice(room_id.as_bytes());
    aad
}

/// Pad with JSON whitespace to the next bucket so the server learns only a
/// coarse size. `serde_json` ignores trailing whitespace on read.
fn pad(mut plaintext: Vec<u8>) -> Vec<u8> {
    let bucket = DRAFT_PADDING_BUCKET_BYTES;
    let padded = plaintext.len().div_ceil(bucket).max(1) * bucket;
    plaintext.resize(padded, b' ');
    plaintext
}

/// Encode a payload that already fits [`MAX_DRAFT_PAYLOAD_BYTES`]; callers cap
/// it first with [`super::cap_payload`].
pub fn seal_payload(
    key: &DraftsKey,
    user_id: &str,
    room_id: &str,
    payload: &SynaraDraftsPayload,
) -> Result<DraftEnvelope, &'static str> {
    let plaintext =
        Zeroizing::new(serde_json::to_vec(payload).map_err(|_| "drafts-payload-serialize-failed")?);
    if plaintext.len() > MAX_DRAFT_PAYLOAD_BYTES {
        return Err("drafts-payload-too-large");
    }
    let padded = Zeroizing::new(pad(plaintext.to_vec()));
    let mut nonce = [0u8; NONCE_BYTES];
    getrandom::fill(&mut nonce).expect("drafts nonce entropy");
    let aad = aad(user_id, room_id);
    let ciphertext = key
        .cipher()
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: padded.as_ref(),
                aad: &aad,
            },
        )
        .map_err(|_| "drafts-seal-failed")?;
    Ok(DraftEnvelope {
        v: DRAFTS_ENVELOPE_VERSION,
        alg: DRAFTS_ALGORITHM.to_owned(),
        key_id: key.key_id(),
        nonce: B64::new(nonce.to_vec()).encode(),
        ciphertext: B64::new(ciphertext).encode(),
    })
}

/// Parse an envelope from the exact account-data JSON, bounded before parsing.
pub fn parse_envelope(raw_json: &str) -> Result<DraftEnvelope, DraftOpenError> {
    if raw_json.len() > MAX_DRAFT_ENVELOPE_BYTES {
        return Err(DraftOpenError::Invalid);
    }
    serde_json::from_str(raw_json).map_err(|_| DraftOpenError::Invalid)
}

pub fn open_envelope(
    key: &DraftsKey,
    user_id: &str,
    room_id: &str,
    envelope: &DraftEnvelope,
) -> Result<SynaraDraftsPayload, DraftOpenError> {
    if envelope.v != DRAFTS_ENVELOPE_VERSION || envelope.alg != DRAFTS_ALGORITHM {
        return Err(DraftOpenError::Unsupported);
    }
    if envelope.key_id != key.key_id() {
        return Err(DraftOpenError::UnknownKey);
    }
    let nonce = B64::parse(&envelope.nonce)
        .map_err(|_| DraftOpenError::Invalid)?
        .into_inner();
    let nonce: [u8; NONCE_BYTES] = nonce.try_into().map_err(|_| DraftOpenError::Invalid)?;
    let ciphertext = B64::parse(&envelope.ciphertext)
        .map_err(|_| DraftOpenError::Invalid)?
        .into_inner();
    let aad = aad(user_id, room_id);
    let plaintext = Zeroizing::new(
        key.cipher()
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| DraftOpenError::Invalid)?,
    );
    let value: serde_json::Value =
        serde_json::from_slice(&plaintext).map_err(|_| DraftOpenError::Invalid)?;
    super::normalize_payload(&value).ok_or(DraftOpenError::Invalid)
}
