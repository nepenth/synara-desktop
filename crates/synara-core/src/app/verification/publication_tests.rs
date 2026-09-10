use super::publication::ensure_own_device_keys_published;
use matrix_sdk::{test_utils::mocks::MatrixMockServer, Client};
use matrix_sdk_crypto::types::DeviceKeys;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use wiremock::{
    matchers::{method, path_regex},
    Mock, ResponseTemplate,
};

async fn fixture() -> (MatrixMockServer, Client, Value) {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let device = client.encryption().get_own_device().await.unwrap().unwrap();
    let keys = serde_json::to_value(device.as_device_keys()).unwrap();
    (server, client, keys)
}

fn query_response(keys: &Value) -> Value {
    json!({"device_keys": {
        keys["user_id"].as_str().unwrap(): {
            keys["device_id"].as_str().unwrap(): keys
        }
    }})
}

async fn query_mock(server: &MatrixMockServer, status: u16, body: Value) {
    Mock::given(method("POST"))
        .and(path_regex(r"^/_matrix/client/.*/keys/query$"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server.server())
        .await;
}

#[tokio::test]
async fn missing_keys_upload_only_the_original_signed_public_device_and_read_back() {
    let (server, client, original) = fixture().await;
    let published = Arc::new(Mutex::new(None::<Value>));
    let queried = published.clone();
    Mock::given(method("POST"))
        .and(path_regex(r"^/_matrix/client/.*/keys/query$"))
        .respond_with(move |_: &wiremock::Request| {
            let body = queried
                .lock()
                .unwrap()
                .as_ref()
                .map(query_response)
                .unwrap_or_else(|| json!({"device_keys": {}}));
            ResponseTemplate::new(200).set_body_json(body)
        })
        .mount(server.server())
        .await;
    let uploaded = published.clone();
    Mock::given(method("POST"))
        .and(path_regex(r"^/_matrix/client/.*/keys/upload$"))
        .respond_with(move |request: &wiremock::Request| {
            let body: Value = request.body_json().unwrap();
            assert_eq!(
                body.as_object().unwrap().len(),
                1,
                "no OTKs, fallback or secret fields"
            );
            let keys: DeviceKeys = serde_json::from_value(body["device_keys"].clone()).unwrap();
            keys.check_self_signature()
                .expect("SDK self-signature must survive upload");
            *uploaded.lock().unwrap() = Some(body["device_keys"].clone());
            ResponseTemplate::new(200).set_body_json(json!({"one_time_key_counts": {}}))
        })
        .expect(1)
        .mount(server.server())
        .await;

    ensure_own_device_keys_published(&client).await.unwrap();
    assert_eq!(*published.lock().unwrap(), Some(original.clone()));
    // A second action sees existing keys and does not upload again.
    ensure_own_device_keys_published(&client).await.unwrap();
    let own = client.encryption().get_own_device().await.unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(own.as_device_keys()).unwrap(),
        original
    );
    let requests = server.received_requests().await.unwrap();
    // Session restoration may concurrently read secret-storage account data.
    // Only mutating requests belong to publication/verification ordering.
    let requests: Vec<_> = requests
        .iter()
        .filter(|r| r.method.as_str() != "GET")
        .collect();
    let paths: Vec<_> = requests
        .iter()
        .map(|r| r.url.path().rsplit('/').next().unwrap())
        .collect();
    assert_eq!(paths, ["query", "upload", "query", "query"]);
}

#[tokio::test]
async fn failed_incomplete_malformed_or_conflicting_query_prevents_upload_and_verification() {
    let (server, client, original) = fixture().await;
    let different_store = server.client_builder().build().await;
    let different_device = different_store
        .encryption()
        .get_own_device()
        .await
        .unwrap()
        .unwrap();
    let different_keys = serde_json::to_value(different_device.as_device_keys()).unwrap();
    assert_eq!(different_keys["device_id"], original["device_id"]);
    assert_ne!(different_keys["keys"], original["keys"]);
    let mut conflicting = original.clone();
    conflicting["device_id"] = json!("DIFFERENT");
    let mut invalid_signature = original.clone();
    invalid_signature["signatures"] = json!({});
    let cases = [
        (200, query_response(&different_keys), "conflict"),
        (
            500,
            json!({"errcode":"M_UNKNOWN","error":"unavailable"}),
            "query-failed",
        ),
        (
            200,
            json!({"device_keys":{},"failures":{"example.org":{}}}),
            "query-incomplete",
        ),
        (
            200,
            json!({"device_keys":{original["user_id"].as_str().unwrap():{original["device_id"].as_str().unwrap():{}}}}),
            "invalid",
        ),
        (
            200,
            json!({"device_keys":{original["user_id"].as_str().unwrap():{original["device_id"].as_str().unwrap():conflicting}}}),
            "conflict",
        ),
        (200, query_response(&invalid_signature), "invalid"),
    ];
    for (status, body, reason) in cases {
        server.reset().await;
        query_mock(&server, status, body).await;
        let owner = super::NativeVerificationOwner::new(&client, 1);
        let error = owner.start(None).await.unwrap_err();
        assert_eq!(error, format!("v-crypto.1-own-device-publication-{reason}"));
        let requests = server.received_requests().await.unwrap();
        // Session restoration may concurrently read secret-storage account data.
        // Only mutating requests belong to publication/verification ordering.
        let requests: Vec<_> = requests
            .iter()
            .filter(|r| r.method.as_str() != "GET")
            .collect();
        assert_eq!(
            requests.len(),
            1,
            "must stop before uploading or sending verification"
        );
        assert!(requests[0].url.path().ends_with("/keys/query"));
    }
}

#[tokio::test]
async fn upload_failure_or_missing_readback_prevents_verification() {
    let (server, client, _) = fixture().await;
    for (status, reason) in [(500, "failed"), (200, "missing")] {
        server.reset().await;
        query_mock(&server, 200, json!({"device_keys":{}})).await;
        Mock::given(method("POST"))
            .and(path_regex(r"^/_matrix/client/.*/keys/upload$"))
            .respond_with(
                ResponseTemplate::new(status).set_body_json(if status == 200 {
                    json!({"one_time_key_counts":{}})
                } else {
                    json!({"errcode":"M_UNKNOWN","error":"unavailable"})
                }),
            )
            .expect(1)
            .mount(server.server())
            .await;
        let owner = super::NativeVerificationOwner::new(&client, 1);
        let error = owner.start(None).await.unwrap_err();
        assert_eq!(error, format!("v-crypto.1-own-device-publication-{reason}"));
        let requests = server.received_requests().await.unwrap();
        // Session restoration may concurrently read secret-storage account data.
        // Only mutating requests belong to publication/verification ordering.
        let requests: Vec<_> = requests
            .iter()
            .filter(|r| r.method.as_str() != "GET")
            .collect();
        assert_eq!(requests.len(), if status == 200 { 3 } else { 2 });
        assert!(requests.iter().all(|r| r.url.path().contains("/keys/")));
    }
}
