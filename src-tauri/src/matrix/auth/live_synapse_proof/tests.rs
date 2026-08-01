//! Authenticated V-SEND.R-CALL-MEDIA proof against disposable Synapse.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use matrix_sdk::media::{MediaFormat, MediaRequestParameters};
use matrix_sdk::ruma::events::room::MediaSource;
use sha1::Sha1;

use crate::matrix::auth::{login_with_password, LoginOptions};
use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::matrix::store::{AccountIdentity, StoreKeyMaterial};

type HmacSha1 = Hmac<Sha1>;

const MEDIA_FIXTURE: &[u8] = b"synara-call-widget-media-proof-v1\0original-file";

fn live_enabled() -> bool {
    std::env::var("SYNARA_RUN_MATRIX_RUST_CALL_MEDIA_LIVE")
        .ok()
        .as_deref()
        == Some("1")
}

fn loopback_homeserver_url() -> Option<String> {
    let raw = std::env::var("SYNARA_MATRIX_HOMESERVER_URL").ok()?;
    let url = url::Url::parse(&raw).ok()?;
    if url.scheme() != "http"
        || !matches!(
            url.host_str(),
            Some("127.0.0.1") | Some("localhost") | Some("::1")
        )
        || !url.username().is_empty()
        || url.password().is_some()
        || !(url.path().is_empty() || url.path() == "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

fn registration_secret_from_harness() -> String {
    let config = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("integration/synapse/runtime/homeserver.yaml");
    let text = std::fs::read_to_string(&config)
        .unwrap_or_else(|err| panic!("read disposable Synapse config: {err}"));
    let mut secret = None;
    for line in text.lines() {
        let Some(value) = line.strip_prefix("registration_shared_secret:") else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        assert!(
            value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()),
            "disposable Synapse registration secret must be one generated 64-hex value"
        );
        assert!(secret.is_none(), "duplicate registration_shared_secret");
        secret = Some(value.to_owned());
    }
    secret.expect("disposable Synapse registration secret")
}

fn registration_mac(secret: &str, nonce: &str, localpart: &str, password: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).expect("hmac key");
    mac.update(nonce.as_bytes());
    mac.update(&[0]);
    mac.update(localpart.as_bytes());
    mac.update(&[0]);
    mac.update(password.as_bytes());
    mac.update(&[0]);
    mac.update(b"notadmin");
    hex::encode(mac.finalize().into_bytes())
}

async fn register_disposable_account(
    base_url: &str,
    localpart: &str,
    password: &str,
) -> serde_json::Value {
    let secret = registration_secret_from_harness();
    let endpoint = format!("{base_url}/_synapse/admin/v1/register");
    let http = reqwest::Client::builder().build().expect("http client");
    let nonce_response = http
        .get(&endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .expect("registration nonce request");
    assert!(
        nonce_response.status().is_success(),
        "registration nonce HTTP {}",
        nonce_response.status()
    );
    let nonce_text = nonce_response.text().await.expect("nonce body");
    let nonce_json: serde_json::Value = serde_json::from_str(&nonce_text).expect("nonce json");
    let nonce = nonce_json["nonce"]
        .as_str()
        .expect("registration nonce")
        .to_owned();
    let mac = registration_mac(&secret, &nonce, localpart, password);
    let body = serde_json::json!({
        "nonce": nonce,
        "username": localpart,
        "password": password,
        "admin": false,
        "mac": mac,
    })
    .to_string();
    let register_response = http
        .post(&endpoint)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .await
        .expect("shared-secret registration");
    assert!(
        register_response.status().is_success(),
        "registration HTTP {}",
        register_response.status()
    );
    let register_text = register_response.text().await.expect("registration body");
    serde_json::from_str(&register_text).expect("registration json")
}

fn temp_store_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-v-send-call-media-{nanos}"));
    std::fs::create_dir_all(&root).expect("temp store root");
    root
}

#[tokio::test]
async fn live_native_call_widget_media_paths_against_disposable_synapse_when_configured() {
    if !live_enabled() {
        return;
    }
    let base = loopback_homeserver_url()
        .expect("SYNARA_MATRIX_HOMESERVER_URL must be credential-free HTTP loopback");

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let localpart = format!("vcall_{stamp:x}");
    let password = format!("S-{stamp:x}");
    let registration = register_disposable_account(&base, &localpart, &password).await;
    let user_id = registration["user_id"]
        .as_str()
        .expect("registration user_id")
        .to_owned();

    let store_root = temp_store_root();
    let identity = AccountIdentity::new(&user_id, &base).expect("account identity");
    let key = StoreKeyMaterial::generate().expect("store key");
    let config = ClientBuildConfig::product_default(&store_root, identity, Some(key))
        .expect("client config");
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");
    login_with_password(
        &client,
        &localpart,
        &password,
        &LoginOptions {
            device_display_name: Some("V-SEND.R CallWidget media proof".into()),
            request_refresh_token: false,
        },
    )
    .await
    .expect("password login");

    // This is the authenticated `matrix_call_media_config` SDK operation.
    let upload_size = client
        .load_or_fetch_max_upload_size()
        .await
        .expect("authenticated CallWidget media config");
    assert!(
        i64::from(upload_size) > 0,
        "Synapse must return a positive m.upload.size"
    );

    let fixture = MEDIA_FIXTURE.to_vec();
    let upload = client
        .media()
        .upload(&mime::APPLICATION_OCTET_STREAM, fixture.clone(), None)
        .await
        .expect("authenticated CallWidget media fixture upload");
    let content_uri = upload.content_uri;
    assert!(content_uri.as_str().starts_with("mxc://"));

    // This is the authenticated `matrix_media_download` SDK operation. The
    // File format proves original bytes rather than a thumbnail response.
    let request = MediaRequestParameters {
        source: MediaSource::Plain(content_uri),
        format: MediaFormat::File,
    };
    let downloaded = client
        .media()
        .get_media_content(&request, true)
        .await
        .expect("authenticated original-file media download");
    assert_eq!(downloaded, fixture);
}
