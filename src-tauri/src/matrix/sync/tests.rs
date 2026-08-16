//! Unit tests for P4.1 sync readiness / reconnect foundation.

use super::*;
use crate::matrix::diagnostics::SyncPhase;
use crate::matrix::ipc::MatrixIpcErrorCategory;
use matrix_sdk_ui::sync_service::State as SdkSyncState;
use std::sync::Arc;

#[test]
fn marker_stable() {
    assert_eq!(matrix_sync_markers(), MATRIX_SYNC_MARKER);
}

#[test]
fn readiness_labels_cover_all() {
    assert_eq!(SyncReadiness::ALL.len(), 6);
    for r in SyncReadiness::ALL {
        assert!(!r.as_str().is_empty());
    }
}

#[test]
fn product_ready_only_when_running() {
    assert!(!SyncReadiness::Unconfigured.is_product_ready());
    assert!(!SyncReadiness::Idle.is_product_ready());
    assert!(SyncReadiness::Running.is_product_ready());
    assert!(!SyncReadiness::Offline.is_product_ready());
    assert!(!SyncReadiness::Terminated.is_product_ready());
    assert!(!SyncReadiness::Failed.is_product_ready());
    assert!(SyncReadiness::Running.allows_live_projections());
    assert!(!SyncReadiness::Offline.allows_live_projections());
}

#[test]
fn readiness_maps_to_diagnostics_sync_phase() {
    assert_eq!(SyncReadiness::Unconfigured.to_sync_phase(), SyncPhase::Idle);
    assert_eq!(SyncReadiness::Idle.to_sync_phase(), SyncPhase::Idle);
    assert_eq!(SyncReadiness::Running.to_sync_phase(), SyncPhase::Live);
    assert_eq!(
        SyncReadiness::Offline.to_sync_phase(),
        SyncPhase::Reconnecting
    );
    assert_eq!(SyncReadiness::Terminated.to_sync_phase(), SyncPhase::Idle);
    assert_eq!(SyncReadiness::Failed.to_sync_phase(), SyncPhase::Failed);
}

#[test]
fn sdk_state_mapping_is_privacy_safe() {
    assert_eq!(
        readiness_from_sdk_state(&SdkSyncState::Idle),
        SyncReadiness::Idle
    );
    assert_eq!(
        readiness_from_sdk_state(&SdkSyncState::Running),
        SyncReadiness::Running
    );
    assert_eq!(
        readiness_from_sdk_state(&SdkSyncState::Terminated),
        SyncReadiness::Terminated
    );
    assert_eq!(
        readiness_from_sdk_state(&SdkSyncState::Offline),
        SyncReadiness::Offline
    );

    // Error arm: use a real Error value via Arc without exporting message.
    let err = matrix_sdk_ui::sync_service::Error::Supervisor;
    let state = SdkSyncState::Error(Arc::new(err));
    assert_eq!(readiness_from_sdk_state(&state), SyncReadiness::Failed);
    assert_eq!(
        failure_diagnostic_from_sdk_state(&state),
        Some("p4.1-sync-service-error")
    );

    let snap = snapshot_from_sdk_state(&state, 7, true);
    assert_eq!(snap.session_generation, 7);
    assert!(snap.offline_mode_enabled);
    assert_eq!(snap.readiness, SyncReadiness::Failed);
    assert_eq!(snap.failure_diagnostic_id, Some("p4.1-sync-service-error"));
    assert!(!snap.is_product_ready());
    // Snapshot Debug/Display path must not include token-like material.
    let rendered = format!("{snap:?}");
    assert!(!rendered.to_ascii_lowercase().contains("access_token"));
    assert!(!rendered.contains("syt_"));
}

#[test]
fn reconnect_bootstrap_and_recover_table() {
    // Bootstrap
    assert_eq!(
        decide_reconnect(SyncReadiness::Unconfigured, SyncIntent::Bootstrap),
        ReconnectAction::None
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Idle, SyncIntent::Bootstrap),
        ReconnectAction::Start
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Running, SyncIntent::Bootstrap),
        ReconnectAction::None
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Failed, SyncIntent::Bootstrap),
        ReconnectAction::Start
    );

    // Recover
    assert_eq!(
        decide_reconnect(SyncReadiness::Failed, SyncIntent::Recover),
        ReconnectAction::Restart
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Offline, SyncIntent::Recover),
        ReconnectAction::Start
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Running, SyncIntent::Recover),
        ReconnectAction::None
    );

    // Pause / shutdown
    assert_eq!(
        decide_reconnect(SyncReadiness::Running, SyncIntent::Pause),
        ReconnectAction::Stop
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Offline, SyncIntent::Shutdown),
        ReconnectAction::Stop
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Idle, SyncIntent::Shutdown),
        ReconnectAction::None
    );
    assert_eq!(
        decide_reconnect(SyncReadiness::Running, SyncIntent::Observe),
        ReconnectAction::None
    );
}

