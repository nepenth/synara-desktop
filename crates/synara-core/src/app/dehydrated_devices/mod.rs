//! MSC3814 dehydrated-device manager (backup Olm catcher).
//!
//! Attached with the session owner set. Start requires an unlocked Secret
//! Storage handle still in host memory; probe failures stay silent. Status
//! DTOs are privacy-safe: no pickle key, recovery key, or to-device
//! ciphertext. Ordinary logout calls `stop()`, not MSC3814 `delete()`.

use serde::{Deserialize, Serialize};

mod live;
pub use live::{start_with_secret, NativeDehydratedDevicesOwner, NativeDehydratedStartOutcome};

use crate::app::devices::MAX_DEVICE_DISPLAY_NAME_CHARS;

/// Diagnostic ids for start failures. Never include SDK error text.
pub const START_UNSUPPORTED: &str = "v-crypto.msc3814-unsupported";
pub const START_SECRET_EMPTY: &str = "v-crypto.msc3814-secret-empty";
pub const START_SECRET_STORE_UNAVAILABLE: &str = "v-crypto.msc3814-secret-store-unavailable";
pub const START_FAILED: &str = "v-crypto.msc3814-start-failed";

/// Privacy-safe lifecycle kind. SDK error strings stay off this wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDehydratedDeviceEventKind {
    Created,
    Uploaded,
    Deleted,
    KeyCached,
    RehydrationStarted,
    RehydrationProgress,
    RehydrationCompleted,
    RehydrationError,
    RotationError,
    Lagged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDehydratedDevicesStatus {
    pub session_generation: u64,
    pub supported: bool,
    pub active: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_kind: Option<NativeDehydratedDeviceEventKind>,
}

pub fn project_dehydrated_devices_status(
    session_generation: u64,
    supported: bool,
    active: bool,
    last_device_id: Option<String>,
    last_event_kind: Option<NativeDehydratedDeviceEventKind>,
) -> NativeDehydratedDevicesStatus {
    NativeDehydratedDevicesStatus {
        session_generation,
        supported,
        active,
        last_device_id: last_device_id.and_then(|device_id| {
            crate::app::devices::bounded_optional_hs_text(
                Some(device_id),
                MAX_DEVICE_DISPLAY_NAME_CHARS,
            )
        }),
        last_event_kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_projection_never_serializes_secret_material() {
        let status = project_dehydrated_devices_status(
            4,
            true,
            true,
            Some("DEHYDRATEDDEV".into()),
            Some(NativeDehydratedDeviceEventKind::Uploaded),
        );
        let json = serde_json::to_string(&status).unwrap().to_ascii_lowercase();
        assert!(json.contains("dehydrateddev"));
        assert_eq!(
            status.last_event_kind,
            Some(NativeDehydratedDeviceEventKind::Uploaded)
        );
        for forbidden in [
            "pickle",
            "recovery_key",
            "recoverykey",
            "private_key",
            "privatekey",
            "ciphertext",
            "passphrase",
            "org.matrix.msc3814",
            "to_device",
            "error\":",
        ] {
            assert!(
                !json.contains(forbidden),
                "status must not serialize {forbidden}: {json}"
            );
        }
    }

    #[test]
    fn status_drops_oversized_device_ids() {
        let status = project_dehydrated_devices_status(
            1,
            true,
            false,
            Some("D".repeat(MAX_DEVICE_DISPLAY_NAME_CHARS + 1)),
            Some(NativeDehydratedDeviceEventKind::Created),
        );
        assert_eq!(status.last_device_id, None);
        assert!(!status.active);
    }

    #[test]
    fn event_kinds_are_closed_vocabulary() {
        let kind = NativeDehydratedDeviceEventKind::RehydrationError;
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            "\"rehydration_error\""
        );
        let rotation = NativeDehydratedDeviceEventKind::RotationError;
        assert_eq!(
            serde_json::to_string(&rotation).unwrap(),
            "\"rotation_error\""
        );
    }
}
