//! Opaque, session-scoped capabilities for invite-card avatars.
//!
//! This module deliberately owns only the mapping from an invite snapshot to
//! its native media source. The webview receives the opaque handle, never an
//! MXC URI. A later Tauri URI protocol owner may resolve the handle and obtain
//! thumbnail bytes through the managed SDK client without widening this into a
//! generic media IPC API.

use std::collections::{HashMap, VecDeque};

use matrix_sdk::ruma::OwnedMxcUri;

/// Keep the invite-only capability surface bounded even if snapshots are
/// repeatedly requested before the UI consumes them.
pub const MAX_INVITE_AVATAR_HANDLES: usize = 256;

/// Native-only request selected by the invite projection. It must never cross
/// the Tauri command boundary because it carries the original media source.
#[derive(Clone)]
pub struct InviteAvatarSource {
    pub room_id: String,
    pub mxc_uri: OwnedMxcUri,
}

/// Session-generation-scoped opaque avatar capabilities.
///
/// Handles are random bearer capabilities, not encoded Matrix identifiers.
/// They are invalidated wholesale on session retirement and selectively when
/// an invitation is acted on.
pub struct InviteAvatarHandles {
    session_generation: u64,
    entries: HashMap<String, InviteAvatarSource>,
    order: VecDeque<String>,
}

// Preserved as-is from src-tauri (behavior-identical, SNC-P1-5b). `len()` only
// feeds bounded-capability tests; no `is_empty` consumer ever existed. The lint
// only fires because room_list is now public API at the synara-core crate root.
#[allow(clippy::len_without_is_empty)]
impl InviteAvatarHandles {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return a stable capability for the same current-session invite source,
    /// or mint a fresh random capability without exposing its MXC URI.
    pub fn issue(&mut self, room_id: &str, mxc_uri: OwnedMxcUri) -> Result<String, &'static str> {
        if let Some((handle, _)) = self
            .entries
            .iter()
            .find(|(_, source)| source.room_id == room_id && source.mxc_uri == mxc_uri)
        {
            return Ok(handle.clone());
        }

        while self.entries.len() >= MAX_INVITE_AVATAR_HANDLES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }

        for _ in 0..4 {
            let handle = random_handle()?;
            if self.entries.contains_key(&handle) {
                continue;
            }
            self.order.push_back(handle.clone());
            self.entries.insert(
                handle.clone(),
                InviteAvatarSource {
                    room_id: room_id.to_owned(),
                    mxc_uri,
                },
            );
            return Ok(handle);
        }

        Err("v-rooms.1-invite-avatar-handle-unavailable")
    }

    /// Resolve a capability only when the protocol owner is still serving the
    /// session generation that minted it.
    pub fn resolve(&self, session_generation: u64, handle: &str) -> Option<InviteAvatarSource> {
        (self.session_generation == session_generation)
            .then(|| self.entries.get(handle).cloned())
            .flatten()
    }

    /// Revoke every URL capability for an invite immediately after a native
    /// invite action. A fresh snapshot can mint a new capability if the invite
    /// remains visible.
    pub fn revoke_room(&mut self, room_id: &str) {
        self.entries.retain(|_, source| source.room_id != room_id);
        self.order
            .retain(|handle| self.entries.contains_key(handle));
    }

    /// Logout/account switch invalidates every prior-session capability.
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.entries.clear();
        self.order.clear();
    }
}

fn random_handle() -> Result<String, &'static str> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| "v-rooms.1-invite-avatar-handle-unavailable")?;
    let mut handle = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut handle, "{byte:02x}")
            .map_err(|_| "v-rooms.1-invite-avatar-handle-unavailable")?;
    }
    Ok(handle)
}

#[cfg(test)]
mod tests {
    use matrix_sdk::ruma::OwnedMxcUri;

    use super::*;

    fn mxc(value: &str) -> OwnedMxcUri {
        OwnedMxcUri::from(value)
    }

    #[test]
    fn handles_are_opaque_stable_and_generation_scoped() {
        let mut handles = InviteAvatarHandles::new(7);
        let first = handles
            .issue("!invite:example.org", mxc("mxc://example.org/one"))
            .unwrap();
        let second = handles
            .issue("!invite:example.org", mxc("mxc://example.org/one"))
            .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert!(!first.contains("example.org"));
        assert!(handles.resolve(7, &first).is_some());
        assert!(handles.resolve(8, &first).is_none());
    }

    #[test]
    fn invite_action_and_session_retirement_revoke_handles() {
        let mut handles = InviteAvatarHandles::new(7);
        let one = handles
            .issue("!one:example.org", mxc("mxc://example.org/one"))
            .unwrap();
        let two = handles
            .issue("!two:example.org", mxc("mxc://example.org/two"))
            .unwrap();

        handles.revoke_room("!one:example.org");
        assert!(handles.resolve(7, &one).is_none());
        assert!(handles.resolve(7, &two).is_some());

        handles.retire_generation(8);
        assert_eq!(handles.session_generation(), 8);
        assert_eq!(handles.len(), 0);
        assert!(handles.resolve(8, &two).is_none());
    }

    #[test]
    fn capability_store_stays_bounded() {
        let mut handles = InviteAvatarHandles::new(1);
        for index in 0..=MAX_INVITE_AVATAR_HANDLES {
            handles
                .issue(
                    &format!("!{index}:example.org"),
                    mxc(&format!("mxc://example.org/{index}")),
                )
                .unwrap();
        }
        assert_eq!(handles.len(), MAX_INVITE_AVATAR_HANDLES);
    }
}
