//! Homeserver URL preview (`Media::get_media_preview`) with a fail-closed
//! encrypted-room gate and a privacy-safe DTO.
//!
//! Encrypted and unknown encryption never call the homeserver: the URL would
//! otherwise be disclosed. `og:image` MXC becomes an invite-avatar handle;
//! HTTPS images are omitted (no webview fetch). OpenGraph JSON is cached in
//! memory per `(session_generation, url)` and never persisted.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedMxcUri, UInt};
use matrix_sdk::{Client, EncryptionState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex as AsyncMutex;
use url::Url;

use super::parse_plain_media_uri;
use crate::app::room_list::InviteAvatarHandles;

pub const MAX_MEDIA_PREVIEW_CACHE: usize = 64;
pub const MAX_MEDIA_PREVIEW_URL_BYTES: usize = 2048;
const MAX_TITLE_CHARS: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 400;
const MAX_SITE_NAME_CHARS: usize = 80;

const PREVIEW_URL_INVALID: &str = "v-send.r-media-preview-url-invalid";
const PREVIEW_UNAVAILABLE: &str = "v-send.r-media-preview-unavailable";
const PREVIEW_HANDLE: &str = "v-send.r-media-preview-handle-unavailable";

/// Privacy-safe URL preview DTO. Raw homeserver JSON and `og:image` MXC never
/// appear on this wire shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMediaPreviewSnapshot {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_handle_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaPreviewGate {
    Fetch,
    SkipEncrypted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedOpenGraphPreview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image_mxc: Option<OwnedMxcUri>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CachedPreview {
    Ok(MappedOpenGraphPreview),
    Unavailable,
}

pub struct MediaPreviewCache {
    session_generation: u64,
    entries: HashMap<String, CachedPreview>,
    order: VecDeque<String>,
}

impl MediaPreviewCache {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn get(&self, session_generation: u64, url: &str) -> Option<CachedPreview> {
        (self.session_generation == session_generation)
            .then(|| self.entries.get(url).cloned())
            .flatten()
    }

    fn insert(&mut self, session_generation: u64, url: &str, value: CachedPreview) {
        if self.session_generation != session_generation {
            self.session_generation = session_generation;
            self.entries.clear();
            self.order.clear();
        }
        if self.entries.contains_key(url) {
            self.entries.insert(url.to_owned(), value);
            return;
        }
        while self.entries.len() >= MAX_MEDIA_PREVIEW_CACHE {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            self.entries.remove(&oldest);
        }
        self.order.push_back(url.to_owned());
        self.entries.insert(url.to_owned(), value);
    }
}

/// Encrypted and unknown rooms must not disclose the URL to the homeserver.
pub fn media_preview_gate(
    encryption: EncryptionState,
    url: &str,
) -> Result<MediaPreviewGate, &'static str> {
    if !preview_url_is_allowed(url) {
        return Err(PREVIEW_URL_INVALID);
    }
    if encryption.is_encrypted() || encryption.is_unknown() {
        return Ok(MediaPreviewGate::SkipEncrypted);
    }
    Ok(MediaPreviewGate::Fetch)
}

pub fn preview_url_is_allowed(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_MEDIA_PREVIEW_URL_BYTES || value != value.trim() {
        return false;
    }
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match url.scheme() {
        "http" | "https" => url
            .host_str()
            .map(is_safe_public_preview_host)
            .unwrap_or(false),
        _ => false,
    }
}

fn is_safe_public_preview_host(host: &str) -> bool {
    let host = normalize_url_host(host);
    if host.is_empty() || is_local_hostname(&host) {
        return false;
    }
    if host.contains(':') {
        return !is_private_ipv6(&host);
    }
    if let Some(octets) = parse_ipv4(&host) {
        return !is_private_ipv4_octets(octets);
    }
    true
}

