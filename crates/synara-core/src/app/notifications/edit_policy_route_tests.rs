//! Execute production pusher registration against a scripted HTTP homeserver.
//! Server readbacks are independent of the client's SDK/account-data cache.
use super::*;
use matrix_sdk::{
    authentication::matrix::MatrixSession,
    config::RequestConfig,
    ruma::{api::MatrixVersion, user_id},
    store::RoomLoadSettings,
    SessionMeta, SessionTokens,
};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Step {
    method: &'static str,
    path: String,
    status: u16,
    response: serde_json::Value,
}
fn step(method: &'static str, path: &str, response: serde_json::Value) -> Step {
    Step {
        method,
        path: path.to_owned(),
        status: 200,
        response,
    }
}
fn rules_response(installed: bool, enabled: bool) -> serde_json::Value {
    let mut rules = Ruleset::server_default(user_id!("@reader:example.org"));
    if installed {
        rules
            .insert(
                NewPushRule::Override(NewConditionalPushRule::new(
                    RULE_ID.to_owned(),
                    conditions(),
                    vec![],
                )),
                None,
                None,
            )
            .unwrap();
        rules
            .set_enabled(RuleKind::Override, RULE_ID, enabled)
            .unwrap();
    }
    serde_json::json!({"global": rules})
}
async fn client_and_script(
    steps: Vec<Step>,
) -> (Client, tokio::task::JoinHandle<Vec<serde_json::Value>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let mut bodies = vec![];
        let mut bootstrap_reads = 0;
        for step in steps {
            loop {
                let (mut socket, _) =
                    tokio::time::timeout(Duration::from_secs(10), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut request = vec![];
                let (header_end, content_len) = loop {
                    let mut bytes = [0u8; 4096];
                    let size = socket.read(&mut bytes).await.unwrap();
                    assert!(size > 0);
                    request.extend_from_slice(&bytes[..size]);
                    if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let len = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if request.len() >= end + 4 + len {
                            break (end + 4, len);
                        }
                    }
                };
                let headers = String::from_utf8_lossy(&request[..header_end]).to_lowercase();
                // The SDK's recovery initialization and backup-state observer can
                // each read the same absent key. Route only that exact ancillary
                // endpoint without consuming any expected policy operation.
                if headers.lines().next() == Some("get /_matrix/client/v3/user/@reader:example.org/account_data/m.secret_storage.default_key http/1.1") {
                assert!(headers.contains("authorization: bearer policy-proof-token"));
                assert_eq!(content_len, 0);
                bootstrap_reads += 1;
                assert!(bootstrap_reads <= 2, "unexpected repeated SDK recovery initialization");
                let body = r#"{"errcode":"M_NOT_FOUND","error":"No default key"}"#;
                let response = format!("HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
                continue;
            }
                assert_eq!(
                    headers.lines().next().unwrap(),
                    format!("{} {} http/1.1", step.method.to_lowercase(), step.path),
                    "loopback fixture received an unexpected Matrix owner route"
                );
                assert!(headers.contains("authorization: bearer policy-proof-token"));
                let body = if content_len == 0 {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice(&request[header_end..header_end + content_len]).unwrap()
                };
                bodies.push(body);
                let body = step.response.to_string();
                let response = format!("HTTP/1.1 {} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", step.status, body.len(), body);
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.shutdown().await.unwrap();
                break;
            }
        }
        bodies
    });
    let client = Client::builder()
        .homeserver_url(url)
        .server_versions([MatrixVersion::V1_11])
        .request_config(
            RequestConfig::new()
                .retry_limit(0)
                .timeout(Duration::from_secs(3)),
        )
        .build()
        .await
        .unwrap();
    client
        .matrix_auth()
        .restore_session(
            MatrixSession {
                meta: SessionMeta {
                    user_id: user_id!("@reader:example.org").to_owned(),
                    device_id: "POLICYDEVICE".into(),
                },
                tokens: SessionTokens {
                    access_token: "policy-proof-token".to_owned(),
                    refresh_token: None,
                },
            },
            RoomLoadSettings::default(),
        )
        .await
        .unwrap();
    // Use the SDK's actual initialization completion boundary. A delay or an
    // accept-any-request fallback would hide unrelated owner-route mistakes.
    tokio::time::timeout(
        Duration::from_secs(5),
        client.encryption().wait_for_e2ee_initialization_tasks(),
    )
    .await
    .expect("SDK E2EE initialization must finish before the policy test");
    (client, server)
}
async fn register(
    client: &Client,
) -> Result<super::super::http_pusher::MatrixHttpPusherWriteResult, &'static str> {
    super::super::http_pusher::register_http_pusher(
        client,
        "test-apns-token",
        "com.whylandcreative.synara",
        "https://push.example.org/notify",
        "Synara",
        "POLICYDEVICE",
        "en-US",
    )
    .await
}
const GET: &str = "/_matrix/client/v3/pushrules/";
const PUT: &str =
    "/_matrix/client/v3/pushrules/global/override/com.whylandcreative.synara.suppress_edits";
