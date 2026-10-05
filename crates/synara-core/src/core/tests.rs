//! Core transport and owner regression tests.

use super::*;
use crate::app::sync::SyncReadiness;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{PlatformStatus, SecretVault, UnavailableSecretVault};
use crate::transport::{CommandFuture, CommandRegistry};

const TEST_HTTP_USER_AGENT: &str = "Synara-Core-Test/1.0";

fn unconfigured_platform_status() -> PlatformSyncStatus {
    PlatformSyncStatus::new(SyncReadiness::Unconfigured, 0, false, None, None)
        .expect("unconfigured status is a valid string-free projection")
}

fn unavailable_platform_crypto_status() -> PlatformCryptoStatus {
    PlatformCryptoStatus::new(0, false, PlatformCryptoCrossSigningState::Unavailable)
        .expect("unavailable is a valid string-free crypto projection")
}

fn available_platform_media_config() -> PlatformMediaConfig {
    PlatformMediaConfig::new(16 * 1024 * 1024)
        .expect("a normal upload limit is a valid closed media projection")
}

#[tokio::test]
async fn typed_recovery_operations_fail_closed_without_native_owner_or_valid_input() {
    let core = Core::new(Arc::new(TestPlatform));
    assert!(core.backup_setup("passphrase").await.is_err());
    assert!(core.backup_repair("recovery-secret").await.is_err());
    assert!(core.secret_storage_bootstrap("passphrase").await.is_err());
    assert!(core.secret_storage_unlock("recovery-secret").await.is_err());
    assert!(core.secret_storage_reset("passphrase").await.is_err());
    assert!(core.backup_setup("").await.is_err());
    assert!(core
        .secret_storage_reset(&"x".repeat(100_001))
        .await
        .is_err());
}

#[derive(Default)]
struct TestPlatform;
impl Platform for TestPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async { Ok(unconfigured_platform_status()) })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async { Ok(unavailable_platform_crypto_status()) })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async { Ok(available_platform_media_config()) })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

/// A test shell may supply only the closed platform status/error types.
/// It has no field in which a diagnostic string can enter Core.
struct StatusPlatform {
    status: Result<PlatformSyncStatus, crate::platform::PlatformSyncStatusError>,
}

impl Platform for StatusPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async move { self.status })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async { Ok(unavailable_platform_crypto_status()) })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async { Ok(available_platform_media_config()) })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

/// A crypto test shell can supply only the closed platform projection or
/// closed static error; neither variant can carry hostile shell text.
struct CryptoStatusPlatform {
    status: Result<PlatformCryptoStatus, crate::platform::PlatformCryptoStatusError>,
}

impl Platform for CryptoStatusPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async { Ok(unconfigured_platform_status()) })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async move { self.status })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async { Ok(available_platform_media_config()) })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

/// Cross-signing tests can supply only the closed private projection or
/// four static errors. There is no identity, user id, SDK/client/store,
/// key, secret, or raw diagnostic field in this seam.
struct CrossSigningStatusPlatform {
    status: Result<PlatformCrossSigningStatus, PlatformCrossSigningStatusError>,
}

impl Platform for CrossSigningStatusPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async { Ok(unconfigured_platform_status()) })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async { Ok(unavailable_platform_crypto_status()) })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async move { self.status })
    }
    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async { Ok(available_platform_media_config()) })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

/// Secret-storage tests can supply only the fixed closed projection/error.
/// There is no field in which a secret, key, identifier, SDK value, or raw
/// diagnostic could reach Core.
struct SecretStorageStatusPlatform {
    status: Result<PlatformSecretStorageStatus, PlatformSecretStorageStatusError>,
}

impl Platform for SecretStorageStatusPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async { Ok(unconfigured_platform_status()) })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async { Ok(unavailable_platform_crypto_status()) })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }
    fn secret_storage_status(&self) -> crate::platform::SecretStorageStatusFuture<'_> {
        Box::pin(async move { self.status })
    }
    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async { Ok(available_platform_media_config()) })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

/// Media tests can supply only the bounded projection or one static error.
/// There is no field in which an SDK/client/cache/store value or raw text
/// could reach Core.
struct MediaConfigPlatform {
    config: Result<PlatformMediaConfig, PlatformMediaConfigError>,
}

impl Platform for MediaConfigPlatform {
    fn emit(&self, _envelope: crate::transport::MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }
    fn http_user_agent(&self) -> String {
        TEST_HTTP_USER_AGENT.into()
    }
    fn sync_status(&self) -> crate::platform::SyncStatusFuture<'_> {
        Box::pin(async { Ok(unconfigured_platform_status()) })
    }
    fn crypto_status(&self) -> crate::platform::CryptoStatusFuture<'_> {
        Box::pin(async { Ok(unavailable_platform_crypto_status()) })
    }
    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> crate::platform::MediaConfigFuture<'_> {
        Box::pin(async move { self.config })
    }
    fn notify(&self, _candidate: crate::dto::NotificationCandidate) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn set_badge(&self, _count: u64) -> Result<(), MatrixIpcError> {
        Ok(())
    }
    fn status(&self, _status: PlatformStatus) -> Result<(), MatrixIpcError> {
        Ok(())
    }
}

fn session() -> SessionSnapshot {
    SessionSnapshot {
        session_generation: 1,
        user_id: "@alice:example.org".into(),
        device_id: "DEVICE".into(),
        homeserver_url: "https://example.org".into(),
        display_name: None,
        avatar_url: None,
        lifecycle: SessionLifecycle::Ready,
        crypto_ready: true,
    }
}

#[test]
fn matrix_session_snapshot_response_uses_exact_desktop_wire_keys() {
    assert_eq!(
        serde_json::to_value(MatrixSessionSnapshotResponse::from(None)).unwrap(),
        serde_json::json!({"status":"logged_out"})
    );

    let response = MatrixSessionSnapshotResponse::from(Some(SessionSnapshot {
        session_generation: 7,
        user_id: "@alice:example.org".into(),
        device_id: "DEVICE".into(),
        homeserver_url: "https://example.org".into(),
        display_name: Some("Alice".into()),
        avatar_url: Some("mxc://example.org/avatar".into()),
        lifecycle: SessionLifecycle::Ready,
        crypto_ready: true,
    }));
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        serde_json::json!({
            "status":"logged_in",
            "user_id":"@alice:example.org",
            "device_id":"DEVICE",
            "homeserver_url":"https://example.org",
            "sessionGeneration":7,
        })
    );
}

#[tokio::test]
async fn default_registry_dispatches_matrix_session_snapshot() {
    let core = Core::new(Arc::new(TestPlatform));
    assert_eq!(
        core.registered_commands(),
        vec![
            "matrix_agent_approval_decide",
            "matrix_agent_approval_history_snapshot",
            "matrix_agent_approvals_list",
            "matrix_agent_notification_preferences_set",
            "matrix_agent_notification_preferences_snapshot",
            "matrix_backup_status",
            "matrix_composer_clear_reply_draft",
            "matrix_composer_get_reply_draft",
            "matrix_composer_set_reply_draft",
            "matrix_cross_signing_setup",
            "matrix_cross_signing_status",
            "matrix_crypto_status",
            "matrix_device_delete_cancel",
            "matrix_device_delete_start",
            "matrix_device_rename",
            "matrix_device_snapshot",
            "matrix_edit_message",
            "matrix_enable_room_encrypted_state",
            "matrix_get_global_image_packs",
            "matrix_get_own_profile",
            "matrix_get_room_directory_visibility",
            "matrix_get_room_image_packs",
            "matrix_get_user_image_pack",
            "matrix_ignored_users_ignore",
            "matrix_ignored_users_snapshot",
            "matrix_ignored_users_unignore",
            "matrix_inbox_notifications",
            "matrix_invites_accept",
            "matrix_invites_block_sender",
            "matrix_invites_decline",
            "matrix_invites_report_spam",
            "matrix_invites_snapshot",
            "matrix_later_clear_completed",
            "matrix_later_complete",
            "matrix_later_mark_reminded",
            "matrix_later_snapshot",
            "matrix_later_snooze",
            "matrix_later_upsert",
            "matrix_login_flows",
            "matrix_mdirect_add",
            "matrix_mdirect_remove",
            "matrix_mdirect_snapshot",
            "matrix_media_config",
            "matrix_media_preview",
            "matrix_message_search",
            "matrix_notification_decide",
            "matrix_notification_dismiss",
            "matrix_notification_focus_set",
            "matrix_notification_pending_snapshot",
            "matrix_pinned_events",
            "matrix_poll_respond",
            "matrix_presence_set",
            "matrix_presence_snapshot",
            "matrix_presence_subscribe",
            "matrix_presence_unsubscribe",
            "matrix_push_rules_add_keyword",
            "matrix_push_rules_remove_keyword",
            "matrix_push_rules_set_default",
            "matrix_push_rules_set_mention",
            "matrix_push_rules_snapshot",
            "matrix_reaction_ensure",
            "matrix_reaction_redact",
            "matrix_register_flows",
            "matrix_restricted_join_reparent",
            "matrix_room_ban",
            "matrix_room_create",
            "matrix_room_creators_snapshot",
            "matrix_room_directory_cancel",
            "matrix_room_directory_protocols",
            "matrix_room_directory_search",
            "matrix_room_invite",
            "matrix_room_join",
            "matrix_room_join_rule_snapshot",
            "matrix_room_key_transfer_status",
            "matrix_room_kick",
            "matrix_room_leave",
            "matrix_room_list_snapshot",
            "matrix_room_members_snapshot",
            "matrix_room_notes_complete_todo",
            "matrix_room_notes_delete",
            "matrix_room_notes_move_todo",
            "matrix_room_notes_snapshot",
            "matrix_room_notes_upsert",
            "matrix_room_notification_set",
            "matrix_room_notification_snapshot",
            "matrix_room_notifications_snapshot",
            "matrix_room_power_level_tags_snapshot",
            "matrix_room_power_levels_snapshot",
            "matrix_room_retention",
            "matrix_room_set_favorite",
            "matrix_room_set_join_rule",
            "matrix_room_set_power_level",
            "matrix_room_set_power_level_tags",
            "matrix_room_set_power_levels",
            "matrix_room_set_read_state",
            "matrix_room_unban",
            "matrix_rtc_transports_refresh",
            "matrix_rtc_transports_snapshot",
            "matrix_secret_storage_status",
            "matrix_send_poll",
            "matrix_send_state_event",
            "matrix_send_text",
            "matrix_session_snapshot",
            "matrix_set_encrypted_state_events_setting",
            "matrix_set_global_image_packs",
            "matrix_set_own_avatar",
            "matrix_set_own_display_name",
            "matrix_set_room_avatar",
            "matrix_set_room_directory_visibility",
            "matrix_set_room_image_pack",
            "matrix_set_room_name",
            "matrix_set_room_topic",
            "matrix_set_user_image_pack",
            "matrix_space_child_remove",
            "matrix_space_child_set",
            "matrix_space_children_snapshot",
            "matrix_space_hierarchy_snapshot",
            "matrix_space_parents_snapshot",
            "matrix_sync_status",
            "matrix_thread_list",
            "matrix_threepid_add_email",
            "matrix_threepid_delete",
            "matrix_threepid_request_email_token",
            "matrix_threepid_snapshot",
            "matrix_timeline_call_decline",
            "matrix_timeline_close",
            "matrix_timeline_edit_text",
            "matrix_timeline_event_readback",
            "matrix_timeline_follow_live",
            "matrix_timeline_forward_media",
            "matrix_timeline_forward_text",
            "matrix_timeline_jump_latest",
            "matrix_timeline_open",
            "matrix_timeline_paginate",
            "matrix_timeline_pin",
            "matrix_timeline_poll_vote",
            "matrix_timeline_reaction_toggle",
            "matrix_timeline_redact",
            "matrix_timeline_report",
            "matrix_timeline_set_read_state",
            "matrix_timeline_snapshot",
            "matrix_timeline_timestamp_to_event",
            "matrix_timeline_unpin",
            "matrix_typing_set",
            "matrix_typing_snapshot",
            "matrix_user_directory_search",
            "matrix_user_status_clear",
            "matrix_user_status_set",
            "matrix_user_status_snapshot",
            "matrix_verification_accept",
            "matrix_verification_begin_sas",
            "matrix_verification_cancel",
            "matrix_verification_confirm",
            "matrix_verification_dismiss",
            "matrix_verification_list",
            "matrix_verification_mismatch",
            "matrix_verification_start",
            "matrix_widget_close",
            "matrix_widget_open",
            "matrix_widget_post",
            "matrix_widget_subscribe",
            "matrix_widgets_list",
        ]
    );

    let request = CommandEnvelope {
        command: "matrix_session_snapshot".into(),
        session_generation: 1,
        request_id: None,
        payload: serde_json::Value::Null,
    };
    assert_eq!(
        core.command(request.clone()).await.unwrap().payload,
        serde_json::json!({"status":"logged_out"})
    );

    core.open(session()).await.unwrap();
    assert_eq!(
        core.command(request).await.unwrap().payload,
        serde_json::json!({
            "status":"logged_in",
            "user_id":"@alice:example.org",
            "device_id":"DEVICE",
            "homeserver_url":"https://example.org",
            "sessionGeneration":1,
        })
    );
}