fn normalize_url_host(host: &str) -> String {
    host.trim_matches(|character| character == '[' || character == ']')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn is_local_hostname(host: &str) -> bool {
    if host == "localhost" || host == "0.0.0.0" {
        return true;
    }
    const LOCAL_SUFFIXES: &[&str] = &[
        ".localhost",
        ".local",
        ".localdomain",
        ".internal",
        ".lan",
        ".home.arpa",
    ];
    LOCAL_SUFFIXES.iter().any(|suffix| host.ends_with(suffix))
}

fn parse_ipv4(host: &str) -> Option<[u8; 4]> {
    let mut octets = [0_u8; 4];
    let mut parts = host.split('.');
    for octet in &mut octets {
        *octet = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(octets)
}

fn is_private_ipv4_octets(octets: [u8; 4]) -> bool {
    matches!(
        octets,
        [0, ..]
            | [10, ..]
            | [127, ..]
            | [169, 254, ..]
            | [192, 168, ..]
            | [172, 16..=31, ..]
            | [100, 64..=127, ..]
    )
}

fn is_private_ipv6(host: &str) -> bool {
    host == "::1"
        || host == "::"
        || host.starts_with("fc")
        || host.starts_with("fd")
        || host.starts_with("fe80")
        || host.starts_with("::ffff:")
}

pub fn map_open_graph_preview(raw: &str) -> Result<MappedOpenGraphPreview, &'static str> {
    let value: Value = serde_json::from_str(raw).map_err(|_| PREVIEW_UNAVAILABLE)?;
    if !value.is_object() {
        return Err(PREVIEW_UNAVAILABLE);
    }
    if value.get("imageMxc").is_some() || value.get("image_mxc").is_some() {
        // Hostile or confused payloads must not teach the mapper to forward MXC.
        return Err(PREVIEW_UNAVAILABLE);
    }
    let mapped = MappedOpenGraphPreview {
        title: og_string(&value, "og:title", MAX_TITLE_CHARS)
            .or_else(|| og_string(&value, "title", MAX_TITLE_CHARS)),
        description: og_string(&value, "og:description", MAX_DESCRIPTION_CHARS)
            .or_else(|| og_string(&value, "description", MAX_DESCRIPTION_CHARS)),
        site_name: og_string(&value, "og:site_name", MAX_SITE_NAME_CHARS)
            .or_else(|| og_string(&value, "site_name", MAX_SITE_NAME_CHARS)),
        image_mxc: og_image_mxc(&value),
    };
    Ok(mapped)
}

fn og_string(value: &Value, key: &str, max_chars: usize) -> Option<String> {
    let text = value.get(key)?.as_str()?.trim();
    if text.is_empty() {
        return None;
    }
    Some(truncate_chars(text, max_chars))
}

fn og_image_mxc(value: &Value) -> Option<OwnedMxcUri> {
    let image = value.get("og:image")?.as_str()?.trim();
    if image.is_empty() || !image.starts_with("mxc://") {
        return None;
    }
    parse_plain_media_uri(image).ok()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        truncated.push('…');
    }
    truncated
}

fn mapped_is_empty(mapped: &MappedOpenGraphPreview) -> bool {
    mapped.title.is_none()
        && mapped.description.is_none()
        && mapped.site_name.is_none()
        && mapped.image_mxc.is_none()
}

pub fn skipped_media_preview(
    room_id: String,
    session_generation: u64,
    url: &str,
) -> MatrixMediaPreviewSnapshot {
    MatrixMediaPreviewSnapshot {
        status: "skipped".into(),
        room_id,
        session_generation,
        url: url.to_owned(),
        title: None,
        description: None,
        site_name: None,
        thumbnail_handle_id: None,
    }
}

pub fn unavailable_media_preview(
    room_id: String,
    session_generation: u64,
    url: &str,
) -> MatrixMediaPreviewSnapshot {
    MatrixMediaPreviewSnapshot {
        status: "unavailable".into(),
        room_id,
        session_generation,
        url: url.to_owned(),
        title: None,
        description: None,
        site_name: None,
        thumbnail_handle_id: None,
    }
}

fn snapshot_from_mapped(
    room_id: String,
    session_generation: u64,
    url: &str,
    mapped: &MappedOpenGraphPreview,
    thumbnail_handle_id: Option<String>,
) -> MatrixMediaPreviewSnapshot {
    MatrixMediaPreviewSnapshot {
        status: "ok".into(),
        room_id,
        session_generation,
        url: url.to_owned(),
        title: mapped.title.clone(),
        description: mapped.description.clone(),
        site_name: mapped.site_name.clone(),
        thumbnail_handle_id,
    }
}

async fn issue_thumbnail_handle(
    invite_avatars: &Arc<AsyncMutex<InviteAvatarHandles>>,
    room_id: &str,
    image_mxc: Option<OwnedMxcUri>,
) -> Result<Option<String>, &'static str> {
    let Some(mxc) = image_mxc else {
        return Ok(None);
    };
    let mut handles = invite_avatars.lock().await;
    match handles.issue(room_id, mxc) {
        Ok(handle) => Ok(Some(handle)),
        Err(_) => Err(PREVIEW_HANDLE),
    }
}