#[test]
fn restartable_requires_configured_service() {
    assert!(!is_restartable(SyncReadiness::Unconfigured));
    assert!(is_restartable(SyncReadiness::Idle));
    assert!(is_restartable(SyncReadiness::Failed));
}

#[test]
fn unconfigured_snapshot_defaults() {
    let snap = unconfigured_snapshot(3);
    assert_eq!(snap.readiness, SyncReadiness::Unconfigured);
    assert_eq!(snap.session_generation, 3);
    assert!(!snap.offline_mode_enabled);
    assert!(snap.failure_diagnostic_id.is_none());
    assert_eq!(readiness_of(None), SyncReadiness::Unconfigured);
}

#[test]
fn sync_error_categories_are_stable() {
    let e = SyncError::NotAuthenticated {
        diagnostic_id: "p4.1-sync-requires-session",
    };
    assert_eq!(e.diagnostic_id(), "p4.1-sync-requires-session");
    assert_eq!(e.category(), MatrixIpcErrorCategory::AuthenticationRejected);
    let display = e.to_string();
    assert!(display.contains("p4.1-sync-requires-session"));
    assert!(!display.contains("access_token"));

    let stale = SyncError::StaleGeneration {
        diagnostic_id: "p4.1-stale-sync-generation",
        expected: 2,
        observed: 1,
    };
    assert_eq!(
        stale.category(),
        MatrixIpcErrorCategory::StaleSessionGeneration
    );
}

#[test]
fn default_config_enables_offline_mode() {
    let cfg = SyncServiceConfig::default();
    assert!(cfg.offline_mode);
    let off = SyncServiceConfig {
        offline_mode: false,
    };
    assert!(!off.offline_mode);
}

#[tokio::test(flavor = "multi_thread")]
async fn build_refuses_unauthenticated_client() {
    use crate::matrix::client_builder::{build_unauthenticated_client, ClientBuildConfig};
    use crate::matrix::store::{AccountIdentity, StoreKeyMaterial};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let root: PathBuf = std::env::temp_dir().join(format!(
        "synara-p4.1-unauth-{}-{}",
        std::process::id(),
        nanos
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let identity =
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap();
    let key = StoreKeyMaterial::generate().unwrap();
    let config = ClientBuildConfig::product_default(&root, identity, Some(key)).unwrap();
    let client = build_unauthenticated_client(&config)
        .await
        .expect("unauthenticated client");

    let result = build_sync_service(&client, 1, SyncServiceConfig::default()).await;
    let err = match result {
        Ok(_) => panic!("must refuse unauthenticated client"),
        Err(e) => e,
    };
    assert_eq!(err.diagnostic_id(), "p4.1-sync-requires-session");
    assert_eq!(
        err.category(),
        MatrixIpcErrorCategory::AuthenticationRejected
    );

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn assert_generation_detects_stale_owner_without_service() {
    // Pure stale check uses owner generation only; construct via unconfigured path
    // is not available without a service. Unit-test the error shape instead.
    let err = SyncError::StaleGeneration {
        diagnostic_id: "p4.1-stale-sync-generation",
        expected: 5,
        observed: 4,
    };
    assert_eq!(err.diagnostic_id(), "p4.1-stale-sync-generation");
    let msg = format!("{err}");
    assert!(msg.contains("expected 5"));
    assert!(msg.contains("observed 4"));
}

/// SNC-P1-5a: `capability.rs` moved to `synara_core::app::sync`; these mirror
/// its unit tests against the src-tauri re-export so the desktop test count
/// (and sliding-sync capability coverage) stays identical to the pre-move
/// baseline — the same shape SNC-P1-4 used to keep its desktop suite intact.
#[cfg(test)]
mod capability_mirror {
    use super::server_supports_sliding_sync;
    use std::collections::BTreeMap;

    fn versions(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }
    fn unstable(pairs: &[(&str, bool)]) -> BTreeMap<String, bool> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn empty_server_response_is_not_supported() {
        assert!(!server_supports_sliding_sync(&[], &BTreeMap::new()));
    }

    #[test]
    fn unstable_feature_marker_enables_support() {
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.msc3575", true)])
        ));
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.msc4186", true)])
        ));
        // matrix-sdk 0.18's native SyncService uses
        // /unstable/org.matrix.simplified_msc3575/sync (MSC4186).
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.simplified_msc3575", true)])
        ));
    }

    #[test]
    fn versions_list_marker_enables_support() {
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11", "org.matrix.msc3575"]),
            &BTreeMap::new()
        ));
    }

    #[test]
    fn absent_or_false_markers_are_not_supported() {
        assert!(!server_supports_sliding_sync(
            &versions(&["v1.11", "v1.12"]),
            &unstable(&[("org.matrix.msc3575", false), ("org.matrix.thing", true)])
        ));
    }
}
