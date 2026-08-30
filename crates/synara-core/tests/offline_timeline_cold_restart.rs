//! Cold-restart proof for the production persisted timeline route.
//!
//! This test deliberately uses a real HTTP `/sync` once, the production
//! `ClientBuildConfig` SQLite state/cache layout, full client destruction, and
//! a fresh `NativeTimelineOwner` after the local homeserver has stopped. It
//! must not add a shell-owned message cache or retain the pre-restart client.

use std::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use matrix_sdk::config::SyncSettings;
use serde_json::{json, Value};
use synara_core::app::client_builder::{
    build_unauthenticated_client, ClientBuildConfig, TimeoutPolicy,
};
use synara_core::app::lifecycle::{restore_session_onto_client, SessionMaterial};
use synara_core::app::store::{AccountIdentity, StoreKeyMaterial};
use synara_core::app::timeline::{
    NativeTimelineOpenPosition, NativeTimelineOpenRequest, NativeTimelineOwner,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const USER_ID: &str = "@alice:localhost";
const ROOM_ID: &str = "!offline-proof:localhost";
const EVENT_ID: &str = "$offline-proof-event:localhost";
const EVENT_BODY: &str = "offline timeline cold restart proof";

fn temp_root() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("synara-offline-timeline-proof-{nanos}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create proof root");
    root
}

fn sync_response() -> Value {
    json!({
        "next_batch": "offline-proof-batch-1",
        "account_data": { "events": [] },
        "presence": { "events": [] },
        "to_device": { "events": [] },
        "device_lists": { "changed": [], "left": [] },
        "device_one_time_keys_count": {},
        "rooms": {
            "invite": {},
            "leave": {},
            "join": {
                ROOM_ID: {
                    "state": {
                        "events": [
                            {
                                "type": "m.room.create",
                                "state_key": "",
                                "sender": USER_ID,
                                "event_id": "$offline-proof-create:localhost",
                                "origin_server_ts": 1,
                                "content": {
                                    "creator": USER_ID,
                                    "room_version": "10"
                                }
                            },
                            {
                                "type": "m.room.member",
                                "state_key": USER_ID,
                                "sender": USER_ID,
                                "event_id": "$offline-proof-member:localhost",
                                "origin_server_ts": 2,
                                "content": {
                                    "membership": "join",
                                    "displayname": "Alice"
                                }
                            }
                        ]
                    },
                    "timeline": {
                        "events": [
                            {
                                "type": "m.room.message",
                                "sender": USER_ID,
                                "event_id": EVENT_ID,
                                "origin_server_ts": 3,
                                "content": {
                                    "msgtype": "m.text",
                                    "body": EVENT_BODY
                                }
                            }
                        ],
                        "limited": false,
                        "prev_batch": null
                    },
                    "ephemeral": { "events": [] },
                    "account_data": { "events": [] },
                    "unread_notifications": {
                        "highlight_count": 0,
                        "notification_count": 0
                    }
                }
            }
        }
    })
}

async fn serve_one_sync(listener: TcpListener, body: Value) -> Vec<String> {
    let body = serde_json::to_vec(&body).expect("serialize sync response");
    let versions = serde_json::to_vec(&json!({
        "versions": ["v1.11"],
        "unstable_features": {}
    }))
    .expect("serialize versions response");
    let mut paths = Vec::new();
    loop {
        let (mut socket, _) = listener.accept().await.expect("accept SDK request");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let count = socket.read(&mut chunk).await.expect("read SDK request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8(request).expect("HTTP request is utf-8");
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .expect("HTTP request path")
            .to_owned();
        paths.push(path.clone());

        let is_sync = path.starts_with("/_matrix/client/v3/sync")
            || path.starts_with("/_matrix/client/r0/sync")
            || path.starts_with("/_matrix/client/v4/sync");
        let (status, response_body) = if is_sync {
            ("200 OK", body.as_slice())
        } else if path.starts_with("/_matrix/client/versions") {
            ("200 OK", versions.as_slice())
        } else {
            ("404 Not Found", b"{}".as_slice())
        };
        let headers = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response_body.len()
        );
        socket
            .write_all(headers.as_bytes())
            .await
            .expect("write HTTP headers");
        socket
            .write_all(response_body)
            .await
            .expect("write HTTP body");
        socket.shutdown().await.expect("close HTTP response");

        if is_sync {
            return paths;
        }
    }
}