#[tokio::test]
async fn core_sync_status_uses_exact_desktop_wire_shape() {
    let core = Core::new(Arc::new(TestPlatform));
    let request = CommandEnvelope {
        command: "matrix_sync_status".into(),
        session_generation: 0,
        request_id: Some("sync-status-fixture".into()),
        payload: serde_json::Value::Null,
    };

    let response = core
        .command(request)
        .await
        .expect("status observation succeeds");
    assert_eq!(response.command, "matrix_sync_status");
    assert_eq!(response.session_generation, 0);
    assert_eq!(response.request_id.as_deref(), Some("sync-status-fixture"));
    assert_eq!(
        response.payload,
        serde_json::json!({
            "readiness": "unconfigured",
            "sessionGeneration": 0,
            "offlineModeEnabled": false,
            "failureDiagnosticId": null,
            "slidingSyncCapable": null,
        })
    );
}

#[tokio::test]
async fn core_crypto_status_uses_exact_desktop_wire_shape() {
    let ready = PlatformCryptoStatus::new(9, true, PlatformCryptoCrossSigningState::Ready)
        .expect("ready is a valid closed crypto projection");
    let response = Core::new(Arc::new(CryptoStatusPlatform { status: Ok(ready) }))
        .command(CommandEnvelope {
            command: "matrix_crypto_status".into(),
            session_generation: 0,
            request_id: Some("crypto-status-fixture".into()),
            payload: serde_json::Value::Null,
        })
        .await
        .expect("crypto status observation succeeds");

    assert_eq!(response.command, "matrix_crypto_status");
    assert_eq!(response.session_generation, 0);
    assert_eq!(
        response.request_id.as_deref(),
        Some("crypto-status-fixture")
    );
    assert_eq!(
        response.payload,
        serde_json::json!({
            "sessionGeneration": 9,
            "encryptionEnabled": true,
            "crossSigningState": "ready",
        })
    );
}

#[tokio::test]
async fn core_cross_signing_status_recreates_every_closed_legacy_truth_table_row() {
    for (
        private_state,
        own_identity,
        readiness,
        publication,
        private_identity,
        own_identity_verification,
        bootstrap,
    ) in [
        (
            PlatformCrossSigningPrivateState::Unavailable,
            PlatformCrossSigningOwnIdentity::Missing,
            "unavailable",
            "missing",
            "missing",
            "missing",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Unavailable,
            PlatformCrossSigningOwnIdentity::Unverified,
            "unavailable",
            "published",
            "missing",
            "unverified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Unavailable,
            PlatformCrossSigningOwnIdentity::Verified,
            "unavailable",
            "published",
            "missing",
            "verified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Missing,
            PlatformCrossSigningOwnIdentity::Missing,
            "setup_required",
            "missing",
            "missing",
            "missing",
            "needed",
        ),
        (
            PlatformCrossSigningPrivateState::Missing,
            PlatformCrossSigningOwnIdentity::Unverified,
            "recovery_required",
            "published",
            "missing",
            "unverified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Missing,
            PlatformCrossSigningOwnIdentity::Verified,
            "recovery_required",
            "published",
            "missing",
            "verified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Partial,
            PlatformCrossSigningOwnIdentity::Missing,
            "setup_required",
            "missing",
            "partial",
            "missing",
            "needed",
        ),
        (
            PlatformCrossSigningPrivateState::Partial,
            PlatformCrossSigningOwnIdentity::Unverified,
            "recovery_required",
            "published",
            "partial",
            "unverified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Partial,
            PlatformCrossSigningOwnIdentity::Verified,
            "recovery_required",
            "published",
            "partial",
            "verified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Complete,
            PlatformCrossSigningOwnIdentity::Missing,
            "setup_required",
            "missing",
            "complete",
            "missing",
            "needed",
        ),
        (
            PlatformCrossSigningPrivateState::Complete,
            PlatformCrossSigningOwnIdentity::Unverified,
            "verification_required",
            "published",
            "complete",
            "unverified",
            "not_needed",
        ),
        (
            PlatformCrossSigningPrivateState::Complete,
            PlatformCrossSigningOwnIdentity::Verified,
            "ready",
            "published",
            "complete",
            "verified",
            "not_needed",
        ),
    ] {
        let status = PlatformCrossSigningStatus::new(9, private_state, own_identity)
            .expect("each closed row is representable through Platform");
        let response = Core::new(Arc::new(CrossSigningStatusPlatform { status: Ok(status) }))
            .command(CommandEnvelope {
                command: "matrix_cross_signing_status".into(),
                session_generation: 0,
                request_id: Some("cross-signing-status-fixture".into()),
                payload: serde_json::Value::Null,
            })
            .await
            .expect("closed status row serializes through Core");
        assert_eq!(response.command, "matrix_cross_signing_status");
        assert_eq!(response.session_generation, 0);
        assert_eq!(
            response.request_id.as_deref(),
            Some("cross-signing-status-fixture")
        );
        assert_eq!(
            response.payload,
            serde_json::json!({
                "sessionGeneration": 9,
                "readiness": readiness,
                "masterSigning": publication,
                "selfSigning": publication,
                "userSigning": publication,
                "privateIdentity": private_identity,
                "ownIdentityVerification": own_identity_verification,
                "bootstrap": bootstrap,
            })
        );
    }
}

#[tokio::test]
async fn core_cross_signing_status_rejects_payload_and_maps_every_static_platform_error() {
    let private_text = "@alice:private.example token=secret password=secret key=secret";
    let malformed = Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_cross_signing_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "private": private_text }),
        })
        .await
        .expect_err("cross-signing status is a zero-argument observation");
    assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        malformed.diagnostic_id.as_deref(),
        Some("p2-cross-signing-status-invalid-payload")
    );

    for (status, category, diagnostic_id) in [
        (
            Err(PlatformCrossSigningStatusError::NoSession),
            MatrixIpcErrorCategory::Forbidden,
            "v-crypto.2-cross-signing-requires-session",
        ),
        (
            Err(PlatformCrossSigningStatusError::UserMissing),
            MatrixIpcErrorCategory::Forbidden,
            "v-crypto.2-cross-signing-user-missing",
        ),
        (
            Err(PlatformCrossSigningStatusError::IdentityQueryFailed),
            MatrixIpcErrorCategory::Unknown,
            "v-crypto.2-cross-signing-identity-query-failed",
        ),
        (
            Err(PlatformCrossSigningStatusError::UnsafeSessionGeneration),
            MatrixIpcErrorCategory::SdkInvariant,
            "p2-cross-signing-status-unsafe-session-generation",
        ),
    ] {
        let error = Core::new(Arc::new(CrossSigningStatusPlatform { status }))
            .command(CommandEnvelope {
                command: "matrix_cross_signing_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("closed Platform failure must become a static Core error");
        assert_eq!(error.category, category);
        assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
        let serialized = serde_json::to_string(&error).expect("static error serializes");
        for forbidden in [
            "alice",
            "private.example",
            "token",
            "secret",
            "password",
            "key",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "cross-signing Platform/Core error must not reflect hostile text: {forbidden}"
            );
        }
    }
    assert!(!serde_json::to_string(&malformed)
        .unwrap()
        .contains(private_text));
}

