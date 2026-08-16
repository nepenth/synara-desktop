//! Session-scoped opaque media handles for native timeline rows.
//!
//! The SDK `MediaSource` (including encrypted-file descriptors) never crosses
//! the presenter boundary. This registry is intentionally independent of the
//! eventual URI protocol resolver, so source retention and revocation can be
//! proven before any media bytes are served.

use std::collections::{HashMap, HashSet};

use matrix_sdk::ruma::events::room::MediaSource;

use super::TimelineMediaHandle;

const MAX_TIMELINE_MEDIA_HANDLES: usize = 4_096;
pub const TIMELINE_MEDIA_HANDLE_PREFIX: &str = "timeline-media-";

/// Native-only source retained behind one opaque product handle.
#[derive(Clone)]
pub struct TimelineMediaSource {
    pub source: MediaSource,
    pub item_id: String,
    pub declared_mime_type: Option<String>,
}

/// Session-generation-scoped mapping from timeline item to opaque handles.
///
/// SNC-P1-5c: this type is now public API at the synara-core crate root via
/// the timeline re-exports, which makes clippy's `len_without_is_empty` fire;
/// no `is_empty` consumer ever existed (same allowance as room_list
/// `InviteAvatarHandles`).
#[allow(clippy::len_without_is_empty)]
pub struct TimelineMediaRegistry {
    session_generation: u64,
    stream_id: String,
    sources: HashMap<String, TimelineMediaSource>,
    handles_by_item: HashMap<String, String>,
}

impl TimelineMediaRegistry {
    pub fn new(session_generation: u64, stream_id: impl Into<String>) -> Self {
        Self {
            session_generation,
            stream_id: stream_id.into(),
            sources: HashMap::new(),
            handles_by_item: HashMap::new(),
        }
    }

    /// Register a source held exclusively by Rust and return only its safe
    /// presenter metadata. Re-projection of the same SDK item keeps the
    /// capability stable while atomically replacing its native-only source.
    pub fn register(
        &mut self,
        item_id: &str,
        source: MediaSource,
        mime_type: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
    ) -> Option<TimelineMediaHandle> {
        if item_id.is_empty() {
            return None;
        }
        if let Some(handle_id) = self.handles_by_item.get(item_id).cloned() {
            self.sources.insert(
                handle_id.clone(),
                TimelineMediaSource {
                    source,
                    item_id: item_id.to_owned(),
                    declared_mime_type: mime_type.clone(),
                },
            );
            return Some(TimelineMediaHandle {
                handle_id,
                mime_type,
                width,
                height,
                duration_ms,
            });
        }
        if self.sources.len() >= MAX_TIMELINE_MEDIA_HANDLES {
            return None;
        }
        let handle_id = (0..4).find_map(|_| {
            let candidate = random_handle()?;
            (!self.sources.contains_key(&candidate)).then_some(candidate)
        })?;
        self.sources.insert(
            handle_id.clone(),
            TimelineMediaSource {
                source,
                item_id: item_id.to_owned(),
                declared_mime_type: mime_type.clone(),
            },
        );
        self.handles_by_item
            .insert(item_id.to_owned(), handle_id.clone());
        Some(TimelineMediaHandle {
            handle_id,
            mime_type,
            width,
            height,
            duration_ms,
        })
    }

    /// Resolve one handle for a native media protocol. The returned type is
    /// deliberately not serializable and has no public DTO conversion.
    pub fn resolve(&self, handle_id: &str) -> Option<&TimelineMediaSource> {
        is_timeline_media_handle(handle_id)
            .then(|| self.sources.get(handle_id))
            .flatten()
    }

    /// Revoke every handle whose event row disappeared from the timeline.
    pub fn revoke_item(&mut self, item_id: &str) -> usize {
        let Some(handle) = self.handles_by_item.remove(item_id) else {
            return 0;
        };
        usize::from(self.sources.remove(&handle).is_some())
    }

    /// Keep only capabilities backed by rows still present in this exact
    /// opened stream.
    pub fn retain_items<'a>(&mut self, item_ids: impl IntoIterator<Item = &'a str>) {
        let retained: HashSet<&str> = item_ids.into_iter().collect();
        let removed: Vec<String> = self
            .handles_by_item
            .keys()
            .filter(|item_id| !retained.contains(item_id.as_str()))
            .cloned()
            .collect();
        for item_id in removed {
            self.revoke_item(&item_id);
        }
    }

    pub fn clear(&mut self) {
        self.sources.clear();
        self.handles_by_item.clear();
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}

impl Drop for TimelineMediaRegistry {
    fn drop(&mut self) {
        self.clear();
    }
}

pub fn is_timeline_media_handle(handle_id: &str) -> bool {
    let Some(suffix) = handle_id.strip_prefix(TIMELINE_MEDIA_HANDLE_PREFIX) else {
        return false;
    };
    suffix.len() == 64 && suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn random_handle() -> Option<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    let mut handle = String::with_capacity(TIMELINE_MEDIA_HANDLE_PREFIX.len() + bytes.len() * 2);
    handle.push_str(TIMELINE_MEDIA_HANDLE_PREFIX);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut handle, "{byte:02x}").ok()?;
    }
    Some(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_opaque_and_revoked_with_their_timeline_item() {
        let mut registry = TimelineMediaRegistry::new(7, "live:!room:example.org");
        let handle = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/image".into()),
                Some("image/png".into()),
                Some(32),
                Some(16),
                None,
            )
            .unwrap();
        let json = serde_json::to_string(&handle).unwrap();
        assert!(!json.contains("mxc://"));
        assert!(is_timeline_media_handle(&handle.handle_id));
        assert_eq!(
            handle.handle_id.len(),
            TIMELINE_MEDIA_HANDLE_PREFIX.len() + 64
        );
        assert_eq!(registry.len(), 1);
        assert!(registry.resolve(&handle.handle_id).is_some());
        assert_eq!(registry.revoke_item("item-1"), 1);
        assert!(registry.resolve(&handle.handle_id).is_none());
    }

    #[test]
    fn reprojection_is_stable_and_retention_is_stream_bound() {
        let mut registry = TimelineMediaRegistry::new(7, "focused:!room:example.org:$event");
        let first = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/one".into()),
                Some("image/png".into()),
                None,
                None,
                None,
            )
            .unwrap();
        let updated = registry
            .register(
                "item-1",
                MediaSource::Plain("mxc://example.org/two".into()),
                Some("image/jpeg".into()),
                None,
                None,
                None,
            )
            .unwrap();
        assert_eq!(first.handle_id, updated.handle_id);
        assert_eq!(registry.session_generation(), 7);
        assert_eq!(registry.stream_id(), "focused:!room:example.org:$event");
        registry.retain_items(["another-item"]);
        assert!(registry.resolve(&first.handle_id).is_none());
    }
}