/// Fetch a preview after the encryption/URL gate has already allowed it.
///
/// Callers must not invoke this for encrypted or unknown rooms. The public
/// [`room_media_preview`] entry point is the only production path.
async fn fetch_unencrypted_media_preview(
    client: &Client,
    invite_avatars: &Arc<AsyncMutex<InviteAvatarHandles>>,
    cache: &Arc<AsyncMutex<MediaPreviewCache>>,
    room_id: &str,
    session_generation: u64,
    url: &str,
    ts: Option<u64>,
) -> Result<MatrixMediaPreviewSnapshot, &'static str> {
    let cached = {
        let guard = cache.lock().await;
        guard.get(session_generation, url)
    };
    if let Some(CachedPreview::Unavailable) = cached {
        return Ok(unavailable_media_preview(
            room_id.to_owned(),
            session_generation,
            url,
        ));
    }
    if let Some(CachedPreview::Ok(mapped)) = cached {
        let thumbnail_handle_id =
            issue_thumbnail_handle(invite_avatars, room_id, mapped.image_mxc.clone())
                .await
                .unwrap_or_default();
        return Ok(snapshot_from_mapped(
            room_id.to_owned(),
            session_generation,
            url,
            &mapped,
            thumbnail_handle_id,
        ));
    }

    let ts = ts.and_then(UInt::new).map(MilliSecondsSinceUnixEpoch);
    let raw = match client.media().get_media_preview(url, ts).await {
        Ok(Some(raw)) => raw,
        Ok(None) => {
            cache
                .lock()
                .await
                .insert(session_generation, url, CachedPreview::Unavailable);
            return Ok(unavailable_media_preview(
                room_id.to_owned(),
                session_generation,
                url,
            ));
        }
        Err(_) => return Err(PREVIEW_UNAVAILABLE),
    };

    let mapped = match map_open_graph_preview(raw.get()) {
        Ok(mapped) if !mapped_is_empty(&mapped) => mapped,
        _ => {
            cache
                .lock()
                .await
                .insert(session_generation, url, CachedPreview::Unavailable);
            return Ok(unavailable_media_preview(
                room_id.to_owned(),
                session_generation,
                url,
            ));
        }
    };
    {
        let mut guard = cache.lock().await;
        guard.insert(session_generation, url, CachedPreview::Ok(mapped.clone()));
    }
    let thumbnail_handle_id =
        issue_thumbnail_handle(invite_avatars, room_id, mapped.image_mxc.clone())
            .await
            .unwrap_or_default();
    Ok(snapshot_from_mapped(
        room_id.to_owned(),
        session_generation,
        url,
        &mapped,
        thumbnail_handle_id,
    ))
}