#[tokio::test]
async fn core_secret_storage_status_recreates_every_closed_state_and_missing_secret_case() {
    let states = [
        (
            PlatformSecretStorageState::Unavailable,
            false,
            PlatformSecretStorageAction::UnlockRequired,
            "unavailable",
            "unlock_required",
        ),
        (
            PlatformSecretStorageState::NotSetUp,
            false,
            PlatformSecretStorageAction::BootstrapRequired,
            "not_set_up",
            "bootstrap_required",
        ),
        (
            PlatformSecretStorageState::Locked,
            false,
            PlatformSecretStorageAction::UnlockRequired,
            "locked",
            "unlock_required",
        ),
        (
            PlatformSecretStorageState::Ready,
            true,
            PlatformSecretStorageAction::None,
            "ready",
            "none",
        ),
    ];
    for (state, unlocked, action, state_label, action_label) in states {
        for bits in 0_u8..16 {
            let missing = crate::platform::PlatformSecretStorageMissingSecrets::new(
                bits & 1 != 0,
                bits & 2 != 0,
                bits & 4 != 0,
                bits & 8 != 0,
            );
            let status = PlatformSecretStorageStatus::new(
                9, state, true, unlocked, true, true, true, missing, action,
            )
            .expect("all closed legacy state/missing-secret rows are representable");
            let response = Core::new(Arc::new(SecretStorageStatusPlatform { status: Ok(status) }))
                .command(CommandEnvelope {
                    command: "matrix_secret_storage_status".into(),
                    session_generation: 0,
                    request_id: Some("secret-storage-status-fixture".into()),
                    payload: serde_json::Value::Null,
                })
                .await
                .expect("closed status row serializes through Core");
            let mut missing_secrets = Vec::new();
            if bits & 1 != 0 {
                missing_secrets.push("cross_signing_master");
            }
            if bits & 2 != 0 {
                missing_secrets.push("cross_signing_self_signing");
            }
            if bits & 4 != 0 {
                missing_secrets.push("cross_signing_user_signing");
            }
            if bits & 8 != 0 {
                missing_secrets.push("encryption_backup");
            }
            assert_eq!(response.command, "matrix_secret_storage_status");
            assert_eq!(response.session_generation, 0);
            assert_eq!(
                response.request_id.as_deref(),
                Some("secret-storage-status-fixture")
            );
            assert_eq!(
                response.payload,
                serde_json::json!({
                    "sessionGeneration": 9,
                    "state": state_label,
                    "exists": true,
                    "unlocked": unlocked,
                    "defaultKeySet": true,
                    "passphraseConfigured": true,
                    "bootstrapReady": true,
                    "missingSecrets": missing_secrets,
                    "action": action_label,
                })
            );
        }
    }
}

#[tokio::test]
async fn core_secret_storage_status_rejects_payload_and_maps_every_static_platform_error() {
    let private_text = "https://private.example token=secret recovery_key=secret";
    let malformed = Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_secret_storage_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "private": private_text }),
        })
        .await
        .expect_err("secret-storage status is a zero-argument observation");
    assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        malformed.diagnostic_id.as_deref(),
        Some("p2-secret-storage-status-invalid-payload")
    );

    for (status, category, diagnostic_id) in [
        (
            Err(PlatformSecretStorageStatusError::NoSession),
            MatrixIpcErrorCategory::Forbidden,
            "v-crypto.4-secret-storage-requires-session",
        ),
        (
            Err(PlatformSecretStorageStatusError::DefaultKeyLoadFailed),
            MatrixIpcErrorCategory::RecoveryFailure,
            "v-crypto.4-status-default-key-failed",
        ),
        (
            Err(PlatformSecretStorageStatusError::KeyInfoLoadFailed),
            MatrixIpcErrorCategory::RecoveryFailure,
            "v-crypto.4-status-key-info-failed",
        ),
        (
            Err(PlatformSecretStorageStatusError::SecretCheckFailed),
            MatrixIpcErrorCategory::RecoveryFailure,
            "v-crypto.4-status-secret-check-failed",
        ),
        (
            Err(PlatformSecretStorageStatusError::UnsafeSessionGeneration),
            MatrixIpcErrorCategory::SdkInvariant,
            "p2-secret-storage-status-invalid-platform-projection",
        ),
        (
            Err(PlatformSecretStorageStatusError::InvalidSnapshot),
            MatrixIpcErrorCategory::SdkInvariant,
            "p2-secret-storage-status-invalid-platform-projection",
        ),
    ] {
        let error = Core::new(Arc::new(SecretStorageStatusPlatform { status }))
            .command(CommandEnvelope {
                command: "matrix_secret_storage_status".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("closed Platform failure must become a static Core error");
        assert_eq!(error.category, category);
        assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
        let serialized = serde_json::to_string(&error).expect("static error serializes");
        for forbidden in ["private.example", "token=", "recovery_key="] {
            assert!(
                !serialized.contains(forbidden),
                "secret-storage Platform/Core error must not reflect hostile text: {forbidden}"
            );
        }
    }
    assert!(!serde_json::to_string(&malformed)
        .unwrap()
        .contains(private_text));
}

#[tokio::test]
async fn core_media_config_uses_the_exact_legacy_wire_object() {
    let response = Core::new(Arc::new(MediaConfigPlatform {
        config: Ok(PlatformMediaConfig::new(MAX_WIRE_COUNTER)
            .expect("maximum safe upload size projects through Platform")),
    }))
    .command(CommandEnvelope {
        command: "matrix_media_config".into(),
        session_generation: 0,
        request_id: Some("media-config-fixture".into()),
        payload: serde_json::Value::Null,
    })
    .await
    .expect("closed media config serializes through Core");

    assert_eq!(response.command, "matrix_media_config");
    assert_eq!(response.session_generation, 0);
    assert_eq!(response.request_id.as_deref(), Some("media-config-fixture"));
    assert_eq!(
        response.payload,
        serde_json::json!({ "m.upload.size": MAX_WIRE_COUNTER })
    );
}

#[tokio::test]
async fn core_media_config_rejects_payload_and_maps_each_platform_error_statically() {
    let private_text = "https://private.example token=secret password=secret key=secret";
    let malformed = Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_media_config".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "private": private_text }),
        })
        .await
        .expect_err("media config is a zero-argument command");
    assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        malformed.diagnostic_id.as_deref(),
        Some("p2-media-config-invalid-payload")
    );

    for (config, category, diagnostic_id) in [
        (
            Err(PlatformMediaConfigError::NoSession),
            MatrixIpcErrorCategory::Forbidden,
            "p2-media-config-no-session",
        ),
        (
            Err(PlatformMediaConfigError::LoadFailed),
            MatrixIpcErrorCategory::Unknown,
            "p2-media-config-load-failed",
        ),
        (
            Err(PlatformMediaConfigError::UnsafeSize),
            MatrixIpcErrorCategory::MediaTooLarge,
            "p2-media-config-unsafe-size",
        ),
    ] {
        let error = Core::new(Arc::new(MediaConfigPlatform { config }))
            .command(CommandEnvelope {
                command: "matrix_media_config".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("closed platform media failure must reach the bridge as a static error");
        assert_eq!(error.category, category);
        assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic_id));
        let serialized = serde_json::to_string(&error).expect("static error serializes");
        for forbidden in ["private.example", "token", "secret", "password", "key"] {
            assert!(
                !serialized.contains(forbidden),
                "media Platform/Core error must not reflect hostile text: {forbidden}"
            );
        }
    }
    assert!(!serde_json::to_string(&malformed)
        .unwrap()
        .contains(private_text));
}

#[tokio::test]
async fn crypto_status_projection_is_closed_and_core_errors_are_static() {
    let private_text = "https://private.example token=secret key=secret";
    let valid = PlatformCryptoStatus::new(7, true, PlatformCryptoCrossSigningState::Partial)
        .expect("partial is a valid closed projection");
    assert!(!format!("{valid:?}").contains(private_text));

    // The Platform error contains no dynamic data, and Core replaces it
    // with a fixed command error before the public transport boundary.
    let error = Core::new(Arc::new(CryptoStatusPlatform {
        status: Err(crate::platform::PlatformCryptoStatusError::InvalidSnapshot),
    }))
    .command(CommandEnvelope {
        command: "matrix_crypto_status".into(),
        session_generation: 0,
        request_id: None,
        payload: serde_json::Value::Null,
    })
    .await
    .expect_err("closed Platform errors have no public crypto payload");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-crypto-status-platform-unavailable")
    );

    let malformed = Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_crypto_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "private": private_text }),
        })
        .await
        .expect_err("crypto status accepts no payload");
    assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        malformed.diagnostic_id.as_deref(),
        Some("p2-crypto-status-invalid-payload")
    );

    for public_error in [error, malformed] {
        let serialized = serde_json::to_string(&public_error).expect("static error serializes");
        for forbidden in ["private.example", "token", "secret", "key"] {
            assert!(
                !serialized.contains(forbidden),
                "hostile crypto text must not cross the Platform/Core seam: {forbidden}"
            );
        }
    }

    // Validate the Core-owned response contract independently of the
    // Platform constructor so an accidental future mapping cannot emit an
    // impossible encryption/state pairing.
    let invalid_response = MatrixCryptoStatusResponse {
        session_generation: 7,
        encryption_enabled: false,
        cross_signing_state: MatrixCryptoCrossSigningStateResponse::Ready,
    };
    assert!(!invalid_response.is_valid());
}

