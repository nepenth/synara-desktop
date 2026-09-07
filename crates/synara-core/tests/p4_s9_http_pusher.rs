//! Typed SharedCore consume of live HTTP pusher set/delete.
//!
//! Calls Core methods that take push keys as arguments, never `Core::command`
//! JSON. Failed errors stay static and must not echo push keys, gateway URLs,
//! or tokens. Leftover `pusher_set` / `pusher_delete` stay on SharedCore.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

struct MemoryCallbackVault(Arc<Mutex<HashMap<String, Vec<u8>>>>);

impl IosSecretVault for MemoryCallbackVault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.0.lock().expect("vault").get(&key).cloned())
    }

    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").insert(key, value);
        Ok(())
    }

    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.0.lock().expect("vault").remove(&key);
        Ok(())
    }
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
}

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-p4-s9-http-pusher-it-{tag}-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

async fn accept_http_request(listener: &TcpListener, pushers_response: Option<&str>) -> String {
    let (mut socket, _) = listener.accept().await.expect("accept pusher write");
    let mut request = Vec::new();
    loop {
        let mut chunk = [0_u8; 2_048];
        let count = socket.read(&mut chunk).await.expect("read pusher request");
        assert!(count > 0, "pusher request closed before headers");
        request.extend_from_slice(&chunk[..count]);
        if let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&request[..header_end + 4]);
            let content_length = headers.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            });
            let expected = header_end + 4 + content_length.unwrap_or_default();
            if request.len() >= expected {
                break;
            }
        }
    }
    let is_versions = request.starts_with(b"GET /_matrix/client/versions ");
    let is_get_pushers = request.starts_with(b"GET /_matrix/client/v3/pushers ");
    let is_get_pushrules = request.starts_with(b"GET /_matrix/client/v3/pushrules/ ");
    let is_pusher_write = request.starts_with(b"POST /_matrix/client/v3/pushers/set ");
    let (status, response_body) = if is_versions {
        ("200 OK", r#"{"versions":["v1.11"]}"#)
    } else if is_get_pushrules {
        // This fixture starts with the product edit policy already installed;
        // dedicated policy tests cover installation, ordering and failures.
        (
            "200 OK",
            r#"{"global":{"override":[{"rule_id":"com.whylandcreative.synara.suppress_edits","default":false,"enabled":true,"conditions":[{"kind":"event_property_is","key":"content.m\\.relates_to.rel_type","value":"m.replace"}],"actions":[]}]}}"#,
        )
    } else if is_get_pushers {
        ("200 OK", pushers_response.unwrap_or(r#"{"pushers":[]}"#))
    } else if is_pusher_write {
        ("200 OK", "{}")
    } else {
        (
            "404 Not Found",
            r#"{"errcode":"M_NOT_FOUND","error":"not found"}"#,
        )
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
        response_body.len()
    );
    socket
        .write_all(response.as_bytes())
        .await
        .expect("write pusher response");
    socket.shutdown().await.expect("close pusher response");
    String::from_utf8(request).expect("pusher request is UTF-8 HTTP")
}

async fn accept_account_setup_and_pusher_write(listener: TcpListener) -> String {
    for _ in 0..64 {
        let request = accept_http_request(&listener, None).await;
        if request.starts_with("POST /_matrix/client/v3/pushers/set ") {
            return request;
        }
    }
    panic!("pusher write did not follow bounded account setup requests")
}

async fn accept_account_setup_and_device_cleanup(
    listener: TcpListener,
    pushers_response: String,
) -> (String, String) {
    let mut enumeration = None;
    for _ in 0..64 {
        let request = accept_http_request(&listener, Some(&pushers_response)).await;
        if request.starts_with("GET /_matrix/client/v3/pushers ") {
            enumeration = Some(request);
            continue;
        }
        if request.starts_with("POST /_matrix/client/v3/pushers/set ") {
            if let Some(enumeration) = enumeration.take() {
                return (enumeration, request);
            }
        }
    }
    panic!("device pusher cleanup did not follow bounded account setup requests")
}

async fn accept_account_setup_and_empty_device_cleanups(
    listener: TcpListener,
    expected_enumerations: usize,
) -> Vec<String> {
    let mut enumerations = Vec::new();
    for _ in 0..64 {
        let request = accept_http_request(&listener, Some(r#"{"pushers":[]}"#)).await;
        assert!(
            !request.starts_with("POST /_matrix/client/v3/pushers/set "),
            "empty device cleanup must not issue a pusher delete"
        );
        if request.starts_with("GET /_matrix/client/v3/pushers ") {
            enumerations.push(request);
            if enumerations.len() == expected_enumerations {
                return enumerations;
            }
        }
    }
    panic!("repeated empty device cleanup did not complete within bounded requests")
}

#[test]
fn http_pusher_surface_exposes_product_and_keeps_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("register_http_pusher"));
    assert!(udl.contains("delete_http_pusher"));
    assert!(udl.contains("dictionary PusherWriteDto"));
    assert!(udl.contains("interface PusherCommandError"));
    assert!(udl.contains("interface HttpPusherOwner"));
    assert!(udl.contains("delete_http_pushers_for_device"));
    let http_pusher_owner = udl
        .split("interface HttpPusherOwner {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("HttpPusherOwner");
    assert!(!http_pusher_owner.contains("device_display_name"));
    let image_pack_owner = include_str!("../src/app/account_data/image_packs_live.rs");
    assert!(!image_pack_owner.contains("register_http_pusher"));
    assert!(!image_pack_owner.contains("delete_http_pusher"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("register_http_pusher("));
    assert!(shared_core.contains("bind_http_pusher_owner("));
    assert!(shared_core.contains("delete_http_pusher("));
    assert!(shared_core.contains("pusher_set("));
    assert!(shared_core.contains("pusher_delete("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_login_password"));
}

#[test]
fn account_bound_pusher_owner_survives_core_account_rotation_with_original_credentials() {
    test_runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let old_listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind old account server");
            let new_listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind new account server");
            let old_homeserver = format!(
                "http://{}",
                old_listener.local_addr().expect("old server address")
            );
            let new_homeserver = format!(
                "http://{}",
                new_listener.local_addr().expect("new server address")
            );
            let old_pushers = serde_json::json!({
                "pushers": [
                    {
                        "pushkey": "old-push-key",
                        "app_id": "com.whylandcreative.synara",
                        "kind": "http",
                        "app_display_name": "Synara",
                        "device_display_name": "OLDDEVICE",
                        "lang": "en-US",
                        "data": {
                            "url": "https://push.example.org/_matrix/push/v1/notify",
                            "format": "event_id_only"
                        }
                    },
                    {
                        "pushkey": "other-device-key",
                        "app_id": "com.whylandcreative.synara",
                        "kind": "http",
                        "app_display_name": "Synara",
                        "device_display_name": "OTHERDEVICE",
                        "lang": "en-US",
                        "data": {
                            "url": "https://push.example.org/_matrix/push/v1/notify",
                            "format": "event_id_only"
                        }
                    }
                ]
            })
            .to_string();
            let old_server = tokio::spawn(accept_account_setup_and_device_cleanup(
                old_listener,
                old_pushers,
            ));
            let new_server = tokio::spawn(accept_account_setup_and_pusher_write(new_listener));

            let vault = Arc::new(Mutex::new(HashMap::new()));
            let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
                Arc::clone(&vault),
            )));
            let old_root = temp_root("bound-old");
            let new_root = temp_root("bound-new");
            shared
                .persist_planted_session_for_test(
                    "@old:example.org".to_owned(),
                    old_homeserver.clone(),
                    old_root.to_string_lossy().into_owned(),
                    "OLDDEVICE".to_owned(),
                    "old-account-access".to_owned(),
                    None,
                )
                .await
                .expect("plant old account");
            shared
                .attach_http_pusher_owner_for_test()
                .expect("attach old account pusher owner");
            let old_owner = shared
                .bind_http_pusher_owner(
                    "@old:example.org".to_owned(),
                    "OLDDEVICE".to_owned(),
                    old_homeserver.clone(),
                )
                .expect("bind old account owner");

            let mismatch = match shared.bind_http_pusher_owner(
                "@new:example.org".to_owned(),
                "NEWDEVICE".to_owned(),
                new_homeserver.clone(),
            ) {
                Ok(_) => panic!("wrong shell session must not bind current Core owner"),
                Err(error) => error,
            };
            let mismatch_text = format!("{mismatch:?}{mismatch}");
            assert!(mismatch_text.contains("v-pusher.session-mismatch"));
            assert!(!mismatch_text.contains("@old:example.org"));
            assert!(!mismatch_text.contains("@new:example.org"));
            assert!(!mismatch_text.contains("old-account-access"));

            shared.logout().await.expect("drop old current session");
            shared
                .persist_planted_session_for_test(
                    "@new:example.org".to_owned(),
                    new_homeserver.clone(),
                    new_root.to_string_lossy().into_owned(),
                    "NEWDEVICE".to_owned(),
                    "new-account-access".to_owned(),
                    None,
                )
                .await
                .expect("plant new account");
            shared
                .attach_http_pusher_owner_for_test()
                .expect("attach new account pusher owner");
            let new_owner = shared
                .bind_http_pusher_owner(
                    "@new:example.org".to_owned(),
                    "NEWDEVICE".to_owned(),
                    new_homeserver.clone(),
                )
                .expect("bind new account owner");

            old_owner
                .delete_http_pushers_for_device("com.whylandcreative.synara".to_owned(), None)
                .await
                .expect("enumerate and delete with retained old account owner");
            new_owner
                .delete_http_pusher(
                    "new-push-key".to_owned(),
                    "com.whylandcreative.synara".to_owned(),
                )
                .await
                .expect("delete with current new account owner");

            let (old_enumeration, old_delete) = old_server.await.expect("old pusher server");
            let new_request = new_server.await.expect("new pusher server");
            assert!(old_enumeration.contains("authorization: Bearer old-account-access"));
            assert!(old_delete.contains("authorization: Bearer old-account-access"));
            assert!(old_delete.contains(r#""pushkey":"old-push-key""#));
            assert!(!old_delete.contains("other-device-key"));
            assert!(!old_delete.contains("new-account-access"));
            assert!(new_request.contains("authorization: Bearer new-account-access"));
            assert!(!new_request.contains("old-account-access"));

            drop(old_owner);
            drop(new_owner);
            drop(shared);
            let _ = fs::remove_dir_all(old_root);
            let _ = fs::remove_dir_all(new_root);
        })
        .await
        .expect("account-bound pusher rotation proof timed out");
    });
}

#[test]
fn account_bound_device_cleanup_is_idempotent_when_no_pusher_matches() {
    test_runtime().block_on(async {
        tokio::time::timeout(Duration::from_secs(30), async {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind account server");
            let homeserver = format!(
                "http://{}",
                listener.local_addr().expect("account server address")
            );
            let server = tokio::spawn(accept_account_setup_and_empty_device_cleanups(listener, 2));
            let vault = Arc::new(Mutex::new(HashMap::new()));
            let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
                Arc::clone(&vault),
            )));
            let root = temp_root("empty-cleanup");
            shared
                .persist_planted_session_for_test(
                    "@empty:example.org".to_owned(),
                    homeserver.clone(),
                    root.to_string_lossy().into_owned(),
                    "EMPTYDEVICE".to_owned(),
                    "empty-account-access".to_owned(),
                    None,
                )
                .await
                .expect("plant account");
            shared
                .attach_http_pusher_owner_for_test()
                .expect("attach account pusher owner");
            let owner = shared
                .bind_http_pusher_owner(
                    "@empty:example.org".to_owned(),
                    "EMPTYDEVICE".to_owned(),
                    homeserver,
                )
                .expect("bind account owner");

            for _ in 0..2 {
                owner
                    .delete_http_pushers_for_device("com.whylandcreative.synara".to_owned(), None)
                    .await
                    .expect("empty cleanup remains idempotent");
            }

            let enumerations = server.await.expect("account pusher server");
            assert_eq!(enumerations.len(), 2);
            assert!(enumerations
                .iter()
                .all(|request| request.contains("authorization: Bearer empty-account-access")));

            drop(owner);
            drop(shared);
            let _ = fs::remove_dir_all(root);
        })
        .await
        .expect("idempotent pusher cleanup proof timed out");
    });
}

