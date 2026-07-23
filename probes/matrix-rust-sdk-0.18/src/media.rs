//! Media retrieval and upload compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::Client;
use matrix_sdk::media::{Media, MediaRequestParameters};

/// P0.3b-media-type — `matrix_sdk::Media` is a public type.
///
/// Source: `crates/matrix-sdk/src/media.rs` (`pub struct Media`) and crate-root
/// re-export.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_media_type() -> &'static str {
    std::any::type_name::<Media>()
}

/// P0.3b-client-media — `Client::media() -> Media`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn media`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_media() {
    fn _shape(client: &Client) -> Media {
        client.media()
    }
    let _ = _shape;
}

/// P0.3b-media-upload — `Media::upload`.
///
/// Source: `crates/matrix-sdk/src/media.rs` (`pub fn upload`).
///
/// Resolves the public method without naming `mime::Mime` (not a direct probe
/// dependency) and without naming the private return type. Taking the method as
/// a value forces the compiler to check its public signature shape.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_media_upload() {
    let _method = Media::upload;
    let _ = _method;
}

/// P0.3b-media-get-media-content — `Media::get_media_content`.
///
/// Source: `crates/matrix-sdk/src/media.rs` (`pub async fn get_media_content`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_media_get_media_content() {
    async fn _shape(
        media: &Media,
        request: &MediaRequestParameters,
        use_cache: bool,
    ) -> matrix_sdk::Result<Vec<u8>> {
        media.get_media_content(request, use_cache).await
    }
    let _ = _shape;
}

/// Run every media probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    let _ = probe_media_type();
    probe_client_media();
    probe_media_upload();
    probe_media_get_media_content();
}