#[tokio::test]
async fn core_sync_status_constructs_the_only_public_failure_diagnostic() {
    let status = PlatformSyncStatus::new(
        SyncReadiness::Failed,
        9,
        true,
        Some(PlatformSyncFailure::SyncService),
        Some(true),
    )
    .expect("closed sync failure is a valid Platform projection");
    let response = Core::new(Arc::new(StatusPlatform { status: Ok(status) }))
        .command(CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 9,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect("closed Platform failure serializes through Core");

    assert_eq!(
        response.payload,
        serde_json::json!({
            "readiness": "failed",
            "sessionGeneration": 9,
            "offlineModeEnabled": true,
            "failureDiagnosticId": "p4.1-sync-service-error",
            "slidingSyncCapable": true,
        })
    );
}

#[tokio::test]
async fn hostile_desktop_diagnostic_is_rejected_before_platform_core_or_public_transport() {
    let private_text: &'static str = Box::leak(
        "https://private.example token=secret password=secret"
            .to_owned()
            .into_boxed_str(),
    );
    let hostile_desktop_snapshot = SyncReadinessSnapshot {
        readiness: SyncReadiness::Failed,
        session_generation: 9,
        offline_mode_enabled: true,
        failure_diagnostic_id: Some(private_text),
        sliding_sync_capable: Some(false),
    };

    // This is the desktop-side normalization step. Its typed result has no
    // diagnostic-string field, so the hostile value cannot enter Platform.
    let normalized = PlatformSyncStatus::from_desktop_snapshot(hostile_desktop_snapshot);
    assert_eq!(
        normalized,
        Err(crate::platform::PlatformSyncStatusError::InvalidSnapshot)
    );
    assert!(!format!("{normalized:?}").contains(private_text));

    let error = Core::new(Arc::new(StatusPlatform { status: normalized }))
        .command(CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 9,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("rejected desktop diagnostic has no public status payload");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-sync-status-platform-unavailable")
    );
    let public_error = serde_json::to_string(&error).expect("static Core error serializes");
    for forbidden in ["private.example", "token", "secret", "password"] {
        assert!(
                !public_error.contains(forbidden),
                "hostile desktop diagnostic must not cross Platform/Core or public transport: {forbidden}"
            );
    }
}

#[tokio::test]
async fn core_sync_status_fails_closed_with_static_errors() {
    let malformed = Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_sync_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"private": "token=secret"}),
        })
        .await
        .expect_err("status command must accept no payload");
    assert_eq!(malformed.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        malformed.diagnostic_id.as_deref(),
        Some("p2-sync-status-invalid-payload")
    );

    let error = Core::new(Arc::new(StatusPlatform {
        status: Err(crate::platform::PlatformSyncStatusError::Unavailable),
    }))
    .command(CommandEnvelope {
        command: "matrix_sync_status".into(),
        session_generation: 0,
        request_id: None,
        payload: serde_json::Value::Null,
    })
    .await
    .expect_err("opaque platform errors must not cross the Core transport");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-sync-status-platform-unavailable")
    );
}

fn assert_test_user_agent(request: &str) {
    let user_agent = request
        .split("\r\n")
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("user-agent")
                .then_some(value.trim())
        })
        .expect("core auth probe must send a user-agent");
    assert_eq!(user_agent, TEST_HTTP_USER_AGENT);
}

async fn serve_login_flows_once(listener: &tokio::net::TcpListener) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut socket, _) = listener.accept().await.expect("accept login-flow request");
    let mut request = [0_u8; 2048];
    let read = socket
        .read(&mut request)
        .await
        .expect("read login-flow request");
    let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
    assert!(
        request.starts_with("GET /_matrix/client/v3/login "),
        "handler must request only the login-types endpoint"
    );
    assert_test_user_agent(request);
    let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.token","get_login_token":true},{"type":"m.login.application_service"},{"type":"m.login.custom"}]}"#;
    let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    socket
        .write_all(response.as_bytes())
        .await
        .expect("write login-flow response");
}

#[tokio::test]
async fn core_login_flows_uses_exact_react_payload_and_response_json() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind login-flow server");
    let address = listener.local_addr().expect("login-flow address");
    let server = tokio::spawn(async move { serve_login_flows_once(&listener).await });
    let core = Core::new(Arc::new(TestPlatform));

    let response = core
        .command(CommandEnvelope {
            command: "matrix_login_flows".into(),
            session_generation: 1,
            request_id: Some("login-flows-fixture".into()),
            payload: serde_json::json!({
                "homeserverUrl": format!("http://{address}"),
            }),
        })
        .await
        .expect("login-flow handler succeeds");

    assert_eq!(
        response.payload,
        serde_json::json!({
            "flows": [
                {"kind":"password","matrixType":"m.login.password"},
                {"kind":"token","matrixType":"m.login.token","getLoginToken":true},
                {"kind":"application_service","matrixType":"m.login.application_service"},
                {"kind":"unknown","matrixType":"m.login.custom"},
            ]
        })
    );
    server.await.expect("login-flow server task");
}

#[tokio::test]
async fn core_login_flows_rejects_malformed_missing_and_unsafe_input_privately() {
    let core = Core::new(Arc::new(TestPlatform));
    for payload in [
        serde_json::Value::Null,
        serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
    ] {
        let error = core
            .command(CommandEnvelope {
                command: "matrix_login_flows".into(),
                session_generation: 1,
                request_id: None,
                payload,
            })
            .await
            .expect_err("malformed or missing payload must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-login-flows-invalid-payload")
        );
    }

    let unsafe_url = "https://private.example.invalid/../must-not-appear";
    let error = core
        .command(CommandEnvelope {
            command: "matrix_login_flows".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::json!({"homeserverUrl": unsafe_url}),
        })
        .await
        .expect_err("unsafe homeserver must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p3.1-invalid-homeserver-url")
    );
    assert!(!format!("{error:?}").contains(unsafe_url));
}

async fn serve_register_flows_once(
    listener: &tokio::net::TcpListener,
    status: u16,
    body: &'static str,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut socket, _) = listener
        .accept()
        .await
        .expect("accept registration-flow request");
    let mut request = [0_u8; 4096];
    let read = socket
        .read(&mut request)
        .await
        .expect("read registration-flow request");
    let request = std::str::from_utf8(&request[..read]).expect("HTTP request is text");
    assert_test_user_agent(request);
    let (headers, request_body) = request
        .split_once("\r\n\r\n")
        .expect("registration request has headers and body");
    assert!(
        headers.starts_with("POST /_matrix/client/v3/register "),
        "handler must request only the empty registration-probe endpoint"
    );
    let headers_lower = headers.to_ascii_lowercase();
    assert!(headers_lower.contains("content-type: application/json"));
    assert_eq!(request_body, "{}", "probe must use only an empty JSON body");
    for forbidden in [
        "authorization:",
        "access_token",
        "refresh_token",
        "password",
        "registration_token",
        "client_secret",
        "captcha",
        "threepid",
        "session",
    ] {
        assert!(
            !request.to_ascii_lowercase().contains(forbidden),
            "registration probe request must not contain {forbidden}"
        );
    }

    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        429 => "Too Many Requests",
        _ => "Error",
    };
    let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
    socket
        .write_all(response.as_bytes())
        .await
        .expect("write registration-flow response");
}

async fn core_register_flows_request(
    address: std::net::SocketAddr,
) -> Result<CommandResponseEnvelope, MatrixIpcError> {
    Core::new(Arc::new(TestPlatform))
        .command(CommandEnvelope {
            command: "matrix_register_flows".into(),
            session_generation: 1,
            request_id: Some("register-flows-fixture".into()),
            payload: serde_json::json!({
                "homeserverUrl": format!("http://{address}"),
            }),
        })
        .await
}

#[tokio::test]
async fn core_register_flows_uses_exact_react_wire_fixtures_and_empty_post() {
    const FLOW_REQUIRED_UIAA: &str = r#"{
            "flows":[
                {"stages":["m.login.terms","m.login.dummy"]},
                {"stages":["m.login.registration_token"]}
            ],
            "completed":["m.login.terms"],
            "params":{"m.login.terms":{"policies":[]}},
            "session":"opaque-uia-session"
        }"#;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind registration-flow server");
    let address = listener.local_addr().expect("registration-flow address");
    let server = tokio::spawn(async move {
        serve_register_flows_once(&listener, 401, FLOW_REQUIRED_UIAA).await;
    });

    let response = core_register_flows_request(address)
        .await
        .expect("registration UIAA probe succeeds");
    assert_eq!(
        response.payload,
        serde_json::json!({
            "status":"flow_required",
            "session":"opaque-uia-session",
            "flows":[
                {"stages":["m.login.terms","m.login.dummy"]},
                {"stages":["m.login.registration_token"]}
            ],
            "completed":["m.login.terms"],
            "params":{"m.login.terms":{"policies":[]}},
        })
    );
    assert_eq!(response.command, "matrix_register_flows");
    assert_eq!(
        response.request_id.as_deref(),
        Some("register-flows-fixture")
    );
    server.await.expect("registration-flow server task");
}

#[tokio::test]
async fn core_register_flows_preserves_all_non_uia_probe_wire_variants() {
    for (status, expected) in [
        (200, serde_json::json!({"status":"invalid_request"})),
        (400, serde_json::json!({"status":"invalid_request"})),
        (403, serde_json::json!({"status":"registration_disabled"})),
        (429, serde_json::json!({"status":"rate_limited"})),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind registration-flow server");
        let address = listener.local_addr().expect("registration-flow address");
        let server = tokio::spawn(async move {
            serve_register_flows_once(&listener, status, "{").await;
        });

        let response = core_register_flows_request(address)
            .await
            .expect("known registration-probe status has a safe wire outcome");
        assert_eq!(response.payload, expected, "status {status}");
        server.await.expect("registration-flow server task");
    }
}

#[tokio::test]
async fn core_register_flows_rejects_non_react_or_sensitive_payloads_privately() {
    let core = Core::new(Arc::new(TestPlatform));
    for payload in [
        serde_json::Value::Null,
        serde_json::json!({"homeserver_url":"https://not-the-react-key.invalid"}),
        serde_json::json!({
            "homeserverUrl":"https://not-the-react-key.invalid",
            "password":"must-not-cross-core",
        }),
        serde_json::json!({
            "homeserverUrl":"https://not-the-react-key.invalid",
            "session":"must-not-continue-uia",
        }),
    ] {
        let error = core
            .command(CommandEnvelope {
                command: "matrix_register_flows".into(),
                session_generation: 1,
                request_id: None,
                payload,
            })
            .await
            .expect_err("malformed or sensitive probe payload must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-register-flows-invalid-payload")
        );
        assert!(!format!("{error:?}").contains("must-not"));
    }

    let unsafe_url = "https://private.example.invalid/../must-not-appear";
    let error = core
        .command(CommandEnvelope {
            command: "matrix_register_flows".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::json!({"homeserverUrl": unsafe_url}),
        })
        .await
        .expect_err("unsafe homeserver must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p3.1-invalid-homeserver-url")
    );
    assert!(!format!("{error:?}").contains(unsafe_url));
}

