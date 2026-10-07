//! Session-scoped opaque media handles for native timeline rows.
//!
//! The SDK `MediaSource` (including encrypted-file descriptors) never crosses
//! the presenter boundary. This registry is intentionally independent of the
//! eventual URI protocol resolver, so source retention and revocation can be
//! proven before any media bytes are served.
//!
//! A handle is an HMAC-SHA256 of the session generation and the complete media
//! source under a random per-process secret. The same media in a reopened room
//! or another stream therefore keeps one URL (so the webview and the native
//! media cache reuse it), while handles stay unguessable, change with every
//! session generation, and still resolve only while a live row registers them.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use hmac::{Hmac, KeyInit, Mac};
use matrix_sdk::ruma::events::room::MediaSource;
use sha2::Sha256;

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
    /// Items sharing one handle (the same media posted twice in one stream).
    item_counts: HashMap<String, usize>,
}

impl TimelineMediaRegistry {
    pub fn new(session_generation: u64, stream_id: impl Into<String>) -> Self {
        Self {
            session_generation,
            stream_id: stream_id.into(),
            sources: HashMap::new(),
            handles_by_item: HashMap::new(),
            item_counts: HashMap::new(),
        }
    }

    fn release_handle(&mut self, handle_id: &str) -> bool {
        let Some(count) = self.item_counts.get_mut(handle_id) else {
            return false;
        };
        *count = count.saturating_sub(1);
        if *count > 0 {
            return false;
        }
        self.item_counts.remove(handle_id);
        self.sources.remove(handle_id).is_some()
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
        let handle_id = derived_handle(self.session_generation, &source)?;
        match self.handles_by_item.get(item_id).cloned() {
            // Re-projection of the same media keeps the capability.
            Some(previous) if previous == handle_id => {}
            // An edit replaced the media: move this item to the new handle.
            Some(previous) => {
                if !self.sources.contains_key(&handle_id)
                    && self.sources.len() >= MAX_TIMELINE_MEDIA_HANDLES
                {
                    return None;
                }
                self.release_handle(&previous);
                *self.item_counts.entry(handle_id.clone()).or_default() += 1;
                self.handles_by_item
                    .insert(item_id.to_owned(), handle_id.clone());
            }
            None => {
                if !self.sources.contains_key(&handle_id)
                    && self.sources.len() >= MAX_TIMELINE_MEDIA_HANDLES
                {
                    return None;
                }
                *self.item_counts.entry(handle_id.clone()).or_default() += 1;
                self.handles_by_item
                    .insert(item_id.to_owned(), handle_id.clone());
            }
        }
        self.sources.insert(
            handle_id.clone(),
            TimelineMediaSource {
                source,
                item_id: item_id.to_owned(),
                declared_mime_type: mime_type.clone(),
            },
        );
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
        usize::from(self.release_handle(&handle))
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
        self.item_counts.clear();
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

/// Random per-process key. `None` when the OS RNG is unavailable, in which
/// case no handle is minted (the row renders without media).
fn handle_secret() -> Option<&'static [u8; 32]> {
    static SECRET: OnceLock<Option<[u8; 32]>> = OnceLock::new();
    SECRET
        .get_or_init(|| {
            let mut secret = [0_u8; 32];
            getrandom::fill(&mut secret).ok()?;
            Some(secret)
        })
        .as_ref()
}

fn derived_handle(session_generation: u64, source: &MediaSource) -> Option<String> {
    derive_handle_with_secret(handle_secret()?, session_generation, source)
}

fn derive_handle_with_secret(
    secret: &[u8; 32],
    session_generation: u64,
    source: &MediaSource,
) -> Option<String> {
    let fingerprint = serde_json::to_vec(source).ok()?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).ok()?;
    mac.update(b"synara-timeline-media-v1\0");
    mac.update(&session_generation.to_be_bytes());
    mac.update(&fingerprint);
    let digest = mac.finalize().into_bytes();
    let mut handle = String::with_capacity(TIMELINE_MEDIA_HANDLE_PREFIX.len() + digest.len() * 2);
    handle.push_str(TIMELINE_MEDIA_HANDLE_PREFIX);
    for byte in digest {
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

    fn plain(uri: &str) -> MediaSource {
        MediaSource::Plain(uri.into())
    }

    fn register(registry: &mut TimelineMediaRegistry, item: &str, uri: &str) -> String {
        registry
            .register(item, plain(uri), Some("image/png".into()), None, None, None)
            .unwrap()
            .handle_id
    }

    #[test]
    fn reprojection_is_stable_and_retention_is_stream_bound() {
        let mut registry = TimelineMediaRegistry::new(7, "focused:!room:example.org:$event");
        let first = register(&mut registry, "item-1", "mxc://example.org/one");
        assert_eq!(
            register(&mut registry, "item-1", "mxc://example.org/one"),
            first
        );
        assert_eq!(registry.session_generation(), 7);
        assert_eq!(registry.stream_id(), "focused:!room:example.org:$event");
        registry.retain_items(["another-item"]);
        assert!(registry.resolve(&first).is_none());
    }

    #[test]
    fn edited_media_moves_the_item_to_a_new_handle() {
        let mut registry = TimelineMediaRegistry::new(7, "live:!room:example.org");
        let original = register(&mut registry, "item-1", "mxc://example.org/one");
        let edited = register(&mut registry, "item-1", "mxc://example.org/two");
        assert_ne!(original, edited);
        assert!(registry.resolve(&original).is_none());
        assert!(registry.resolve(&edited).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn reopened_streams_reuse_the_handle_within_one_session_generation() {
        let mut first_open = TimelineMediaRegistry::new(7, "live:!room:example.org:1");
        let mut reopened = TimelineMediaRegistry::new(7, "live:!room:example.org:2");
        // Item ids are per SDK timeline instance and differ across opens.
        let first = register(&mut first_open, "item-a", "mxc://example.org/photo");
        let again = register(&mut reopened, "item-z", "mxc://example.org/photo");
        assert_eq!(first, again);

        let mut next_session = TimelineMediaRegistry::new(8, "live:!room:example.org:3");
        let rotated = register(&mut next_session, "item-a", "mxc://example.org/photo");
        assert_ne!(first, rotated, "a new session generation rotates handles");
        assert!(is_timeline_media_handle(&rotated));
    }

    #[test]
    fn handles_bind_the_complete_source_and_the_process_secret() {
        let secret_a = [1_u8; 32];
        let secret_b = [2_u8; 32];
        let source = plain("mxc://example.org/photo");
        let a = derive_handle_with_secret(&secret_a, 7, &source).unwrap();
        assert_eq!(a, derive_handle_with_secret(&secret_a, 7, &source).unwrap());
        assert_ne!(a, derive_handle_with_secret(&secret_b, 7, &source).unwrap());
        assert_ne!(
            a,
            derive_handle_with_secret(&secret_a, 7, &plain("mxc://example.org/other")).unwrap()
        );
        assert!(!a.contains("example.org"));
    }

    #[test]
    fn a_shared_handle_survives_until_its_last_item_is_revoked() {
        let mut registry = TimelineMediaRegistry::new(7, "live:!room:example.org");
        let first = register(&mut registry, "item-1", "mxc://example.org/same");
        let second = register(&mut registry, "item-2", "mxc://example.org/same");
        assert_eq!(first, second);
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.revoke_item("item-1"), 0);
        assert!(registry.resolve(&first).is_some());
        assert_eq!(registry.revoke_item("item-2"), 1);
        assert!(registry.resolve(&first).is_none());
    }
}