fn product_config(
    root: &std::path::Path,
    identity: AccountIdentity,
    lock_holder: &str,
) -> ClientBuildConfig {
    ClientBuildConfig::product_default(
        root,
        identity,
        Some(StoreKeyMaterial::from_bytes([0x5a; 32])),
    )
    .expect("product client config")
    .with_cross_process_store_lock_holder(lock_holder)
    .expect("proof lock holder")
    .with_timeouts(TimeoutPolicy {
        request_timeout: Duration::from_millis(500),
        retry_limit: 0,
    })
    .expect("bounded proof timeout")
}

#[test]
fn synchronized_room_reopens_from_sqlite_after_cold_restart_with_server_offline() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("proof runtime");
    runtime.block_on(async {
        let root = temp_root();
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind disposable homeserver");
        let address = listener.local_addr().expect("homeserver address");
        let homeserver = format!("http://{address}");
        let identity = AccountIdentity::new(USER_ID, &homeserver).expect("proof identity");
        let session = SessionMaterial::from_matrix_tokens(
            &identity,
            "OFFLINEPROOFDEVICE",
            "offline-proof-access-token",
            None,
        )
        .expect("proof session material");

        let server = tokio::spawn(serve_one_sync(listener, sync_response()));
        let first_client = build_unauthenticated_client(&product_config(
            &root,
            identity.clone(),
            "offline-proof-first",
        ))
        .await
        .expect("build first production client");
        restore_session_onto_client(&first_client, &identity, &session)
            .await
            .expect("restore first session");
        first_client
            .event_cache()
            .subscribe()
            .expect("subscribe authoritative event cache");
        first_client
            .sync_once(SyncSettings::default())
            .await
            .expect("process known event through real sync");
        let proof_room_id = matrix_sdk::ruma::RoomId::parse(ROOM_ID).expect("proof room id");
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let room = first_client.get_room(&proof_room_id);
                if let Some(room) = room {
                    let (cache, _handles) = room.event_cache().await.expect("proof room cache");
                    let events = cache.events().await.expect("read proof room cache");
                    if events.iter().any(|event| {
                        event
                            .event_id()
                            .is_some_and(|event_id| event_id.as_str() == EVENT_ID)
                    }) {
                        break;
                    }
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("sync event must reach the authoritative persistent event cache");
        let paths = server.await.expect("sync server task");
        assert_eq!(
            paths
                .iter()
                .filter(|path| path.contains("/_matrix/client/") && path.contains("/sync"))
                .count(),
            1,
            "proof must use exactly one pre-restart sync"
        );

        // The server task owns the listener and has now returned, so this is
        // the offline boundary. Drop every pre-restart SDK owner before rebuild.
        drop(first_client);
        tokio::time::sleep(Duration::from_millis(100)).await;

        let restored_client = build_unauthenticated_client(&product_config(
            &root,
            identity.clone(),
            "offline-proof-restored",
        ))
        .await
        .expect("build fresh production client from same SQLite root");
        restore_session_onto_client(&restored_client, &identity, &session)
            .await
            .expect("restore fresh session while server is offline");
        restored_client
            .event_cache()
            .subscribe()
            .expect("restore production event-cache subscription owner");
        let owner = NativeTimelineOwner::new(&restored_client, Arc::new(|_| {}), 2);
        let opened = tokio::time::timeout(
            Duration::from_secs(2),
            owner.open_at(NativeTimelineOpenRequest {
                room_id: ROOM_ID.to_owned(),
                position: NativeTimelineOpenPosition::LiveBottom,
            }),
        )
        .await
        .expect("offline timeline open must not wait for network")
        .expect("native timeline owner opens persisted room");

        let snapshot = serde_json::to_value(&opened.snapshot).expect("snapshot readback");
        let snapshot_text = snapshot.to_string();
        assert!(
            snapshot_text.contains(EVENT_ID),
            "cached event id must be read back"
        );
        assert!(
            snapshot_text.contains(EVENT_BODY),
            "cached message body must be read back"
        );

        drop(owner);
        drop(restored_client);
        let _ = fs::remove_dir_all(root);
    });
}