#[tokio::test]
async fn core_register_flows_malformed_uiaa_fails_closed_without_raw_body() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind registration-flow server");
    let address = listener.local_addr().expect("registration-flow address");
    let raw_body = r#"{"flows":"not-an-array","error":"private remote body"}"#;
    let server = tokio::spawn(async move {
        serve_register_flows_once(&listener, 401, raw_body).await;
    });

    let error = core_register_flows_request(address)
        .await
        .expect_err("malformed UIAA response must fail closed");
    assert_eq!(
        error.category,
        MatrixIpcErrorCategory::UnsupportedCapability
    );
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-register-flows-uiaa-response-invalid")
    );
    assert!(!format!("{error:?}").contains("private remote body"));
    server.await.expect("registration-flow server task");
}

#[tokio::test]
async fn core_open_and_close_only_manage_safe_session_projection() {
    let core = Core::new(Arc::new(TestPlatform));
    assert!(core.session_snapshot().unwrap().is_none());
    core.open(session()).await.unwrap();
    assert_eq!(core.session_snapshot().unwrap(), Some(session()));
    core.close().await.unwrap();
    assert!(core.session_snapshot().unwrap().is_none());
}

#[tokio::test]
async fn command_registry_dispatches_one_typed_envelope() {
    let mut registry = CommandRegistry::new();
    registry
        .register(
            "matrix_login_flows",
            |_state: Arc<CoreState>, request: CommandEnvelope| -> CommandFuture {
                Box::pin(async move { Ok(request.payload) })
            },
        )
        .unwrap();
    let core = Core::with_registry(Arc::new(TestPlatform), registry);
    let response = core
        .command(CommandEnvelope {
            command: "matrix_login_flows".into(),
            session_generation: 1,
            request_id: Some("r1".into()),
            payload: serde_json::json!({"safe":true}),
        })
        .await
        .unwrap();
    assert_eq!(response.payload, serde_json::json!({"safe":true}));
    assert_eq!(core.registered_commands(), vec!["matrix_login_flows"]);
}

#[tokio::test]
async fn matrix_typing_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_typing_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("typing snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-typing-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_presence_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"userId":"@alice:example.org"}),
        })
        .await
        .expect_err("presence snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_widgets_list_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_widgets_list".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "experimentalWidgetsEnabled": true,
                "roomId": "!r:example.org",
                "agentWidgets": []
            }),
        })
        .await
        .expect_err("widget list without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-widgets-list-no-session")
    );
}

#[tokio::test]
async fn matrix_widget_post_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_widget_post".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "experimentalWidgetsEnabled": true,
                "sessionId": "w1",
                "message": "{}"
            }),
        })
        .await
        .expect_err("widget post without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-widget-post-no-session")
    );
}

#[tokio::test]
async fn matrix_presence_snapshot_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"userId":"@alice:example.org","token":"no"}),
        })
        .await
        .expect_err("presence snapshot must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-snapshot-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_presence_subscribe_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_subscribe".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"userId":"@alice:example.org"}),
        })
        .await
        .expect_err("presence subscribe without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-subscribe-no-session")
    );
}

#[tokio::test]
async fn matrix_presence_subscribe_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_subscribe".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"userId":"@alice:example.org","token":"no"}),
        })
        .await
        .expect_err("presence subscribe must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-subscribe-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_presence_unsubscribe_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_unsubscribe".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"subscriptionId":"presence-1-0"}),
        })
        .await
        .expect_err("presence unsubscribe without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-unsubscribe-no-session")
    );
}

#[tokio::test]
async fn matrix_presence_unsubscribe_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_unsubscribe".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"subscriptionId":"presence-1-0","token":"no"}),
        })
        .await
        .expect_err("presence unsubscribe must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-unsubscribe-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_presence_set_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"state":"online","statusMsg":"coffee"}),
        })
        .await
        .expect_err("presence set without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-set-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("coffee"));
    assert!(!text.contains("online"));
}

#[tokio::test]
async fn matrix_presence_set_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_presence_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"state":"online","token":"no"}),
        })
        .await
        .expect_err("presence set must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-presence-set-invalid-payload")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_rtc_transports_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_rtc_transports_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("rtc transport snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-rtc-transports-snapshot-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("widget"));
    assert!(!text.contains("livekit"));
}

#[tokio::test]
async fn matrix_rtc_transports_snapshot_rejects_unknown_payload() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_rtc_transports_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"token":"no"}),
        })
        .await
        .expect_err("rtc transport snapshot must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-rtc-transports-snapshot-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_rtc_transports_snapshot_static_unsupported_without_homeserver() {
    let core = Core::new(Arc::new(TestPlatform));
    let owner = NativeRtcTransportsOwner::from_static_snapshot(
        7,
        NativeRtcTransportsSnapshot::unsupported(7),
    );
    core.attach_rtc_transports(Arc::new(owner))
        .expect("attach static rtc owner");
    let response = core
        .command(CommandEnvelope {
            command: "matrix_rtc_transports_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect("static unsupported snapshot");
    let snapshot: NativeRtcTransportsSnapshot =
        serde_json::from_value(response.payload).expect("snapshot payload");
    assert_eq!(
        snapshot.status,
        crate::app::rtc_transports::NativeRtcTransportsStatus::Unsupported
    );
    assert!(snapshot.transports.is_empty());
    let raw = serde_json::to_string(&snapshot).expect("serialize");
    for forbidden in ["widget", "jwt", "accessToken", "password"] {
        assert!(!raw.contains(forbidden), "{raw}");
    }
}

#[tokio::test]
async fn matrix_user_status_set_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_user_status_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"emoji":"☕","text":"secret-status-text"}),
        })
        .await
        .expect_err("user status set without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-user-status-set-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("secret-status-text"));
    assert!(!text.contains("☕"));
}

#[tokio::test]
async fn matrix_user_status_set_rejects_presence_state_and_unknown_keys() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_user_status_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"state":"online","emoji":"☕","text":"hi"}),
        })
        .await
        .expect_err("user status set must not accept presence state keys");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-user-status-set-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_user_status_write_capability_missing_and_success_ack() {
    let core = Core::new(Arc::new(TestPlatform));
    let missing = crate::app::user_status::NativeUserStatusOwner::from_static_snapshot(
        7,
        crate::app::user_status::NativeUserStatusSnapshot {
            session_generation: 7,
            user_id: "@alice:example.org".into(),
            user_status: None,
            in_call: None,
        },
        false,
    );
    core.attach_user_status(Arc::new(missing))
        .expect("attach static user-status owner");
    let error = core
        .command(CommandEnvelope {
            command: "matrix_user_status_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"emoji":"☕","text":"secret-status-text"}),
        })
        .await
        .expect_err("missing MSC4426 capability must fail closed");
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("v-user-status-unsupported")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("secret-status-text"));

    let core = Core::new(Arc::new(TestPlatform));
    let ready = crate::app::user_status::NativeUserStatusOwner::from_static_snapshot(
        8,
        crate::app::user_status::NativeUserStatusSnapshot {
            session_generation: 8,
            user_id: "@alice:example.org".into(),
            user_status: None,
            in_call: Some(crate::app::user_status::NativeInCall {
                call_joined_ts: Some(1_720_000_000),
            }),
        },
        true,
    );
    core.attach_user_status(Arc::new(ready))
        .expect("attach capable user-status owner");
    let response = core
        .command(CommandEnvelope {
            command: "matrix_user_status_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"emoji":"☕","text":"in a meeting"}),
        })
        .await
        .expect("capable set");
    let ack: NativeUserStatusWriteResult = serde_json::from_value(response.payload).expect("ack");
    assert_eq!(ack.status, "ok");
    let raw = serde_json::to_string(&ack).expect("serialize");
    assert!(!raw.contains("☕"));
    assert!(!raw.contains("in a meeting"));
    assert!(!raw.contains("emoji"));
    assert!(!raw.contains("text"));

    let snapshot = core
        .command(CommandEnvelope {
            command: "matrix_user_status_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"userId":"@bob:example.org"}),
        })
        .await
        .expect("snapshot");
    let body: NativeUserStatusSnapshot =
        serde_json::from_value(snapshot.payload).expect("snapshot body");
    assert_eq!(body.user_id, "@bob:example.org");
    assert!(body.in_call.is_some());
}

#[tokio::test]
async fn matrix_verification_accept_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_accept".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification accept without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-accept-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_accept_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_accept".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification accept must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-accept-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_begin_sas_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_begin_sas".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification begin_sas without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-begin-sas-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_begin_sas_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_begin_sas".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification begin_sas must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-begin-sas-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_cancel_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_cancel".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification cancel without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-cancel-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_cancel_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_cancel".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification cancel must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-cancel-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_confirm_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_confirm".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification confirm without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-confirm-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_confirm_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_confirm".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification confirm must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-confirm-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_dismiss_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_dismiss".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification dismiss without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-dismiss-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_dismiss_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_dismiss".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification dismiss must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-dismiss-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_mismatch_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_mismatch".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1"}),
        })
        .await
        .expect_err("verification mismatch without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-mismatch-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_mismatch_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_mismatch".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"flowId":"flow-1","token":"no"}),
        })
        .await
        .expect_err("verification mismatch must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-mismatch-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_start_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_start".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"deviceId":"DEVICE"}),
        })
        .await
        .expect_err("verification start without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-start-no-session")
    );
}

#[tokio::test]
async fn matrix_verification_start_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_start".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"deviceId":"DEVICE","token":"no"}),
        })
        .await
        .expect_err("verification start must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-start-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_verification_list_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_verification_list".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("verification list without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-verification-list-no-session")
    );
}

#[tokio::test]
async fn matrix_backup_status_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_backup_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("backup status without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-backup-status-no-session")
    );
}

