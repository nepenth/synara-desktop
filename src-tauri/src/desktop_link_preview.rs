//! Link previews fetched by each reader's device: when a message shows a link,
//! the desktop app of everyone who reads it requests the linked page itself
//! and reads its title, description, site name and image from the `<head>`.
//! Nothing goes through the homeserver, so previews work in encrypted rooms
//! too, but the site sees every reader's IP address; the Link previews
//! setting says so and turns the fetch off.
//!
//! The fetch, its SSRF protections and the parsing live in
//! `synara_core::app::link_preview`, shared with iOS. This module is the
//! renderer's command and serves preview images to the webview through the
//! `synara-media:` protocol at `link-preview/<handle>`, like other downloaded
//! media, so the CSP `img-src` stays closed to the web.

use std::sync::{Arc, OnceLock};

use synara_core::app::link_preview::{self, LinkPreviewer};

pub use synara_core::app::link_preview::LinkPreview;

/// Path prefix of preview images on the `synara-media:` protocol.
pub const IMAGE_PATH_PREFIX: &str = "link-preview/";

fn previewer() -> Option<&'static LinkPreviewer> {
    static PREVIEWER: OnceLock<Option<LinkPreviewer>> = OnceLock::new();
    PREVIEWER
        .get_or_init(|| LinkPreviewer::new(link_preview::user_agent(env!("CARGO_PKG_VERSION"))))
        .as_ref()
}

/// The page's preview, or `None` when the URL is refused, the page does not
/// answer with HTML in time, or it has no title. `image_handle_id` names an
/// image served at `synara-media:` `link-preview/<id>`.
#[tauri::command]
pub async fn desktop_link_preview(url: String) -> Option<LinkPreview> {
    previewer()?.preview(&url).await
}

/// A stored preview image by handle, for the `synara-media:` protocol.
pub fn stored_image(handle: &str) -> Option<(Arc<Vec<u8>>, &'static str)> {
    previewer()?.image(handle)
}

#[cfg(test)]
mod tests;
