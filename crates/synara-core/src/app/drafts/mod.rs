//! Cross-device composer drafts in encrypted `in.synara.draft` room account
//! data.
//!
//! Authoritative design: `docs/design/cross-device-drafts.md`. This module is
//! the credential-free payload model, merge rules and size cap; `codec` seals
//! it, `key` owns the account key, and `live` is the Client-facing owner.

mod codec;
mod key;
mod live;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use codec::{
    open_envelope, parse_envelope, seal_payload, DraftEnvelope, DraftOpenError, DraftsKey,
    DRAFTS_KEY_BYTES,
};
pub use key::{
    DRAFTS_KEY_CACHE_KEY, DRAFTS_KEY_REQUEST_EVENT_TYPE, DRAFTS_KEY_SHARE_EVENT_TYPE,
    DRAFTS_SECRET_NAME,
};
pub(crate) use live::{on_identity_reset, on_secret_store_open};
pub use live::{
    DraftSyncEmit, DraftSyncOwner, NativeDraftDecision, NativeDraftKeyState, NativeDraftResolution,
    NativeDraftSyncStatus, NativeDraftUpdateSignal, NativeLocalDraftState, NativeRemoteDraft,
};

pub const DRAFTS_EVENT_TYPE: &str = "in.synara.draft";
pub const DRAFTS_ENVELOPE_VERSION: u32 = 1;
pub const DRAFTS_PAYLOAD_VERSION: u32 = 1;
pub const DRAFTS_ALGORITHM: &str = "in.synara.draft.v1.xchacha20poly1305";
pub const DRAFTS_UPDATED_EVENT: &str = "matrix-drafts-updated";
/// Server-side bound on the stored envelope (Synapse's event-size limit).
pub const MAX_DRAFT_ENVELOPE_BYTES: usize = 64 * 1024;
/// Plaintext bound: padding, AEAD tag and base64 must still fit the envelope.
pub const MAX_DRAFT_PAYLOAD_BYTES: usize = 46 * 1024;
pub const DRAFT_PADDING_BUCKET_BYTES: usize = 512;
pub const MAX_DRAFT_SLOTS: usize = 64;
pub const MAX_DRAFT_DEVICE_ID_BYTES: usize = 255;
pub const MAX_DRAFT_EVENT_ID_BYTES: usize = 255;
pub const DRAFT_TOMBSTONE_RETENTION_MS: u64 = 30 * 24 * 60 * 60 * 1000;
pub const MAIN_DRAFT_SLOT: &str = "main";
const THREAD_SLOT_PREFIX: &str = "thread:";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SynaraDraftsPayload {
    pub v: u32,
    pub slots: BTreeMap<String, SynaraDraftSlot>,
}

/// One composer's synced draft. A `cleared` slot is a tombstone: it keeps the
/// ordering fields so a send on one device clears the draft on the others.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SynaraDraftSlot {
    pub updated_at: u64,
    pub device_id: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cleared: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    /// Desktop's Slate value; other shells read `markdown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desktop_slate: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_of: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub truncated: bool,
}

impl SynaraDraftSlot {
    pub fn tombstone(updated_at: u64, device_id: &str) -> Self {
        Self {
            updated_at,
            device_id: device_id.to_owned(),
            cleared: true,
            ..Self::default()
        }
    }

    /// Total order for last-writer-wins: timestamp, then device id.
    pub fn is_newer_than(&self, other: &Self) -> bool {
        (self.updated_at, self.device_id.as_str()) > (other.updated_at, other.device_id.as_str())
    }

    fn is_empty_content(&self) -> bool {
        self.markdown.as_deref().is_none_or(|m| m.trim().is_empty())
            && self.reply_to.is_none()
            && self.edit_of.is_none()
    }
}

/// `main` or `thread:<root event id>`.
pub fn validate_slot_key(slot: &str) -> Result<(), &'static str> {
    if slot == MAIN_DRAFT_SLOT {
        return Ok(());
    }
    match slot.strip_prefix(THREAD_SLOT_PREFIX) {
        Some(root) if is_event_id(root) => Ok(()),
        _ => Err("drafts-invalid-slot"),
    }
}