#[tokio::test]
async fn matrix_agent_approvals_list_requires_session_and_rejects_unknown_input() {
    let core = Core::new(Arc::new(TestPlatform));
    for (payload, expected) in [
        (serde_json::json!({}), MatrixIpcErrorCategory::Forbidden),
        (
            serde_json::json!({"discoveryActive": true}),
            MatrixIpcErrorCategory::Forbidden,
        ),
        (
            serde_json::json!({"discoveryActive": "yes"}),
            MatrixIpcErrorCategory::SdkInvariant,
        ),
        (
            serde_json::json!({"roomId": "!unexpected:example.org"}),
            MatrixIpcErrorCategory::SdkInvariant,
        ),
    ] {
        let error = core
            .command(CommandEnvelope {
                command: "matrix_agent_approvals_list".into(),
                session_generation: 0,
                request_id: None,
                payload,
            })
            .await
            .expect_err("inbox must reject invalid requests");
        assert_eq!(error.category, expected);
    }
}

#[tokio::test]
async fn matrix_agent_approval_decide_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_agent_approval_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!room:example.org",
                "eventId":"$event:example.org",
                "actionId":"agent-approval.approve-once"
            }),
        })
        .await
        .expect_err("approval decision without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("agent-approval-no-session")
    );
}

#[tokio::test]
async fn matrix_agent_approval_history_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_agent_approval_history_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("approval history snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("agent-approval-history-no-session")
    );
}

#[tokio::test]
async fn matrix_agent_approval_decide_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_agent_approval_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!room:example.org",
                "eventId":"$event:example.org",
                "actionId":"agent-approval.approve-once",
                "reaction":"♾️"
            }),
        })
        .await
        .expect_err("approval decision must reject caller-selected reactions");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("agent-approval-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_room_key_transfer_status_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_key_transfer_status".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("room-key transfer status without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-key-transfer-status-no-session")
    );
}

#[tokio::test]
async fn matrix_cross_signing_setup_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_cross_signing_setup".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("cross-signing setup without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-cross-signing-setup-no-session")
    );
}

#[tokio::test]
async fn matrix_room_list_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_list_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("room-list snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-list-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_inbox_notifications_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let result = core
        .command(CommandEnvelope {
            command: "matrix_inbox_notifications".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "limit": 30, "only": "highlight" }),
        })
        .await
        .expect_err("no owner must not return a false empty Inbox");
    assert_eq!(result.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        result.diagnostic_id.as_deref(),
        Some("inbox-notifications.no-session")
    );
}

#[tokio::test]
async fn matrix_invites_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_invites_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("invites snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-invites-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_invites_accept_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_invites_accept".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("invite accept without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-invites-accept-no-session")
    );
}

#[tokio::test]
async fn matrix_invites_decline_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_invites_decline".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("invite decline without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-invites-decline-no-session")
    );
}

#[tokio::test]
async fn matrix_invites_report_spam_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_invites_report_spam".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("invite report spam without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-invites-report-spam-no-session")
    );
}

#[tokio::test]
async fn matrix_invites_block_sender_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_invites_block_sender".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("invite block sender without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-invites-block-sender-no-session")
    );
}

#[tokio::test]
async fn matrix_room_directory_protocols_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_directory_protocols".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("directory protocols without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-directory-protocols-no-session")
    );
}

#[tokio::test]
async fn matrix_room_directory_search_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_directory_search".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "sessionGeneration":1,
                "requestId":1,
                "limit":20
            }),
        })
        .await
        .expect_err("directory search without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-directory-search-no-session")
    );
}

#[tokio::test]
async fn matrix_room_directory_cancel_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_directory_cancel".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "sessionGeneration":1,
                "requestId":1
            }),
        })
        .await
        .expect_err("directory cancel without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-directory-cancel-no-session")
    );
}

#[test]
fn directory_search_owner_errors_keep_classified_categories() {
    let cases = [
        (
            "v-rooms.directory-federation-forbidden",
            MatrixIpcErrorCategory::Forbidden,
        ),
        (
            "v-rooms.directory-network-failed",
            MatrixIpcErrorCategory::Connectivity,
        ),
        (
            "v-rooms.directory-server-not-found",
            MatrixIpcErrorCategory::HomeserverUnavailable,
        ),
        (
            "v-rooms.directory-rate-limited",
            MatrixIpcErrorCategory::RateLimited,
        ),
        (
            "v-rooms.directory-stale-generation-after-request",
            MatrixIpcErrorCategory::StaleSessionGeneration,
        ),
    ];
    for (diagnostic, category) in cases {
        let error = directory_search_owner_error(diagnostic);
        assert_eq!(error.category, category);
        assert_eq!(error.diagnostic_id.as_deref(), Some(diagnostic));
    }
}

#[tokio::test]
async fn matrix_device_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("device snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_device_rename_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_rename".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"deviceId":"DEVICE","displayName":"laptop"}),
        })
        .await
        .expect_err("device rename without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-rename-no-session")
    );
}

#[tokio::test]
async fn matrix_device_rename_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_rename".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "deviceId":"DEVICE",
                "displayName":"laptop",
                "token":"no"
            }),
        })
        .await
        .expect_err("device rename must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-rename-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_device_delete_start_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_delete_start".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"deviceIds":["OTHER"]}),
        })
        .await
        .expect_err("device delete start without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-delete-start-no-session")
    );
}

#[tokio::test]
async fn matrix_device_delete_start_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_delete_start".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"deviceIds":["OTHER"],"token":"no"}),
        })
        .await
        .expect_err("device delete start must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-delete-start-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_device_delete_cancel_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_delete_cancel".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"operationId":1,"sessionGeneration":1}),
        })
        .await
        .expect_err("device delete cancel without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-delete-cancel-no-session")
    );
}

#[tokio::test]
async fn matrix_device_delete_cancel_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_device_delete_cancel".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "operationId":1,
                "sessionGeneration":1,
                "token":"no"
            }),
        })
        .await
        .expect_err("device delete cancel must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-device-delete-cancel-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_set_room_name_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_room_name".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","name":"Room"}),
        })
        .await
        .expect_err("set room name without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-room-name-no-session")
    );
}

#[tokio::test]
async fn matrix_send_state_event_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_send_state_event".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventType":"m.room.canonical_alias",
                "stateKey":"",
                "content":{"alias":"#r:example.org"}
            }),
        })
        .await
        .expect_err("send state event without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-send-state-event-no-session")
    );
}

#[tokio::test]
async fn matrix_enable_room_encrypted_state_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_enable_room_encrypted_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "encryptStateEvents":true
            }),
        })
        .await
        .expect_err("enable encrypted state without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-enable-room-encrypted-state-no-session")
    );
}

#[tokio::test]
async fn matrix_set_encrypted_state_events_setting_does_not_need_a_session() {
    let core = Core::new(Arc::new(TestPlatform));
    let payload = core
        .command(CommandEnvelope {
            command: "matrix_set_encrypted_state_events_setting".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "enabled": false }),
        })
        .await
        .expect("setting command is session-free")
        .payload;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["enabled"], false);
    assert!(!crate::app::room_ops::encrypted_state_events_setting_enabled());
    let _ = core
        .command(CommandEnvelope {
            command: "matrix_set_encrypted_state_events_setting".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "enabled": true }),
        })
        .await
        .expect("restore default-on");
    assert!(crate::app::room_ops::encrypted_state_events_setting_enabled());
}

#[tokio::test]
async fn matrix_set_room_topic_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_room_topic".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","topic":"Hello"}),
        })
        .await
        .expect_err("set room topic without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-room-topic-no-session")
    );
}

#[tokio::test]
async fn matrix_set_room_avatar_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_room_avatar".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","mxc":""}),
        })
        .await
        .expect_err("set room avatar without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-room-avatar-no-session")
    );
}

#[tokio::test]
async fn matrix_set_own_display_name_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_own_display_name".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"displayName":"Alice"}),
        })
        .await
        .expect_err("set own display name without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-own-display-name-no-session")
    );
}

#[tokio::test]
async fn matrix_set_own_avatar_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_own_avatar".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"mxc":"mxc://example.org/abc"}),
        })
        .await
        .expect_err("set own avatar without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-own-avatar-no-session")
    );
}

#[tokio::test]
async fn matrix_get_own_profile_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_get_own_profile".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("get own profile without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-get-own-profile-no-session")
    );
}

#[tokio::test]
async fn matrix_ignored_users_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_ignored_users_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("ignored-users snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-ignored-users-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_user_directory_search_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_user_directory_search".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "term": "alice" }),
        })
        .await
        .expect_err("user-directory search without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-user-directory-search-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("alice"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_message_search_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let term = "s7MessageSearchSecretTerm";
    let error = core
        .command(CommandEnvelope {
            command: "matrix_message_search".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "term": term, "rooms": ["!r:example.org"] }),
        })
        .await
        .expect_err("message search without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-message-search-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains(term));
    assert!(!text.contains("!r:example.org"));
    assert!(!text.contains("syt_"));
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_push_rules_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_push_rules_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("push-rules snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-push-rules-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_notification_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_notification_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("room-notification snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-notification-snapshot-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("!r:example.org"));
}

