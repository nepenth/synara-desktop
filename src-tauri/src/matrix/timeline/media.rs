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

/// Native-only source retained behind one opaque product handle.
#[derive(Clone)]
pub struct TimelineMediaSource {
    pub source: MediaSource,
    pub item_id: String,
}

/// Session-generation-scoped mapping from timeline item to opaque handles.
pub struct TimelineMediaRegistry {
    session_generation: u64,
    next_handle: u64,
    sources: HashMap<String, TimelineMediaSource>,
    handles_by_item: HashMap<String, HashSet<String>>,
}

impl TimelineMediaRegistry {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            next_handle: 0,
            sources: HashMap::new(),
            handles_by_item: HashMap::new(),
        }
    }

    /// Register a source held exclusively by Rust and return only its safe
    /// presenter metadata. Repeated registrations for one item intentionally
    /// receive distinct handles: thumbnail and original sources may differ.
    pub fn register(
        &mut self,
        item_id: &str,
        source: MediaSource,
        mime_type: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
    ) -> Option<TimelineMediaHandle> {
        if item_id.is_empty() || self.sources.len() >= MAX_TIMELINE_MEDIA_HANDLES {
            return None;
        }
        self.next_handle = self.next_handle.saturating_add(1);
        let handle_id = format!(
            "timeline-media:{}:{}",
            self.session_generation, self.next_handle
        );
        self.sources.insert(
            handle_id.clone(),
            TimelineMediaSource {
                source,
                item_id: item_id.to_owned(),
            },
        );
        self.handles_by_item
            .entry(item_id.to_owned())
            .or_default()
            .insert(handle_id.clone());
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
        self.sources.get(handle_id)
    }

    /// Revoke every handle whose event row disappeared from the timeline.
    pub fn revoke_item(&mut self, item_id: &str) -> usize {
        let Some(handles) = self.handles_by_item.remove(item_id) else {
            return 0;
        };
        let mut removed = 0;
        for handle in handles {
            removed += usize::from(self.sources.remove(&handle).is_some());
        }
        removed
    }

    pub fn clear(&mut self) {
        self.sources.clear();
        self.handles_by_item.clear();
    }

    pub fn len(&self) -> usize {
        self.sources.len()
    }
}

impl Drop for TimelineMediaRegistry {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_opaque_and_revoked_with_their_timeline_item() {
        let mut registry = TimelineMediaRegistry::new(7);
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
        assert_eq!(registry.len(), 1);
        assert!(registry.resolve(&handle.handle_id).is_some());
        assert_eq!(registry.revoke_item("item-1"), 1);
        assert!(registry.resolve(&handle.handle_id).is_none());
    }
}