fn is_event_id(value: &str) -> bool {
    value.starts_with('$')
        && value.len() > 1
        && value.len() <= MAX_DRAFT_EVENT_ID_BYTES
        && !value.chars().any(char::is_control)
}

fn normalize_slot(value: &serde_json::Value) -> Option<SynaraDraftSlot> {
    let mut slot: SynaraDraftSlot = serde_json::from_value(value.clone()).ok()?;
    if slot.device_id.is_empty() || slot.device_id.len() > MAX_DRAFT_DEVICE_ID_BYTES {
        return None;
    }
    if slot.reply_to.as_deref().is_some_and(|id| !is_event_id(id))
        || slot.edit_of.as_deref().is_some_and(|id| !is_event_id(id))
    {
        return None;
    }
    if slot.cleared {
        // A tombstone carries no content, whatever a peer wrote.
        slot = SynaraDraftSlot::tombstone(slot.updated_at, &slot.device_id);
    }
    Some(slot)
}

/// Fail-closed decode of a decrypted payload. Malformed slots are dropped;
/// a payload of the wrong shape or version is rejected whole.
pub fn normalize_payload(value: &serde_json::Value) -> Option<SynaraDraftsPayload> {
    let object = value.as_object()?;
    if object.get("v")?.as_u64()? != u64::from(DRAFTS_PAYLOAD_VERSION) {
        return None;
    }
    let slots = object.get("slots")?.as_object()?;
    let mut payload = SynaraDraftsPayload {
        v: DRAFTS_PAYLOAD_VERSION,
        slots: BTreeMap::new(),
    };
    for (key, slot) in slots.iter().take(MAX_DRAFT_SLOTS) {
        if validate_slot_key(key).is_err() {
            continue;
        }
        if let Some(slot) = normalize_slot(slot) {
            payload.slots.insert(key.clone(), slot);
        }
    }
    Some(payload)
}

/// Per-slot last-writer-wins union of two payloads.
pub fn merge_payloads(
    base: &SynaraDraftsPayload,
    incoming: &SynaraDraftsPayload,
) -> SynaraDraftsPayload {
    let mut merged = base.clone();
    merged.v = DRAFTS_PAYLOAD_VERSION;
    for (key, slot) in &incoming.slots {
        match merged.slots.get(key) {
            Some(existing) if !slot.is_newer_than(existing) => {}
            _ => {
                merged.slots.insert(key.clone(), slot.clone());
            }
        }
    }
    merged
}

/// Lamport-style stamp: never behind a write already seen for the slot, so a
/// device with a slow clock still wins with its newer edit.
pub fn next_updated_at(now_ms: u64, highest_seen: Option<u64>) -> u64 {
    highest_seen.map_or(now_ms, |seen| now_ms.max(seen.saturating_add(1)))
}

/// Drop tombstones nobody needs any more.
pub fn prune_tombstones(payload: &mut SynaraDraftsPayload, now_ms: u64) {
    payload.slots.retain(|_, slot| {
        !slot.cleared || now_ms.saturating_sub(slot.updated_at) < DRAFT_TOMBSTONE_RETENTION_MS
    });
}