#[tokio::test]
async fn matrix_room_notifications_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_notifications_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("room-notifications snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-notifications-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_notification_decide_round_trip_through_attached_owner() {
    use crate::app::notifications::NativeNotificationDecisionOwner;

    let core = Core::new(Arc::new(TestPlatform));
    core.attach_notification_decisions(Arc::new(NativeNotificationDecisionOwner::for_tests(7)))
        .expect("decision owner attaches");

    let response = core
        .command(CommandEnvelope {
            command: "matrix_notification_focus_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "roomId": null }),
        })
        .await
        .expect("focus clear succeeds");
    assert_eq!(response.payload, serde_json::json!({ "status": "ok" }));

    // The renderer can no longer hand Core a mode or highlight verdict:
    // the retired wire fields are rejected before any policy runs.
    let legacy_payload = core
        .command(CommandEnvelope {
            command: "matrix_notification_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$m1",
                "kind": "message",
                "title": "Room",
                "body": "Hello",
                "roomMode": "mute",
                "highlight": false,
            }),
        })
        .await
        .expect_err("renderer-supplied policy fields are rejected");
    assert_eq!(
        legacy_payload.diagnostic_id.as_deref(),
        Some("p2-notification-decide-invalid-payload")
    );

    // A message decision needs the SDK event; the table-only test owner
    // has no bound client and must fail closed rather than guess.
    let unbound_message = core
        .command(CommandEnvelope {
            command: "matrix_notification_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$m1",
                "kind": "message",
                "title": "Room",
                "body": "Hello",
            }),
        })
        .await
        .expect_err("message decisions fail closed without a client");
    assert_eq!(
        unbound_message.diagnostic_id.as_deref(),
        Some("v-notify.no-client")
    );

    let shown = core
        .command(CommandEnvelope {
            command: "matrix_notification_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$i1",
                "kind": "invite",
                "title": "Invitation",
                "body": "You have 1 new invitation request.",
            }),
        })
        .await
        .expect("decide succeeds");
    assert_eq!(shown.payload["decision"], serde_json::json!("show"));
    let candidate_id = shown.payload["candidate"]["candidateId"]
        .as_str()
        .expect("show carries a candidate id")
        .to_owned();

    let pending = core
        .command(CommandEnvelope {
            command: "matrix_notification_pending_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect("pending snapshot succeeds");
    assert_eq!(pending.payload["candidates"].as_array().unwrap().len(), 1);
    assert_eq!(
        pending.payload["delivery"],
        serde_json::json!({ "delivered": 0, "failed": 0, "unreported": 0 })
    );

    // The delivery receipt travels with the acknowledgement; a failed OS
    // delivery releases the candidate and is counted, never retried.
    let dismissed = core
        .command(CommandEnvelope {
            command: "matrix_notification_dismiss".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "candidateId": candidate_id, "outcome": "failed" }),
        })
        .await
        .expect("dismiss succeeds");
    assert_eq!(
        dismissed.payload,
        serde_json::json!({
            "dismissed": true,
            "delivery": { "delivered": 0, "failed": 1, "unreported": 0 },
        })
    );
    let receipt_vocabulary = core
        .command(CommandEnvelope {
            command: "matrix_notification_dismiss".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "candidateId": candidate_id, "outcome": "retry" }),
        })
        .await
        .expect_err("delivery receipts use the closed vocabulary");
    assert_eq!(
        receipt_vocabulary.diagnostic_id.as_deref(),
        Some("p2-notification-dismiss-invalid-payload")
    );

    // Unknown kind vocabulary fails closed with a static diagnostic.
    let invalid = core
        .command(CommandEnvelope {
            command: "matrix_notification_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId": "!r:example.org",
                "kind": "loud",
                "title": "Room",
                "body": "Hello",
            }),
        })
        .await
        .expect_err("unknown kind must fail closed");
    assert_eq!(invalid.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        invalid.diagnostic_id.as_deref(),
        Some("v-notify.invalid-kind")
    );
}

#[tokio::test]
async fn matrix_notification_commands_without_owner_fail_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let decide = core
        .command(CommandEnvelope {
            command: "matrix_notification_decide".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId": "!r:example.org",
                "eventId": "$e1",
                "kind": "message",
                "title": "Room",
                "body": "New message",
            }),
        })
        .await
        .expect_err("notification decide without an attached owner must fail closed");
    assert_eq!(decide.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        decide.diagnostic_id.as_deref(),
        Some("p2-notification-decide-no-session")
    );
    assert!(!format!("{decide:?}").contains("!r:example.org"));

    let focus = core
        .command(CommandEnvelope {
            command: "matrix_notification_focus_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "roomId": "!r:example.org" }),
        })
        .await
        .expect_err("notification focus without an attached owner must fail closed");
    assert_eq!(focus.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        focus.diagnostic_id.as_deref(),
        Some("p2-notification-focus-set-no-session")
    );

    let dismiss = core
        .command(CommandEnvelope {
            command: "matrix_notification_dismiss".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "candidateId": "notif-1" }),
        })
        .await
        .expect_err("notification dismiss without an attached owner must fail closed");
    assert_eq!(dismiss.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        dismiss.diagnostic_id.as_deref(),
        Some("p2-notification-dismiss-no-session")
    );

    let pending = core
        .command(CommandEnvelope {
            command: "matrix_notification_pending_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("notification pending snapshot without an attached owner must fail closed");
    assert_eq!(pending.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        pending.diagnostic_id.as_deref(),
        Some("p2-notification-pending-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_threepid_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_threepid_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("threepid snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-threepid-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_get_room_directory_visibility_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_get_room_directory_visibility".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","sessionGeneration":1}),
        })
        .await
        .expect_err("directory visibility get without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-get-room-directory-visibility-no-session")
    );
}

#[tokio::test]
async fn matrix_room_retention_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_retention".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","sessionGeneration":1}),
        })
        .await
        .expect_err("room retention without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-retention-no-session")
    );
}

#[tokio::test]
async fn matrix_media_preview_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_media_preview".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "sessionGeneration":1,
                "url":"https://example.org/x"
            }),
        })
        .await
        .expect_err("media preview without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-media-preview-no-session")
    );
}

#[tokio::test]
async fn matrix_set_room_directory_visibility_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_room_directory_visibility".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "sessionGeneration":1,
                "visibility":"public"
            }),
        })
        .await
        .expect_err("directory visibility set without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-room-directory-visibility-no-session")
    );
}

#[tokio::test]
async fn matrix_later_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_later_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("later snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-later-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_later_upsert_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_later_upsert".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "item": {
                    "id": "i",
                    "kind": "saved",
                    "roomId": "!r:example.org",
                    "eventId": "$e",
                    "createdAt": 1.0
                }
            }),
        })
        .await
        .expect_err("later upsert without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-later-upsert-no-session")
    );
}

#[tokio::test]
async fn matrix_room_notes_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_notes_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("room notes snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-notes-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_mdirect_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("mdirect snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-mdirect-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_mdirect_add_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_add".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org"
            }),
        })
        .await
        .expect_err("mdirect add without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-mdirect-add-no-session")
    );
}

#[tokio::test]
async fn matrix_mdirect_remove_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_mdirect_remove".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("mdirect remove without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-mdirect-remove-no-session")
    );
}

#[tokio::test]
async fn matrix_room_join_rule_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_join_rule_snapshot".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","sessionGeneration":1}),
        })
        .await
        .expect_err("join-rule snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-join-rule-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_join_rule_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_join_rule".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "joinRule":"invite"
            }),
        })
        .await
        .expect_err("join-rule write without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-join-rule-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("!r:example.org"));
}

#[tokio::test]
async fn matrix_room_leave_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_leave".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("room leave without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-leave-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_favorite_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_favorite".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","favorite":true}),
        })
        .await
        .expect_err("room set-favorite without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-favorite-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_read_state_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_read_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","action":"mark_read"}),
        })
        .await
        .expect_err("room set-read-state without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-read-state-no-session")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("!r:example.org"));
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_room_set_read_state_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_read_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "action":"mark_read",
                "token":"no"
            }),
        })
        .await
        .expect_err("room set-read-state must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-read-state-invalid-payload")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("!r:example.org"));
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_room_join_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_join".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomIdOrAlias":"!r:example.org"}),
        })
        .await
        .expect_err("room join without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-join-no-session")
    );
}

#[tokio::test]
async fn matrix_room_invite_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_invite".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org"
            }),
        })
        .await
        .expect_err("room invite without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-invite-no-session")
    );
}

#[tokio::test]
async fn matrix_room_kick_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_kick".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org"
            }),
        })
        .await
        .expect_err("room kick without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-kick-no-session")
    );
}

#[tokio::test]
async fn matrix_room_ban_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_ban".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org"
            }),
        })
        .await
        .expect_err("room ban without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-ban-no-session")
    );
}

#[tokio::test]
async fn matrix_room_unban_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_unban".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org"
            }),
        })
        .await
        .expect_err("room unban without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-unban-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_power_level_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_power_level".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "userId":"@alice:example.org",
                "powerLevel":50
            }),
        })
        .await
        .expect_err("set power level without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-power-level-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_power_levels_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_power_levels".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "content":{"users":{}}
            }),
        })
        .await
        .expect_err("set power levels without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-power-levels-no-session")
    );
}

#[tokio::test]
async fn matrix_room_set_power_level_tags_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_power_level_tags".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "content":{}
            }),
        })
        .await
        .expect_err("set power-level tags without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-power-level-tags-no-session")
    );
}

#[tokio::test]
async fn matrix_room_create_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_create".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "name":"Room",
                "encryption":false,
                "isDirect":false,
                "invite":[],
                "knock":false
            }),
        })
        .await
        .expect_err("room create without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-create-no-session")
    );
}

#[tokio::test]
async fn matrix_room_members_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_members_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("members snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-members-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_power_levels_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_power_levels_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("power-levels snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-power-levels-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_creators_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_creators_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("creators snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-creators-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_power_level_tags_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_power_level_tags_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("power-level tags snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-power-level-tags-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_room_join_rule_snapshot_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_join_rule_snapshot".into(),
            session_generation: 1,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "sessionGeneration":1,
                "token":"no"
            }),
        })
        .await
        .expect_err("join-rule snapshot must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-join-rule-snapshot-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_room_set_join_rule_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_room_set_join_rule".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "joinRule":"invite",
                "token":"no"
            }),
        })
        .await
        .expect_err("join-rule write must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-set-join-rule-invalid-payload")
    );
    let text = format!("{error:?}");
    assert!(!text.contains("!r:example.org"));
    assert!(!text.contains("token"));
}

#[tokio::test]
async fn matrix_get_global_image_packs_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_get_global_image_packs".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("global image packs without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-global-image-packs-no-session")
    );
}

#[tokio::test]
async fn matrix_get_user_image_pack_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_get_user_image_pack".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("user image pack without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-user-image-pack-no-session")
    );
}

#[tokio::test]
async fn matrix_get_room_image_packs_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_get_room_image_packs".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("room image packs without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-room-image-packs-no-session")
    );
}