const PUSHER: &str = "/_matrix/client/v3/pushers/set";

#[tokio::test]
async fn registration_installs_confirms_then_registers_and_repeated_policy_is_read_only() {
    let (client, server) = client_and_script(vec![
        step("GET", GET, rules_response(false, true)),
        step("PUT", PUT, serde_json::json!({})),
        step("GET", GET, rules_response(true, true)),
        step("POST", PUSHER, serde_json::json!({})),
        step("GET", GET, rules_response(true, true)),
        step("POST", PUSHER, serde_json::json!({})),
    ])
    .await;
    register(&client).await.unwrap();
    register(&client).await.unwrap();
    let bodies = server.await.unwrap();
    assert_eq!(
        bodies[1],
        serde_json::json!({"conditions": conditions(), "actions": []})
    );
    assert_eq!(bodies[3]["data"]["format"], "event_id_only");
    assert_eq!(bodies[3], bodies[5]);
}

#[tokio::test]
async fn policy_write_failure_prevents_pusher_registration() {
    let mut failed = step(
        "PUT",
        PUT,
        serde_json::json!({"errcode":"M_FORBIDDEN","error":"denied"}),
    );
    failed.status = 403;
    let (client, server) =
        client_and_script(vec![step("GET", GET, rules_response(false, true)), failed]).await;
    assert_eq!(register(&client).await.unwrap_err(), POLICY_FAILED);
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn unconfirmed_readback_prevents_pusher_registration() {
    let (client, server) = client_and_script(vec![
        step("GET", GET, rules_response(false, true)),
        step("PUT", PUT, serde_json::json!({})),
        step("GET", GET, rules_response(false, true)),
    ])
    .await;
    assert_eq!(register(&client).await.unwrap_err(), POLICY_UNCONFIRMED);
    assert_eq!(server.await.unwrap().len(), 3);
}

#[tokio::test]
async fn disabled_policy_is_enabled_before_registration() {
    let (client, server) = client_and_script(vec![
        step("GET", GET, rules_response(true, false)),
        step("PUT", PUT, serde_json::json!({})),
        step("PUT", &format!("{PUT}/enabled"), serde_json::json!({})),
        step("GET", GET, rules_response(true, true)),
        step("POST", PUSHER, serde_json::json!({})),
    ])
    .await;
    register(&client).await.unwrap();
    assert_eq!(
        server.await.unwrap()[2],
        serde_json::json!({"enabled":true})
    );
}

#[tokio::test]
async fn shadowed_policy_repositions_only_owned_rule() {
    let mut shadowed = rules_response(true, true);
    shadowed["global"]["override"]
        .as_array_mut()
        .unwrap()
        .insert(
            1,
            serde_json::json!({
                "rule_id":"example.other-notify", "default":false, "enabled":true,
                "conditions":[], "actions":["notify"]
            }),
        );
    let mut confirmed = rules_response(true, true);
    confirmed["global"]["override"]
        .as_array_mut()
        .unwrap()
        .insert(
            2,
            serde_json::json!({
                "rule_id":"example.other-notify", "default":false, "enabled":true,
                "conditions":[], "actions":["notify"]
            }),
        );
    let (client, server) = client_and_script(vec![
        step("GET", GET, shadowed),
        step("DELETE", PUT, serde_json::json!({})),
        step("PUT", PUT, serde_json::json!({})),
        step("GET", GET, confirmed),
        step("POST", PUSHER, serde_json::json!({})),
    ])
    .await;
    register(&client).await.unwrap();
    let bodies = server.await.unwrap();
    assert_eq!(
        bodies[2],
        serde_json::json!({"conditions":conditions(),"actions":[]})
    );
}

#[tokio::test]
async fn unavailable_authoritative_rules_do_not_become_default_rules() {
    let mut failed = step(
        "GET",
        GET,
        serde_json::json!({"errcode":"M_FORBIDDEN","error":"denied"}),
    );
    failed.status = 403;
    let (client, server) = client_and_script(vec![failed]).await;
    assert_eq!(register(&client).await.unwrap_err(), POLICY_FAILED);
    assert_eq!(server.await.unwrap().len(), 1);
}