fn payload_len(payload: &SynaraDraftsPayload) -> usize {
    serde_json::to_vec(payload).map_or(usize::MAX, |bytes| bytes.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapOutcome {
    Fits,
    DroppedRichText,
    Truncated,
}

/// Fit `payload` under [`MAX_DRAFT_PAYLOAD_BYTES`] by shrinking only the slot
/// being written: Slate first, then old tombstones, then the markdown tail.
/// Other devices' slots are never cut to make room for this one; if they alone
/// overflow (a peer bug), the oldest of them are dropped last.
pub fn cap_payload(payload: &mut SynaraDraftsPayload, slot_key: &str, now_ms: u64) -> CapOutcome {
    if payload_len(payload) <= MAX_DRAFT_PAYLOAD_BYTES {
        return CapOutcome::Fits;
    }
    if let Some(slot) = payload.slots.get_mut(slot_key) {
        if slot.desktop_slate.take().is_some() && payload_len(payload) <= MAX_DRAFT_PAYLOAD_BYTES {
            return CapOutcome::DroppedRichText;
        }
    }
    prune_tombstones(payload, now_ms);
    if payload_len(payload) <= MAX_DRAFT_PAYLOAD_BYTES {
        return CapOutcome::DroppedRichText;
    }

    if let Some(slot) = payload.slots.get_mut(slot_key) {
        // Mark first, so the flag's own bytes count against the budget.
        slot.truncated = slot.markdown.is_some();
    }
    while payload_len(payload) > MAX_DRAFT_PAYLOAD_BYTES {
        let excess = payload_len(payload) - MAX_DRAFT_PAYLOAD_BYTES;
        let shrunk = payload.slots.get_mut(slot_key).is_some_and(|slot| {
            match slot.markdown.take().filter(|markdown| !markdown.is_empty()) {
                Some(markdown) => {
                    slot.markdown = Some(truncate_json_len(&markdown, excess));
                    true
                }
                None => false,
            }
        });
        if shrunk {
            continue;
        }
        // Our slot is empty and the peers' slots alone overflow.
        let oldest = payload
            .slots
            .iter()
            .filter(|(key, _)| key.as_str() != slot_key)
            .min_by_key(|(_, slot)| slot.updated_at)
            .map(|(key, _)| key.clone());
        match oldest {
            Some(key) => {
                payload.slots.remove(&key);
            }
            None => break,
        }
    }
    CapOutcome::Truncated
}

/// Remove at least `excess` JSON-encoded bytes from the end, on a char
/// boundary.
fn truncate_json_len(text: &str, excess: usize) -> String {
    let mut removed = 0usize;
    let mut cut = text.len();
    for (index, ch) in text.char_indices().rev() {
        if removed >= excess {
            break;
        }
        removed += json_char_len(ch);
        cut = index;
    }
    text[..cut].to_owned()
}

fn json_char_len(ch: char) -> usize {
    match ch {
        '"' | '\\' | '\n' | '\r' | '\t' | '\u{08}' | '\u{0c}' => 2,
        c if (c as u32) < 0x20 => 6,
        c => c.len_utf8(),
    }
}

/// What a shell should do with a newer remote slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftDecision {
    /// Nothing to do: not newer, our own echo, or identical text.
    Keep,
    /// The composer is idle: replace it silently (or clear it).
    Apply,
    /// The user is typing: keep local text and offer the remote draft.
    Prompt,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocalDraftState {
    /// When the local draft last changed, in the same ms clock as slots.
    pub updated_at: Option<u64>,
    /// Typed in since the last save or apply.
    pub dirty: bool,
    pub markdown: Option<String>,
}

pub fn decide_remote(
    local: &LocalDraftState,
    remote: &SynaraDraftSlot,
    own_device_id: &str,
) -> DraftDecision {
    // Our own write echoed back by sync: local is at least as new.
    if remote.device_id == own_device_id {
        return DraftDecision::Keep;
    }
    if local
        .updated_at
        .is_some_and(|local| local >= remote.updated_at)
    {
        return DraftDecision::Keep;
    }
    let local_text = local.markdown.as_deref().unwrap_or("").trim();
    if remote.cleared {
        // Sent elsewhere. If the user is writing something new here, or the
        // local text's age is unknown, keep it.
        return if local.dirty || local_text.is_empty() || local.updated_at.is_none() {
            DraftDecision::Keep
        } else {
            DraftDecision::Apply
        };
    }
    if remote.markdown.as_deref().unwrap_or("").trim() == local_text && !remote.is_empty_content() {
        return DraftDecision::Keep;
    }
    // Typing, or text of unknown age (a shell that keeps no stamp): it may be
    // newer than the remote draft, so never replace it silently.
    if !local_text.is_empty() && (local.dirty || local.updated_at.is_none()) {
        DraftDecision::Prompt
    } else {
        DraftDecision::Apply
    }
}