#[tokio::test]
async fn matrix_set_user_image_pack_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_set_user_image_pack".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"content":{}}),
        })
        .await
        .expect_err("set user image pack without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-set-user-image-pack-no-session")
    );
}

#[tokio::test]
async fn matrix_typing_set_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_typing_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","typing":true}),
        })
        .await
        .expect_err("typing set without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-typing-set-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_close_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_close".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1"}),
        })
        .await
        .expect_err("timeline close without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-close-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_close_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_close".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1","token":"no"}),
        })
        .await
        .expect_err("timeline close must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-close-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_open_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_open".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","position":{"kind":"live_bottom"}}),
            })
            .await
            .expect_err("timeline open without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-open-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_open_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_open".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "position":{"kind":"live_bottom"},
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline open must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-open-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_jump_latest_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_jump_latest".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1"}),
        })
        .await
        .expect_err("timeline jump_latest without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-jump-latest-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1"}),
        })
        .await
        .expect_err("timeline snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_snapshot_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1","token":"no"}),
        })
        .await
        .expect_err("timeline snapshot must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-snapshot-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_event_readback_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_event_readback".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e"}),
        })
        .await
        .expect_err("timeline event readback without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-event-readback-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_event_readback_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_event_readback".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e",
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline event readback must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-event-readback-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_timestamp_to_event_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
            .command(CommandEnvelope {
                command: "matrix_timeline_timestamp_to_event".into(),
                session_generation: 0,
                request_id: None,
                payload: serde_json::json!({"roomId":"!r:example.org","timestampMs":1_700_000_000_000_u64}),
            })
            .await
            .expect_err("timeline timestamp_to_event without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-timestamp-to-event-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_timestamp_to_event_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_timestamp_to_event".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "timestampMs":1_700_000_000_000_u64,
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline timestamp_to_event must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-timestamp-to-event-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_paginate_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_paginate".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"streamId":"view-1","direction":"backwards"}),
        })
        .await
        .expect_err("timeline paginate without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-paginate-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_paginate_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_paginate".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "direction":"backwards",
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline paginate must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-paginate-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_set_read_state_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_set_read_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "action":"mark_read",
                "intent":"explicit_user"
            }),
        })
        .await
        .expect_err("timeline set_read_state without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-set-read-state-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_follow_live_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_follow_live".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "observedLiveTailEventId":"$e1"
            }),
        })
        .await
        .expect_err("timeline follow_live without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-follow-live-no-session")
    );
    assert!(!format!("{error:?}").contains("$e1"));
}

#[tokio::test]
async fn matrix_timeline_follow_live_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_follow_live".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "observedLiveTailEventId":"$e1",
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline follow_live must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-follow-live-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_set_read_state_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_set_read_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "action":"mark_read",
                "intent":"explicit_user",
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline set_read_state must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-set-read-state-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_set_read_state_requires_an_explicit_intent() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_set_read_state".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "streamId":"view-1",
                "action":"mark_read"
            }),
        })
        .await
        .expect_err("timeline set_read_state must distinguish automatic and explicit intent");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-set-read-state-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_reaction_toggle_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_reaction_toggle".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e","key":"✅"}),
        })
        .await
        .expect_err("reaction toggle without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-reaction-toggle-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_reaction_toggle_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_reaction_toggle".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e",
                "key":"✅",
                "token":"no"
            }),
        })
        .await
        .expect_err("reaction toggle must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-reaction-toggle-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_reaction_ensure_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_reaction_ensure".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org","eventId":"$e","key":"✅"}),
        })
        .await
        .expect_err("reaction ensure without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-reaction-ensure-no-session")
    );
}

#[tokio::test]
async fn matrix_reaction_redact_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_reaction_redact".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "targetEventId":"$e",
                "reactionEventId":"$r",
                "key":"✅"
            }),
        })
        .await
        .expect_err("reaction redact without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-reaction-redact-no-session")
    );
}

#[tokio::test]
async fn matrix_send_text_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_send_text".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "body":"hello"
            }),
        })
        .await
        .expect_err("send text without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-send-text-no-session")
    );
}

#[test]
fn outbound_payload_and_mention_bounds_are_sdk_invariants() {
    for diagnostic in [
        "d0.4-send-text-payload-too-large",
        "v-send.4-mention-user-id-too-long",
        "v-send.4-too-many-mentions",
    ] {
        assert_eq!(
            send_text_owner_error(diagnostic).category,
            MatrixIpcErrorCategory::SdkInvariant,
            "{diagnostic}"
        );
        assert_eq!(
            send_room_attachment_owner_error(diagnostic).category,
            MatrixIpcErrorCategory::SdkInvariant,
            "{diagnostic}"
        );
    }
}

#[tokio::test]
async fn matrix_send_poll_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_send_poll".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "question":"Q?",
                "answers":["A","B"],
                "maxSelections":1
            }),
        })
        .await
        .expect_err("send poll without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-send-poll-no-session")
    );
}

#[tokio::test]
async fn matrix_poll_respond_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_poll_respond".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "pollEventId":"$e:example.org",
                "answerIds":["a1"]
            }),
        })
        .await
        .expect_err("poll respond without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-poll-respond-no-session")
    );
}

#[tokio::test]
async fn matrix_space_parents_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_space_parents_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("space parents snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-space-parents-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_space_hierarchy_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_space_hierarchy_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!space:example.org"}),
        })
        .await
        .expect_err("space hierarchy snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-space-hierarchy-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_space_children_snapshot_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_space_children_snapshot".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .expect_err("space children snapshot without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-space-children-snapshot-no-session")
    );
}

#[tokio::test]
async fn matrix_space_child_set_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_space_child_set".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "parentId":"!space:example.org",
                "childId":"!room:example.org",
                "via":["example.org"]
            }),
        })
        .await
        .expect_err("space child set without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-space-child-set-no-session")
    );
}

#[tokio::test]
async fn matrix_space_child_remove_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_space_child_remove".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "parentId":"!space:example.org",
                "childId":"!room:example.org"
            }),
        })
        .await
        .expect_err("space child remove without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-space-child-remove-no-session")
    );
}

#[tokio::test]
async fn matrix_restricted_join_reparent_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_restricted_join_reparent".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!room:example.org",
                "addParentId":"!space:example.org"
            }),
        })
        .await
        .expect_err("restricted join reparent without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-restricted-join-reparent-no-session")
    );
}

#[tokio::test]
async fn matrix_edit_message_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_edit_message".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e:example.org",
                "body":"hello"
            }),
        })
        .await
        .expect_err("edit message without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-edit-message-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_edit_text_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_edit_text".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e",
                "body":"updated"
            }),
        })
        .await
        .expect_err("timeline edit without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-edit-text-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_edit_text_rejects_unknown_payload_fields() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_edit_text".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e",
                "body":"updated",
                "token":"no"
            }),
        })
        .await
        .expect_err("timeline edit must reject unknown payload fields");
    assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-edit-text-invalid-payload")
    );
}

#[tokio::test]
async fn matrix_timeline_redact_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_redact".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("timeline redact without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-redact-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_report_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_report".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("timeline report without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-report-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_pin_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_pin".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("timeline pin without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-pin-no-session")
    );
}

#[tokio::test]
async fn matrix_pinned_events_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_pinned_events".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org"
            }),
        })
        .await
        .expect_err("pinned events without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-pinned-events-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_unpin_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_unpin".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("timeline unpin without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-unpin-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_poll_vote_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_poll_vote".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e",
                "answerIds":["yes"]
            }),
        })
        .await
        .expect_err("timeline poll vote without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-poll-vote-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_call_decline_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_call_decline".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("timeline call decline without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-call-decline-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_forward_text_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_forward_text".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "sourceRoomId":"!s:example.org",
                "eventId":"$e",
                "targetRoomId":"!t:example.org",
                "confirmedEncryptionDowngrade":false
            }),
        })
        .await
        .expect_err("timeline forward text without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-forward-text-no-session")
    );
}

#[tokio::test]
async fn matrix_timeline_forward_media_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_timeline_forward_media".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "sourceRoomId":"!s:example.org",
                "eventId":"$e",
                "targetRoomId":"!t:example.org",
                "confirmedEncryptionDowngrade":false
            }),
        })
        .await
        .expect_err("timeline forward media without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-timeline-forward-media-no-session")
    );
}

#[tokio::test]
async fn matrix_composer_set_reply_draft_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_composer_set_reply_draft".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "eventId":"$e"
            }),
        })
        .await
        .expect_err("composer set reply draft without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-composer-set-reply-draft-no-session")
    );
}

#[tokio::test]
async fn matrix_composer_clear_reply_draft_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_composer_clear_reply_draft".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "expectedDraftRevision":1
            }),
        })
        .await
        .expect_err("composer clear reply draft without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-composer-clear-reply-draft-no-session")
    );
}

#[tokio::test]
async fn matrix_composer_get_reply_draft_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_composer_get_reply_draft".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({"roomId":"!r:example.org"}),
        })
        .await
        .expect_err("composer get reply draft without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-composer-get-reply-draft-no-session")
    );
}

#[tokio::test]
async fn matrix_thread_list_without_owner_fails_closed() {
    let core = Core::new(Arc::new(TestPlatform));
    let error = core
        .command(CommandEnvelope {
            command: "matrix_thread_list".into(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({
                "roomId":"!r:example.org",
                "action":"open"
            }),
        })
        .await
        .expect_err("thread list without an attached owner must fail closed");
    assert_eq!(error.category, MatrixIpcErrorCategory::Forbidden);
    assert_eq!(
        error.diagnostic_id.as_deref(),
        Some("p2-thread-list-no-session")
    );
}

#[tokio::test]
async fn known_but_unregistered_commands_fail_closed_with_static_diagnostic() {
    let core = Core::new(Arc::new(TestPlatform));
    for command in [
        "matrix_login_password",
        "matrix_register",
        "matrix_register_request_email_token",
        "matrix_sync_recover",
    ] {
        let error = core
            .command(CommandEnvelope {
                command: command.into(),
                session_generation: 1,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .expect_err("known but unregistered command must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::SdkInvariant);
        assert_eq!(
            error.diagnostic_id.as_deref(),
            Some("p2-command-unregistered")
        );
    }
}