/// Sole production preview entry. Encrypted/unknown rooms return `skipped`
/// without touching `Media::get_media_preview`.
#[allow(clippy::too_many_arguments)] // Explicit session/room/URL/ts fields mirror the IPC contract.
pub async fn room_media_preview(
    client: &Client,
    encryption: EncryptionState,
    invite_avatars: Arc<AsyncMutex<InviteAvatarHandles>>,
    cache: Arc<AsyncMutex<MediaPreviewCache>>,
    room_id: &str,
    session_generation: u64,
    url: &str,
    ts: Option<u64>,
) -> Result<MatrixMediaPreviewSnapshot, &'static str> {
    match media_preview_gate(encryption, url)? {
        MediaPreviewGate::SkipEncrypted => Ok(skipped_media_preview(
            room_id.to_owned(),
            session_generation,
            url,
        )),
        MediaPreviewGate::Fetch => {
            fetch_unencrypted_media_preview(
                client,
                &invite_avatars,
                &cache,
                room_id,
                session_generation,
                url,
                ts,
            )
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preview_url_gate_matches_scope_cases() {
        assert!(preview_url_is_allowed("https://example.org/x"));
        assert!(!preview_url_is_allowed("javascript:alert(1)"));
        assert!(!preview_url_is_allowed("https://user:pass@example.org/"));
        assert!(!preview_url_is_allowed("mxc://example.org/aa"));
        assert!(!preview_url_is_allowed("https://[fd12:3456:789a::1]/x"));
        assert!(!preview_url_is_allowed("matrix:u/alice:example.org"));
        assert!(!preview_url_is_allowed("mailto:user@example.org"));
        assert!(!preview_url_is_allowed("http://192.168.1.10/admin"));
        assert!(!preview_url_is_allowed("https://localhost/x"));
        assert!(!preview_url_is_allowed("https://10.0.0.5/x"));
    }

    #[test]
    fn encrypted_and_unknown_rooms_never_plan_a_homeserver_call() {
        assert_eq!(
            media_preview_gate(EncryptionState::Encrypted, "https://example.org/x").unwrap(),
            MediaPreviewGate::SkipEncrypted
        );
        assert_eq!(
            media_preview_gate(EncryptionState::Unknown, "https://example.org/x").unwrap(),
            MediaPreviewGate::SkipEncrypted
        );
        assert_eq!(
            media_preview_gate(EncryptionState::NotEncrypted, "https://example.org/x").unwrap(),
            MediaPreviewGate::Fetch
        );
        assert_eq!(
            media_preview_gate(EncryptionState::Encrypted, "javascript:alert(1)").unwrap_err(),
            PREVIEW_URL_INVALID
        );
    }

    #[test]
    fn open_graph_mapper_keeps_mxc_off_the_wire_and_skips_https_images() {
        let mapped = map_open_graph_preview(
            r#"{
                "og:title": "Example",
                "og:description": "Hello",
                "og:site_name": "Example Site",
                "og:image": "mxc://example.org/thumb",
                "matrix:image:size": 12
            }"#,
        )
        .unwrap();
        assert_eq!(mapped.title.as_deref(), Some("Example"));
        assert_eq!(mapped.description.as_deref(), Some("Hello"));
        assert_eq!(mapped.site_name.as_deref(), Some("Example Site"));
        assert_eq!(
            mapped
                .image_mxc
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("mxc://example.org/thumb")
        );

        let https_image = map_open_graph_preview(
            r#"{"og:title":"Https image","og:image":"https://cdn.example.org/og.png"}"#,
        )
        .unwrap();
        assert_eq!(https_image.title.as_deref(), Some("Https image"));
        assert!(https_image.image_mxc.is_none());

        let snapshot = snapshot_from_mapped(
            "!room:example.org".into(),
            7,
            "https://example.org/x",
            &mapped,
            Some("ab".repeat(32)),
        );
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(
            value,
            json!({
                "status": "ok",
                "roomId": "!room:example.org",
                "sessionGeneration": 7,
                "url": "https://example.org/x",
                "title": "Example",
                "description": "Hello",
                "siteName": "Example Site",
                "thumbnailHandleId": "ab".repeat(32),
            })
        );
        let dumped = value.to_string();
        assert!(!dumped.contains("imageMxc"));
        assert!(!dumped.contains("og:image"));
        assert!(!dumped.contains("mxc://"));
        assert!(!dumped.contains("access_token"));
    }

    #[test]
    fn preview_cache_is_generation_scoped_and_bounded() {
        let mut cache = MediaPreviewCache::new(1);
        let sample = CachedPreview::Ok(MappedOpenGraphPreview {
            title: Some("one".into()),
            description: None,
            site_name: None,
            image_mxc: None,
        });
        for index in 0..=MAX_MEDIA_PREVIEW_CACHE {
            cache.insert(1, &format!("https://example.org/{index}"), sample.clone());
        }
        assert_eq!(cache.entries.len(), MAX_MEDIA_PREVIEW_CACHE);
        assert!(cache.get(1, "https://example.org/0").is_none());
        assert!(cache
            .get(1, &format!("https://example.org/{MAX_MEDIA_PREVIEW_CACHE}"))
            .is_some());
        assert!(cache.get(2, "https://example.org/1").is_none());
    }

    #[test]
    fn room_media_preview_skips_encrypted_before_homeserver_call() {
        let source = include_str!("preview.rs");
        let start = source
            .find("pub async fn room_media_preview")
            .expect("public preview entry");
        let body = &source[start..];
        let end = body
            .find("\nasync fn fetch_unencrypted_media_preview")
            .or_else(|| body.find("\n#[cfg(test)]"))
            .unwrap_or(body.len());
        let public_fn = &body[..end];
        let gate = public_fn
            .find("media_preview_gate")
            .expect("encryption/URL gate");
        let fetch = public_fn
            .find("fetch_unencrypted_media_preview")
            .expect("unencrypted fetch");
        assert!(
            gate < fetch,
            "encrypted skip must run before the homeserver fetch helper"
        );
        assert!(
            !public_fn.contains("get_media_preview"),
            "public entry must not call the SDK itself"
        );
        assert!(public_fn.contains("SkipEncrypted"));

        let helper = source
            .split("async fn fetch_unencrypted_media_preview")
            .nth(1)
            .expect("unencrypted helper")
            .split("pub async fn room_media_preview")
            .next()
            .expect("unencrypted helper body");
        assert!(helper.contains("get_media_preview"));
        assert!(!helper.contains("EncryptionState"));
        assert!(!helper.contains("media_preview_gate"));
    }
}
