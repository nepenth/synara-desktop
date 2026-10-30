//! The fetcher itself is tested in `synara_core::app::link_preview` against
//! local mock servers; these cover what the desktop command adds.

use super::*;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn the_command_never_connects_to_a_local_server() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"<head><title>Local</title></head>".to_vec(), "text/html"),
        )
        .expect(0)
        .mount(&server)
        .await;
    // A literal loopback address and a name that resolves to it.
    assert_eq!(
        desktop_link_preview(format!("{}/", server.uri())).await,
        None
    );
    let port = server.address().port();
    assert_eq!(
        desktop_link_preview(format!("http://localhost:{port}/")).await,
        None
    );
    assert_eq!(
        desktop_link_preview("file:///etc/passwd".into()).await,
        None
    );
}

#[test]
fn stored_images_need_a_valid_handle() {
    assert!(stored_image("../etc").is_none());
    assert!(stored_image(&"0".repeat(64)).is_none());
    assert!(stored_image("").is_none());
}

#[test]
fn the_preview_serializes_in_camel_case_without_empty_fields() {
    let preview = LinkPreview {
        url: "https://example.org/".into(),
        title: "T".into(),
        description: None,
        site_name: Some("S".into()),
        image_handle_id: Some("ab".into()),
    };
    assert_eq!(
        serde_json::to_value(preview).unwrap(),
        serde_json::json!({
            "url": "https://example.org/",
            "title": "T",
            "siteName": "S",
            "imageHandleId": "ab",
        })
    );
}

#[test]
fn the_user_agent_names_the_desktop_version() {
    assert_eq!(
        link_preview::user_agent(env!("CARGO_PKG_VERSION")),
        format!("Synara/{} (link preview)", env!("CARGO_PKG_VERSION"))
    );
}
