//! Session-scoped SDK media-store retention. Not a second cache backend.

use std::time::Duration;

use matrix_sdk::media::MediaRetentionPolicy;
use matrix_sdk::{Client, RoomState};
use tokio::task::JoinHandle;

use super::policy::{build_media_retention_policy_spec, shortest_joined_max_lifetime};

const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

/// Applies `MediaRetentionPolicy` at session start and when joined-room
/// retention may have changed. Abort on drop (logout).
pub struct NativeMediaRetentionOwner {
    task: JoinHandle<()>,
}

impl NativeMediaRetentionOwner {
    pub fn start(client: &Client, session_generation: u64) -> Result<Self, &'static str> {
        if session_generation == 0 {
            return Err("v-media.retention-owner-invalid-generation");
        }
        client
            .user_id()
            .ok_or("v-media.retention-owner-no-session")?;
        let client = client.clone();
        let task = tokio::spawn(async move {
            loop {
                apply_media_retention_policy(&client).await;
                tokio::time::sleep(REFRESH_INTERVAL).await;
            }
        });
        Ok(Self { task })
    }
}

impl Drop for NativeMediaRetentionOwner {
    fn drop(&mut self) {
        self.task.abort();
    }
}

pub async fn joined_room_max_lifetimes(client: &Client) -> Vec<Duration> {
    let mut lifetimes = Vec::new();
    let config = match client.get_retention_configuration().await {
        Ok(config) => config,
        Err(error) if error.is_endpoint_not_implemented() => return lifetimes,
        Err(_) => return lifetimes,
    };
    for room in client.joined_rooms() {
        if room.state() != RoomState::Joined {
            continue;
        }
        if let Some(policy) = room.effective_retention_with_server_config(&config) {
            if let Some(max_lifetime) = policy.max_lifetime() {
                lifetimes.push(max_lifetime);
            }
        }
    }
    lifetimes
}

pub async fn shortest_joined_room_max_lifetime(client: &Client) -> Option<Duration> {
    let lifetimes = joined_room_max_lifetimes(client).await;
    shortest_joined_max_lifetime(&lifetimes)
}

async fn apply_media_retention_policy(client: &Client) {
    let lifetimes = joined_room_max_lifetimes(client).await;
    let hs_upload = match client.load_or_fetch_max_upload_size().await {
        Ok(size) => {
            let bytes: u64 = size.into();
            (bytes > 0).then_some(bytes)
        }
        Err(_) => None,
    };
    let spec = build_media_retention_policy_spec(&lifetimes, hs_upload);
    let policy = MediaRetentionPolicy::empty()
        .with_max_cache_size(Some(spec.max_cache_size))
        .with_max_file_size(Some(spec.max_file_size))
        .with_last_access_expiry(Some(spec.last_access_expiry))
        .with_cleanup_frequency(Some(spec.cleanup_frequency));
    let media = client.media();
    if media.set_media_retention_policy(policy).await.is_err() {
        return;
    }
    let _ = media.clean().await;
}