#[test]
fn http_pusher_without_session_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let rt = test_runtime();
    let push_key = "s9-it-push-key";
    let gateway = "https://push.example.org/_matrix/push/v1/notify";
    let app_id = "com.whylandcreative.synara";

    let register = rt
        .block_on(shared.register_http_pusher(
            push_key.to_owned(),
            app_id.to_owned(),
            gateway.to_owned(),
            "Synara".to_owned(),
            "en-US".to_owned(),
        ))
        .expect_err("no HTTP pusher owner");
    let delete = rt
        .block_on(shared.delete_http_pusher(push_key.to_owned(), app_id.to_owned()))
        .expect_err("no HTTP pusher owner");
    let bind = match shared.bind_http_pusher_owner(
        "@alice:push.example.org".to_owned(),
        "DEVICE".to_owned(),
        "https://push.example.org".to_owned(),
    ) {
        Ok(_) => panic!("no account-bound HTTP pusher owner"),
        Err(error) => error,
    };

    let register_text = format!("{register:?}{register}");
    let delete_text = format!("{delete:?}{delete}");
    let bind_text = format!("{bind:?}{bind}");
    assert!(register_text.contains("p2-register-http-pusher-no-session"));
    assert!(delete_text.contains("p2-delete-http-pusher-no-session"));
    assert!(bind_text.contains("p2-bind-http-pusher-no-session"));
    assert!(!register_text.contains("p4-s10-leftover-unavailable"));
    assert!(!delete_text.contains("p4-s10-leftover-unavailable"));

    let text = format!("{register_text}{delete_text}{bind_text}");
    assert!(!text.contains(push_key));
    assert!(!text.contains(gateway));
    assert!(!text.contains("push.example.org"));
    assert!(!text.contains(app_id));
}

