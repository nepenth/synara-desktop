//! Opt-in live proof of the shipping narrow notification request.
//! Creates two disposable test-account devices and an encrypted private room.
//! The reader's full Core is closed before the new message is sent; only the
//! normal NSE request may fetch/decrypt it from the shared encrypted store.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use synara_core::{IosSecretVault, IosSecretVaultError, RoomCreateRequestDto, SharedCore};
use synara_nse_core::{NseCoreError, NsePreviewRequest, NseSecretVault, NseSecretVaultError};

#[derive(Clone, Default)]
struct Vault(Arc<Mutex<HashMap<String, Vec<u8>>>>);
impl IosSecretVault for Vault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
        Ok(self.0.lock().unwrap().get(&key).cloned())
    }
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
        self.0.lock().unwrap().insert(key, value);
        Ok(())
    }
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
        self.0.lock().unwrap().remove(&key);
        Ok(())
    }
}
impl NseSecretVault for Vault {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, NseSecretVaultError> {
        Ok(self.0.lock().unwrap().get(&key).cloned())
    }
}

struct Fixture {
    core: SharedCore,
    vault: Vault,
    user: String,
    homeserver: String,
    store: String,
    device: String,
}
impl Fixture {
    async fn login(second: bool, nonce: u128) -> Result<Self, &'static str> {
        let homeserver =
            std::env::var("SYNARA_LIVE_HOMESERVER").map_err(|_| "homeserver required")?;
        let prefix = if second {
            "SYNARA_LIVE_SECOND"
        } else {
            "SYNARA_LIVE"
        };
        let username =
            std::env::var(format!("{prefix}_USERNAME")).map_err(|_| "username required")?;
        let password =
            std::env::var(format!("{prefix}_PASSWORD")).map_err(|_| "password required")?;
        let user = if username.starts_with('@') {
            username
        } else {
            let url = url::Url::parse(&homeserver).map_err(|_| "invalid homeserver")?;
            format!(
                "@{}:{}",
                username,
                url.host_str().ok_or("missing homeserver host")?
            )
        };
        let root = std::env::temp_dir().join(format!("synara-nse-live-{nonce}-{second}"));
        std::fs::create_dir(&root).map_err(|_| "create fixture directory")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "protect fixture directory")?;
        }
        let store = root.to_string_lossy().into_owned();
        let vault = Vault::default();
        let core = SharedCore::new_with_secret_store(Box::new(vault.clone()));
        let login = core
            .login_with_password(user.clone(), homeserver.clone(), store.clone(), password)
            .await
            .map_err(|_| "product login failed")?;
        Ok(Self {
            core,
            vault,
            user,
            homeserver,
            store,
            device: login.device_id,
        })
    }
    async fn start(&self) -> Result<(), &'static str> {
        self.core
            .attach_session_owners()
            .await
            .map_err(|_| "attach failed")?;
        self.core.start_sync().await.map_err(|_| "sync failed")?;
        Ok(())
    }
    async fn wait_for_encrypted_room(&self, room_id: &str) -> Result<(), &'static str> {
        let until = Instant::now() + Duration::from_secs(40);
        while Instant::now() < until {
            if let Ok(snapshot) = self.core.room_list_snapshot().await {
                if snapshot.rooms.iter().any(|room| {
                    room.room_id == room_id && room.is_encrypted && room.membership == "join"
                }) {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err("encrypted joined room not projected")
    }
    async fn cleanup(self) -> bool {
        let revoked = self
            .core
            .revoke_server_session(self.user, self.device, self.homeserver)
            .await;
        let closed = self.core.logout().await;
        drop(self.core);
        let removed = std::fs::remove_dir_all(self.store);
        matches!(revoked, Ok(true)) && closed.is_ok() && removed.is_ok()
    }
}

async fn prove(
    reader: &Fixture,
    writer: &Fixture,
    room_id: &mut Option<String>,
) -> Result<(), &'static str> {
    reader.start().await?;
    writer.start().await?;
    let created = writer
        .core
        .room_create(RoomCreateRequestDto {
            name: Some("Synara encrypted notification proof".to_owned()),
            topic: None,
            room_alias_name: None,
            visibility: Some("private".to_owned()),
            preset: Some("private_chat".to_owned()),
            is_direct: false,
            encryption: true,
            invite: vec![reader.user.clone()],
            room_version: None,
            join_rule: None,
            knock: false,
            parent_room_id: None,
        })
        .await
        .map_err(|_| "create room failed")?;
    *room_id = Some(created.room_id.clone());
    reader
        .core
        .room_join(created.room_id.clone(), None)
        .await
        .map_err(|_| "join room failed")?;
    reader.wait_for_encrypted_room(&created.room_id).await?;
    writer.wait_for_encrypted_room(&created.room_id).await?;
    // Parent store is initialized by ordinary product sync, then all parent
    // owners are stopped. No parent may fetch this subsequent event.
    reader
        .core
        .logout()
        .await
        .map_err(|_| "close reader before notification")?;
    let body = "A new encrypted notification fixture";
    let sent = writer
        .core
        .send_text(
            created.room_id.clone(),
            body.to_owned(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|_| "encrypted send failed")?;
    let before = reader.vault.0.lock().unwrap().clone();
    let request = NsePreviewRequest::new(
        Box::new(reader.vault.clone()),
        reader.user.clone(),
        reader.homeserver.clone(),
        reader.store.clone(),
        created.room_id,
        sent.event_id,
    );
    let start = Instant::now();
    let result = request.resolve().await;
    eprintln!(
        "synara_nse_live stage=resolved elapsed_ms={} success={}",
        start.elapsed().as_millis(),
        result.is_ok()
    );
    let preview = result.map_err(|error| {
        let NseCoreError::Failed { code, .. } = error;
        // Source-selected code only; never emit SDK errors, tokens or IDs.
        eprintln!("synara_nse_live stage=failed code={code}");
        "narrow notification resolution failed"
    })?;
    if preview.event_type != "m.room.message" || preview.body.as_deref() != Some(body) {
        return Err("new encrypted preview body mismatch");
    }
    if *reader.vault.0.lock().unwrap() != before {
        return Err("NSE mutated secret vault");
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires two authorized SYNARA_LIVE_* test accounts; creates encrypted room and revokes fresh fixture devices"]
async fn new_encrypted_event_resolves_after_parent_core_stops() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let reader = Fixture::login(false, nonce).await.expect("reader setup");
    let writer = match Fixture::login(true, nonce).await {
        Ok(writer) => writer,
        Err(error) => {
            assert!(reader.cleanup().await, "reader cleanup");
            panic!("{error}");
        }
    };
    let mut room = None;
    let result = prove(&reader, &writer, &mut room).await;
    let mut room_cleanup = true;
    if let Some(room_id) = room {
        // Restore through the ordinary owner solely for fixture teardown.
        if reader.core.room_leave(room_id.clone()).await.is_err() {
            let restored = reader
                .core
                .restore_persisted_session(
                    reader.user.clone(),
                    reader.homeserver.clone(),
                    reader.store.clone(),
                )
                .await;
            if restored.is_ok() && reader.start().await.is_ok() {
                room_cleanup &= reader.core.room_leave(room_id.clone()).await.is_ok();
            } else {
                room_cleanup = false;
            }
        }
        room_cleanup &= writer.core.room_leave(room_id).await.is_ok();
    }
    let reader_cleanup = reader.cleanup().await;
    let writer_cleanup = writer.cleanup().await;
    assert!(
        reader_cleanup && writer_cleanup && room_cleanup,
        "fixture cleanup failed"
    );
    result.expect("live narrow NSE preview path");
}
