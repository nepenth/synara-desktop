//! Typed SharedCore operation for on-device link previews.
//!
//! iOS must not fetch previews with LinkPresentation: `LPMetadataProvider`
//! resolves names itself, follows redirects and loads subresources, so a
//! message link could make every reader's phone send requests into their
//! local network. This uses the fetcher desktop uses
//! (`crate::app::link_preview`), whose every hop is checked.

use std::sync::OnceLock;

use super::*;
use crate::app::link_preview::{self, LinkPreviewer};

/// A link's preview for the card under a message. `image` holds the page's
/// image as fetched (PNG, JPEG, GIF, WebP or AVIF, at most 5 MB); the shell
/// decodes and downsizes it. Callers must not log the payload.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LinkPreviewDto {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub site_name: Option<String>,
    pub image: Option<Vec<u8>>,
}

/// The process's previewer, built on first use with the app's version in its
/// User-Agent.
pub(super) struct LinkPreviews {
    user_agent: String,
    previewer: OnceLock<Option<LinkPreviewer>>,
}

impl LinkPreviews {
    pub(super) fn new(app_version: &str) -> Self {
        Self {
            user_agent: link_preview::user_agent(app_version),
            previewer: OnceLock::new(),
        }
    }

    fn previewer(&self) -> Option<&LinkPreviewer> {
        self.previewer
            .get_or_init(|| LinkPreviewer::new(self.user_agent.clone()))
            .as_ref()
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// The preview of `url`, fetched from this device, or `None` when the URL
    /// is refused (not public http(s), or a redirect leaves the public web),
    /// the page does not answer with HTML in time, or it has no title. The
    /// shell asks only when its Link previews setting is on. Needs no session.
    pub async fn link_preview(&self, url: String) -> Option<LinkPreviewDto> {
        let previewer = self.link_previews.previewer()?;
        let preview = previewer.preview(&url).await?;
        let image = preview
            .image_handle_id
            .as_deref()
            .and_then(|handle| previewer.image(handle))
            .map(|(bytes, _)| bytes.as_ref().clone());
        Some(LinkPreviewDto {
            url: preview.url,
            title: preview.title,
            description: preview.description,
            site_name: preview.site_name,
            image,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_ios_preview_refuses_local_and_non_web_links() {
        let core = SharedCore::new();
        for url in [
            "http://127.0.0.1/",
            "http://localhost/",
            "http://[::1]/",
            "http://192.168.1.1/",
            "http://169.254.169.254/latest/meta-data/",
            "file:///etc/passwd",
            "https://user:pass@example.org/",
            "not a url",
        ] {
            assert_eq!(core.link_preview(url.into()).await, None, "{url}");
        }
    }

    #[test]
    fn the_ios_user_agent_matches_desktop() {
        assert_eq!(
            LinkPreviews::new("2.7.1").user_agent,
            "Synara/2.7.1 (link preview)"
        );
        assert_eq!(
            LinkPreviews::new("").user_agent,
            "Synara/unknown (link preview)"
        );
    }
}