#[test]
fn http_pusher_oversize_push_key_fails_closed_without_echo() {
    let shared = SharedCore::new();
    let marker = "s9ItOversizePushKey";
    let push_key = format!(
        "{marker}{}",
        "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
    );
    let error = test_runtime()
        .block_on(shared.register_http_pusher(
            push_key.clone(),
            "com.whylandcreative.synara".to_owned(),
            "https://push.example.org".to_owned(),
            "Synara".to_owned(),
            "en-US".to_owned(),
        ))
        .expect_err("oversize push key must fail closed");
    let text = format!("{error:?}{error}");
    assert!(text.contains("p4-s9-http-pusher-failed"));
    assert!(!text.contains(&push_key));
    assert!(!text.contains(marker));
    assert!(!text.contains("push.example.org"));
}

#[test]
fn http_pusher_planted_session_returns_owner_or_sdk_diagnostic_without_echo() {
    let access = "syt_s9_http_pusher_access";
    let refresh = "syr_s9_http_pusher_refresh";
    let push_key = "s9-it-planted-push-key";
    let gateway = "https://push.example.org/_matrix/push/v1/notify";
    let identity = alice();
    let map = Arc::new(Mutex::new(HashMap::new()));
    let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(Arc::clone(&map))));
    let root = temp_root("http-pusher-no-start");
    let rt = test_runtime();
    let _enter = rt.enter();
    rt.block_on(shared.persist_planted_session_for_test(
        identity.user_id().to_owned(),
        identity.homeserver_url().to_owned(),
        root.to_string_lossy().into_owned(),
        "DEVICEABC".to_owned(),
        access.to_owned(),
        Some(refresh.to_owned()),
    ))
    .expect("planted persist");
    rt.block_on(shared.attach_session_owners())
        .expect("owners attached");

    let register = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            shared.register_http_pusher(
                push_key.to_owned(),
                "com.whylandcreative.synara".to_owned(),
                gateway.to_owned(),
                "Synara".to_owned(),
                "en-US".to_owned(),
            ),
        )
        .await
        .expect("register timed out")
    });
    let delete = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            shared.delete_http_pusher(push_key.to_owned(), "com.whylandcreative.synara".to_owned()),
        )
        .await
        .expect("delete timed out")
    });
    drop(shared);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);

    let register_text = match &register {
        Ok(value) => format!("ok:{}", value.status),
        Err(error) => format!("{error:?}{error}"),
    };
    let delete_text = match &delete {
        Ok(value) => format!("ok:{}", value.status),
        Err(error) => format!("{error:?}{error}"),
    };

    assert!(
        register.is_ok()
            || register_text.contains("v-push.")
            || register_text.contains("v-pusher."),
        "register must return a handler or SDK diagnostic: {register_text}"
    );
    assert!(
        delete.is_ok() || delete_text.contains("v-push.") || delete_text.contains("v-pusher."),
        "delete must return a handler or SDK diagnostic: {delete_text}"
    );
    assert!(
        !register_text.contains("p4-s10-leftover-unavailable"),
        "register must not use leftover-unavailable: {register_text}"
    );
    assert!(
        !delete_text.contains("p4-s10-leftover-unavailable"),
        "delete must not use leftover-unavailable: {delete_text}"
    );
    assert!(
        !register_text.contains("p4-s9-http-pusher-failed"),
        "register must not hide a wrong envelope: {register_text}"
    );
    assert!(
        !delete_text.contains("p4-s9-http-pusher-failed"),
        "delete must not hide a wrong envelope: {delete_text}"
    );

    let text = format!("{register_text}{delete_text}");
    assert!(!text.contains(push_key));
    assert!(!text.contains(gateway));
    assert!(!text.contains("push.example.org"));
    assert!(!text.contains(access));
    assert!(!text.contains(refresh));
    assert!(!text.contains("syt_"));
}
