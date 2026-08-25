//! One-shot, bounded Matrix notification resolution for Apple's NSE process.
//!
//! The caller can only read existing secrets. This module opens one account
//! store, restores one room, resolves one event, and drops every SDK owner
//! before its async future completes (including error and timeout paths).

use std::path::{Component, Path};
use std::time::Duration;

use matrix_sdk::ruma::{
    events::{room::message::MessageType, AnySyncMessageLikeEvent, AnySyncTimelineEvent},
    OwnedEventId, OwnedRoomId,
};
use matrix_sdk::store::RoomLoadSettings;
use matrix_sdk_ui::notification_client::{
    NotificationClient, NotificationEvent, NotificationProcessSetup, NotificationStatus,
};

use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig, TimeoutPolicy};
use crate::app::lifecycle::{
    restore_session_onto_client_with_room_load_settings, SessionMaterial, SessionMaterialId,
};
use crate::app::store::{AccountIdentity, StoreKeyId, StoreKeyMaterial, STORE_KEY_LEN};

const MAX_ARGUMENT_CHARACTERS: usize = 64 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const RESOLUTION_TIMEOUT: Duration = Duration::from_secs(20);
const STORE_LOCK_HOLDER: &str = "synara-nse-parent";

pub trait NseSecretReader: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, NsePreviewError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseEventPreview {
    pub event_type: String,
    pub sender_id: Option<String>,
    pub body: Option<String>,
    pub message_type: Option<String>,
    pub is_agent_approval: bool,
    pub origin_server_ts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsePreviewError {
    code: String,
    description: String,
}

impl NsePreviewError {
    pub fn failed(code: &str, description: &str) -> Self {
        Self {
            code: code.to_owned(),
            description: description.to_owned(),
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for NsePreviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.description)
    }
}

impl std::error::Error for NsePreviewError {}

fn failed(code: &'static str, description: &'static str) -> NsePreviewError {
    NsePreviewError::failed(code, description)
}

pub async fn resolve_event_preview(
    secrets: &dyn NseSecretReader,
    user_id: &str,
    homeserver_url: &str,
    store_root: &str,
    room_id: &str,
    event_id: &str,
) -> Result<NseEventPreview, NsePreviewError> {
    tokio::time::timeout(RESOLUTION_TIMEOUT, async {
        resolve_event_preview_unbounded(
            secrets,
            user_id,
            homeserver_url,
            store_root,
            room_id,
            event_id,
        )
        .await
    })
    .await
    .map_err(|_| {
        failed(
            "p4-s11-nse-resolution-timeout",
            "The notification resolution timed out.",
        )
    })?
}

async fn resolve_event_preview_unbounded(
    secrets: &dyn NseSecretReader,
    user_id: &str,
    homeserver_url: &str,
    store_root: &str,
    room_id: &str,
    event_id: &str,
) -> Result<NseEventPreview, NsePreviewError> {
    if [user_id, homeserver_url, store_root, room_id, event_id]
        .iter()
        .any(|value| value.len() > MAX_ARGUMENT_CHARACTERS)
    {
        return Err(failed(
            "p4-s11-nse-payload-oversize",
            "The notification request is too large.",
        ));
    }

    let identity = AccountIdentity::new(user_id, homeserver_url).map_err(|_| {
        failed(
            "p4-s3b-identity-invalid",
            "The notification account is invalid.",
        )
    })?;
    let root = valid_store_root(store_root)?;
    let parsed_room = OwnedRoomId::try_from(room_id.trim()).map_err(|_| event_unavailable())?;
    let parsed_event = OwnedEventId::try_from(event_id.trim()).map_err(|_| event_unavailable())?;

    let session_id = SessionMaterialId::from_identity(&identity);
    let session_blob = secrets.get(session_id.account())?.ok_or_else(|| {
        failed(
            "p4-s3b-material-missing",
            "The notification session is unavailable.",
        )
    })?;
    let session = SessionMaterial::from_sealed_blob(session_blob);

    let store_key_id = StoreKeyId::from_identity(&identity);
    let mut store_key_bytes = secrets.get(store_key_id.account())?.ok_or_else(|| {
        failed(
            "p4-s3b-restore-failed",
            "The notification store is unavailable.",
        )
    })?;
    if store_key_bytes.len() != STORE_KEY_LEN {
        store_key_bytes.fill(0);
        return Err(failed(
            "p4-s3b-restore-failed",
            "The notification store is unavailable.",
        ));
    }
    let mut key_array = [0_u8; STORE_KEY_LEN];
    key_array.copy_from_slice(&store_key_bytes);
    store_key_bytes.fill(0);
    let store_key = StoreKeyMaterial::from_bytes(key_array);

    let mut config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
        .and_then(|config| {
            config.with_timeouts(TimeoutPolicy {
                request_timeout: REQUEST_TIMEOUT,
                retry_limit: 0,
            })
        })
        .and_then(|config| config.with_cross_process_store_lock_holder(STORE_LOCK_HOLDER))
        .map_err(|_| {
            failed(
                "p4-s3b-restore-failed",
                "The notification store is unavailable.",
            )
        })?;
    config.handle_refresh_tokens = false;

    let client = build_unauthenticated_client(&config).await.map_err(|_| {
        failed(
            "p4-s3b-restore-failed",
            "The notification store is unavailable.",
        )
    })?;
    restore_session_onto_client_with_room_load_settings(
        &client,
        &identity,
        &session,
        RoomLoadSettings::One(parsed_room.clone()),
    )
    .await
    .map_err(|_| {
        failed(
            "p4-s3b-restore-failed",
            "The notification session could not be restored.",
        )
    })?;

    let notification_client =
        NotificationClient::new(client.clone(), NotificationProcessSetup::MultipleProcesses)
            .await
            .map_err(|_| {
                failed(
                    "p4-s11-nse-client-init-failed",
                    "The notification client could not be opened.",
                )
            })?;
    let status = notification_client
        .get_notification(&parsed_room, &parsed_event)
        .await
        .map_err(|_| {
            failed(
                "p4-s11-nse-event-fetch-failed",
                "The notification event could not be fetched.",
            )
        })?;

    let preview = preview_from_status(status)?;
    drop(notification_client);
    drop(client);
    Ok(preview)
}

fn valid_store_root(value: &str) -> Result<&Path, NsePreviewError> {
    let trimmed = value.trim();
    let path = Path::new(trimmed);
    if trimmed.is_empty()
        || !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(failed(
            "p4-s3b-store-root-invalid",
            "The notification store path is invalid.",
        ));
    }
    Ok(path)
}

