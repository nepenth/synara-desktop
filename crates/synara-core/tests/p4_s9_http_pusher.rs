//! Typed SharedCore consume of live HTTP pusher set/delete.
//!
//! Calls Core methods that take push keys as arguments, never `Core::command`
//! JSON. Failed errors stay static and must not echo push keys, gateway URLs,
//! or tokens. Leftover `pusher_set` / `pusher_delete` stay on SharedCore.

use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use synara_core::app::store::AccountIdentity;
use synara_core::transport::MAX_ENVELOPE_PAYLOAD_JSON_BYTES;
use synara_core::{IosSecretVault, IosSecretVaultError, SharedCore};

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

#[test]
fn http_pusher_surface_exposes_product_and_keeps_leftovers() {
    let udl = include_str!("../src/synara_core.udl");
    assert!(udl.contains("register_http_pusher"));
    assert!(udl.contains("delete_http_pusher"));
    assert!(udl.contains("dictionary PusherWriteDto"));
    assert!(udl.contains("interface PusherCommandError"));
    let shared_core = udl
        .split("interface SharedCore {")
        .nth(1)
        .and_then(|rest| rest.split("};").next())
        .expect("SharedCore");
    assert!(shared_core.contains("register_http_pusher("));
    assert!(shared_core.contains("delete_http_pusher("));
    assert!(shared_core.contains("pusher_set("));
    assert!(shared_core.contains("pusher_delete("));
    assert!(!shared_core.contains("command("));
    assert!(!shared_core.contains("matrix_login_password"));
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
            "DEVICE".to_owned(),
            "en-US".to_owned(),
        ))
        .expect_err("no HTTP pusher owner");
    let delete = rt
        .block_on(shared.delete_http_pusher(push_key.to_owned(), app_id.to_owned()))
        .expect_err("no HTTP pusher owner");

    let register_text = format!("{register:?}{register}");
    let delete_text = format!("{delete:?}{delete}");
    assert!(register_text.contains("p2-register-http-pusher-no-session"));
    assert!(delete_text.contains("p2-delete-http-pusher-no-session"));
    assert!(!register_text.contains("p4-s10-leftover-unavailable"));
    assert!(!delete_text.contains("p4-s10-leftover-unavailable"));

    let text = format!("{register_text}{delete_text}");
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
            "DEVICE".to_owned(),
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
                "DEVICEABC".to_owned(),
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