fn preview_from_status(status: NotificationStatus) -> Result<NseEventPreview, NsePreviewError> {
    let NotificationStatus::Event(item) = status else {
        return Err(event_unavailable());
    };
    let NotificationEvent::Timeline(event) = &item.event else {
        return Err(event_unavailable());
    };
    let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) =
        event.as_ref()
    else {
        return Err(event_unavailable());
    };
    let Some(original) = message.as_original() else {
        return Err(event_unavailable());
    };

    Ok(NseEventPreview {
        event_type: "m.room.message".to_owned(),
        sender_id: Some(bounded(
            item.sender_display_name
                .as_deref()
                .unwrap_or_else(|| item.event.sender().as_str()),
            255,
        )),
        body: Some(bounded(original.content.body(), 240)),
        message_type: message_type(&original.content.msgtype).map(|value| bounded(value, 64)),
        is_agent_approval: crate::app::agent_approvals::is_agent_approval_prompt(
            original.content.body(),
        ),
        origin_server_ts: original.origin_server_ts.get().into(),
    })
}

fn event_unavailable() -> NsePreviewError {
    failed(
        "p4-s11-nse-event-not-in-store",
        "The notification event is unavailable.",
    )
}

fn bounded(value: &str, maximum_characters: usize) -> String {
    value.chars().take(maximum_characters).collect()
}

fn message_type(message: &MessageType) -> Option<&'static str> {
    match message {
        MessageType::Audio(_) => Some("m.audio"),
        MessageType::Emote(_) => Some("m.emote"),
        MessageType::File(_) => Some("m.file"),
        MessageType::Image(_) => Some("m.image"),
        MessageType::Location(_) => Some("m.location"),
        MessageType::Notice(_) => Some("m.notice"),
        MessageType::ServerNotice(_) => Some("m.server_notice"),
        MessageType::Text(_) => Some("m.text"),
        MessageType::Video(_) => Some("m.video"),
        MessageType::VerificationRequest(_) => Some("m.key.verification.request"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct RecordingReader {
        keys: Mutex<Vec<String>>,
    }

    impl NseSecretReader for RecordingReader {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, NsePreviewError> {
            self.keys.lock().expect("keys").push(key.to_owned());
            Ok(None)
        }
    }

    #[tokio::test]
    async fn missing_session_reads_only_the_derived_session_key_and_fails_closed() {
        let reader = RecordingReader::default();
        let error = resolve_event_preview(
            &reader,
            "@alice:example.org",
            "https://matrix.example.org",
            "/tmp/synara-nse-test-store",
            "!room:example.org",
            "$event:example.org",
        )
        .await
        .expect_err("missing session must fail closed");

        assert_eq!(error.code(), "p4-s3b-material-missing");
        let keys = reader.keys.lock().expect("keys");
        assert_eq!(keys.len(), 1);
        assert!(keys[0].starts_with("matrix-session:"));
        assert!(!error.to_string().contains("@alice"));
        assert!(!error.to_string().contains("matrix.example.org"));
    }

    #[tokio::test]
    async fn invalid_identifiers_never_touch_the_secret_store() {
        let reader = RecordingReader::default();
        let error = resolve_event_preview(
            &reader,
            "@alice:example.org",
            "https://matrix.example.org",
            "/tmp/synara-nse-test-store",
            "not-a-room",
            "$event:example.org",
        )
        .await
        .expect_err("invalid room must fail closed");

        assert_eq!(error.code(), "p4-s11-nse-event-not-in-store");
        assert!(reader.keys.lock().expect("keys").is_empty());
    }
}
