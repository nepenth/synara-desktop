//! Credential-free V-CRYPTO.7 device presentation plus live device-list owner.
//!
//! Shells supply the emit sink (desktop Tauri event / later iOS UniFFI).
//! Password UIAA continuation stays in the desktop shell so the password
//! never crosses `Core::command`. Start/cancel route through the owner.

use serde::{Deserialize, Serialize};

mod live;
pub use live::{
    snapshot, supported_delete_authentication, DeviceListUpdateEmit, NativeDeviceOwner,
    NativeDeviceUpdateSignal, PendingDeviceDeletion,
};

/// Tauri event: device list may have changed; UI re-snapshots via matrix_get_* commands.
/// Signal only — never carries device keys or tokens.
pub const DEVICE_LIST_UPDATED_EVENT: &str = "matrix-device-list-updated";

/// Per-session crypto trust. `verified` is cross-signing trust only.
/// Direct SAS without cross-signing is `verified_locally_only`. The previous
/// `unsupported` wire value deserializes as `no_encryption`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceTrust {
    Verified,
    VerifiedLocallyOnly,
    Unverified,
    #[serde(alias = "unsupported")]
    NoEncryption,
    Dehydrated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeDeviceTrustSignals {
    pub has_crypto_device: bool,
    pub is_dehydrated: bool,
    pub is_verified_with_cross_signing: bool,
    pub is_verified: bool,
}

/// Password multi-select logout is `/devices` DELETE. That destroys the
/// MSC3814 catcher's queued to-device keys, so backup rows stay out of the
/// selection. Current-device rows are also excluded.
pub fn device_eligible_for_password_logout(device: &NativeDeviceSummary) -> bool {
    !device.is_current && device.trust != NativeDeviceTrust::Dehydrated
}

/// Map SDK crypto flags onto the product trust vocabulary.
pub fn project_native_device_trust(signals: NativeDeviceTrustSignals) -> NativeDeviceTrust {
    if !signals.has_crypto_device {
        return NativeDeviceTrust::NoEncryption;
    }
    if signals.is_dehydrated {
        return NativeDeviceTrust::Dehydrated;
    }
    if signals.is_verified_with_cross_signing {
        return NativeDeviceTrust::Verified;
    }
    if signals.is_verified {
        return NativeDeviceTrust::VerifiedLocallyOnly;
    }
    NativeDeviceTrust::Unverified
}

/// Homeserver display names and last-seen IPs are untrusted. Drop empty or
/// oversized values rather than copying them onto the snapshot wire.
pub const MAX_DEVICE_DISPLAY_NAME_CHARS: usize = 256;
pub const MAX_DEVICE_LAST_SEEN_IP_CHARS: usize = 64;

pub fn bounded_optional_hs_text(value: Option<String>, max_chars: usize) -> Option<String> {
    let trimmed = value?.trim().to_owned();
    if trimmed.is_empty() || trimmed.chars().count() > max_chars {
        None
    } else {
        Some(trimmed)
    }
}

/// Element-style ed25519 fingerprint: unpadded base64 in 4-character groups.
/// Rejects anything that is not a 32-byte ed25519 key encoding.
pub fn format_ed25519_fingerprint(unpadded_base64: &str) -> Option<String> {
    const ED25519_UNPADDED_BASE64_LEN: usize = 43;
    let trimmed = unpadded_base64.trim();
    if trimmed.len() != ED25519_UNPADDED_BASE64_LEN
        || !trimmed
            .bytes()
            .all(|byte| matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/'))
    {
        return None;
    }
    Some(
        trimmed
            .as_bytes()
            .chunks(4)
            .map(|chunk| std::str::from_utf8(chunk).expect("ascii base64"))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceSummary {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: NativeDeviceTrust,
    pub is_current: bool,
    #[serde(default)]
    pub is_cross_signed_by_owner: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_seen_ts: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ed25519_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceSnapshot {
    pub session_generation: u64,
    /// Matrix SDK's authoritative cross-signing state for this exact device.
    ///
    /// This is intentionally separate from the current row's local trust. A
    /// direct SAS can make the peer locally trusted without the user's
    /// cross-signing identity signing this device.
    pub own_verification: NativeOwnDeviceVerification,
    /// Whether the SDK found an eligible, cross-signed peer authority for
    /// verifying this device. When the authority `/keys/query` fails or times
    /// out, this projects the same SDK predicate over the concurrently
    /// fetched local device set instead of collapsing to unknown. `None` is
    /// reserved for a missing crypto machine or a failed local device fetch,
    /// where there is no set to project. This is not inferred from local row
    /// trust.
    pub has_devices_to_verify_against: Option<bool>,
    pub devices: Vec<NativeDeviceSummary>,
}

impl NativeDeviceSnapshot {
    pub fn contains(&self, device_id: &str) -> bool {
        self.devices
            .iter()
            .any(|device| device.device_id == device_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeOwnDeviceVerification {
    Unknown,
    Unverified,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDeviceDeleteAuthentication {
    Password,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeDeviceDeleteChallenge {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: NativeDeviceDeleteAuthentication,
    pub authentication_failed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum NativeDeviceDeleteResult {
    Complete {
        snapshot: NativeDeviceSnapshot,
    },
    AuthenticationRequired {
        challenge: NativeDeviceDeleteChallenge,
    },
}

/// Current first; other devices retain product parity by most-recent
/// activity descending, with a deterministic ID tiebreaker.
pub fn sort_native_device_summaries(devices: &mut [NativeDeviceSummary]) {
    devices.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| right.last_seen_ts.cmp(&left.last_seen_ts))
            .then_with(|| left.device_id.cmp(&right.device_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_projection_contains_presentation_fields_but_no_device_keys() {
        let snapshot = NativeDeviceSnapshot {
            session_generation: 7,
            own_verification: NativeOwnDeviceVerification::Verified,
            has_devices_to_verify_against: Some(true),
            devices: vec![NativeDeviceSummary {
                device_id: "DEVICE".into(),
                display_name: Some("Synara macOS".into()),
                last_seen_ip: Some("192.0.2.1".into()),
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Verified,
                is_current: true,
                is_cross_signed_by_owner: true,
                first_seen_ts: Some(1),
                ed25519_fingerprint: Some("ABCD EFGH IJKL MNOP QRST UVWX YZab cde".into()),
            }],
        };
        let json = serde_json::to_string(&snapshot)
            .unwrap()
            .to_ascii_lowercase();
        assert!(json.contains("lastseenip"));
        assert!(json.contains("ed25519fingerprint"));
        assert!(json.contains("abcd efgh ijkl"));
        assert!(json.contains("iscrosssignedbyowner"));
        assert!(json.contains("firstseents"));
        for forbidden in [
            "access_token",
            "refresh_token",
            "device_key",
            "curve25519",
            "password",
            "auth_session",
        ] {
            assert!(!json.contains(forbidden));
        }

        let challenge = NativeDeviceDeleteChallenge {
            operation_id: 3,
            session_generation: 7,
            authentication: NativeDeviceDeleteAuthentication::Password,
            authentication_failed: false,
        };
        let challenge_json = serde_json::to_string(&challenge).unwrap();
        assert!(challenge_json.contains(r#""authentication":"password""#));
        assert!(!challenge_json.contains(r#""authentication":["#));
    }

    #[test]
    fn verification_authority_query_failure_remains_unknown() {
        let snapshot = NativeDeviceSnapshot {
            session_generation: 1,
            own_verification: NativeOwnDeviceVerification::Unknown,
            has_devices_to_verify_against: None,
            devices: Vec::new(),
        };
        let value = serde_json::to_value(snapshot).expect("serialize unknown authority");
        assert!(value["hasDevicesToVerifyAgainst"].is_null());
    }

    #[test]
    fn authority_query_is_recoverable_metadata_not_a_snapshot_failure() {
        let source = include_str!("live.rs");
        let snapshot = source
            .split("pub async fn snapshot(\n")
            .nth(1)
            .and_then(|rest| rest.split("pub fn supported_delete_authentication").next())
            .expect("device snapshot owner");
        assert!(snapshot.contains("has_devices_to_verify_against()"));
        assert!(snapshot.contains("Duration::from_secs(8)"));
        assert!(snapshot.contains("Ok(Err(_)) | Err(_) => match crypto_devices.as_ref()"));
        assert!(snapshot.contains("eligible_local_authority(Some(devices))"));
        assert!(snapshot.contains("Err(matrix_sdk::Error::NoOlmMachine) | Err(_) => None"));
        assert!(!snapshot.contains("device-verification-state-failed"));
    }

    #[test]
    fn sort_puts_current_first_then_recent_then_id() {
        let mut devices = vec![
            NativeDeviceSummary {
                device_id: "B".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Unverified,
                is_current: false,
                is_cross_signed_by_owner: false,
                first_seen_ts: None,
                ed25519_fingerprint: None,
            },
            NativeDeviceSummary {
                device_id: "A".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(1),
                trust: NativeDeviceTrust::Unverified,
                is_current: false,
                is_cross_signed_by_owner: false,
                first_seen_ts: None,
                ed25519_fingerprint: None,
            },
            NativeDeviceSummary {
                device_id: "CUR".into(),
                display_name: None,
                last_seen_ip: None,
                last_seen_ts: Some(0),
                trust: NativeDeviceTrust::Verified,
                is_current: true,
                is_cross_signed_by_owner: true,
                first_seen_ts: None,
                ed25519_fingerprint: None,
            },
        ];
        sort_native_device_summaries(&mut devices);
        assert_eq!(devices[0].device_id, "CUR");
        assert_eq!(devices[1].device_id, "A");
        assert_eq!(devices[2].device_id, "B");
        assert!(NativeDeviceSnapshot {
            session_generation: 1,
            own_verification: NativeOwnDeviceVerification::Unverified,
            has_devices_to_verify_against: Some(false),
            devices,
        }
        .contains("CUR"));
    }

    #[test]
    fn trust_projection_distinguishes_cross_signing_local_sas_and_missing_crypto() {
        assert_eq!(
            project_native_device_trust(NativeDeviceTrustSignals {
                has_crypto_device: false,
                is_dehydrated: false,
                is_verified_with_cross_signing: false,
                is_verified: false,
            }),
            NativeDeviceTrust::NoEncryption
        );
        assert_eq!(
            project_native_device_trust(NativeDeviceTrustSignals {
                has_crypto_device: true,
                is_dehydrated: true,
                is_verified_with_cross_signing: false,
                is_verified: false,
            }),
            NativeDeviceTrust::Dehydrated
        );
        assert_eq!(
            project_native_device_trust(NativeDeviceTrustSignals {
                has_crypto_device: true,
                is_dehydrated: false,
                is_verified_with_cross_signing: true,
                is_verified: true,
            }),
            NativeDeviceTrust::Verified
        );
        assert_eq!(
            project_native_device_trust(NativeDeviceTrustSignals {
                has_crypto_device: true,
                is_dehydrated: false,
                is_verified_with_cross_signing: false,
                is_verified: true,
            }),
            NativeDeviceTrust::VerifiedLocallyOnly
        );
        assert_eq!(
            project_native_device_trust(NativeDeviceTrustSignals {
                has_crypto_device: true,
                is_dehydrated: false,
                is_verified_with_cross_signing: false,
                is_verified: false,
            }),
            NativeDeviceTrust::Unverified
        );
        let backup = NativeDeviceSummary {
            device_id: "BACKUP".into(),
            display_name: None,
            last_seen_ip: None,
            last_seen_ts: None,
            trust: NativeDeviceTrust::Dehydrated,
            is_current: false,
            is_cross_signed_by_owner: false,
            first_seen_ts: None,
            ed25519_fingerprint: None,
        };
        let other = NativeDeviceSummary {
            device_id: "PHONE".into(),
            display_name: None,
            last_seen_ip: None,
            last_seen_ts: None,
            trust: NativeDeviceTrust::Unverified,
            is_current: false,
            is_cross_signed_by_owner: false,
            first_seen_ts: None,
            ed25519_fingerprint: None,
        };
        let current = NativeDeviceSummary {
            device_id: "CUR".into(),
            display_name: None,
            last_seen_ip: None,
            last_seen_ts: None,
            trust: NativeDeviceTrust::Verified,
            is_current: true,
            is_cross_signed_by_owner: true,
            first_seen_ts: None,
            ed25519_fingerprint: None,
        };
        assert!(!device_eligible_for_password_logout(&backup));
        assert!(!device_eligible_for_password_logout(&current));
        assert!(device_eligible_for_password_logout(&other));

        let decoded: NativeDeviceTrust = serde_json::from_str("\"unsupported\"").unwrap();
        assert_eq!(decoded, NativeDeviceTrust::NoEncryption);
        assert_eq!(
            serde_json::to_string(&NativeDeviceTrust::NoEncryption).unwrap(),
            "\"no_encryption\""
        );
        assert_eq!(
            serde_json::to_string(&NativeDeviceTrust::VerifiedLocallyOnly).unwrap(),
            "\"verified_locally_only\""
        );
    }

    #[test]
    fn ed25519_fingerprint_is_grouped_in_fours() {
        assert_eq!(
            format_ed25519_fingerprint("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopq").as_deref(),
            Some("ABCD EFGH IJKL MNOP QRST UVWX YZab cdef ghij klmn opq")
        );
        assert_eq!(format_ed25519_fingerprint("  ").as_deref(), None);
        assert_eq!(
            format_ed25519_fingerprint(&"A".repeat(1_024)).as_deref(),
            None
        );
        assert_eq!(format_ed25519_fingerprint(&"é".repeat(43)).as_deref(), None);
        assert_eq!(
            format_ed25519_fingerprint("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij=").as_deref(),
            None
        );
        let value: NativeDeviceSnapshot = serde_json::from_value(serde_json::json!({
            "sessionGeneration": 1,
            "ownVerification": "unverified",
            "hasDevicesToVerifyAgainst": false,
            "devices": [{
                "deviceId": "OLD",
                "trust": "unsupported",
                "isCurrent": false
            }]
        }))
        .expect("older snapshots without additive fields remain readable");
        assert_eq!(value.devices[0].trust, NativeDeviceTrust::NoEncryption);
        assert!(!value.devices[0].is_cross_signed_by_owner);
        assert_eq!(value.devices[0].ed25519_fingerprint, None);
    }

    #[test]
    fn homeserver_display_name_and_ip_are_dropped_when_empty_or_oversized() {
        assert_eq!(
            bounded_optional_hs_text(Some("  MacBook  ".into()), MAX_DEVICE_DISPLAY_NAME_CHARS)
                .as_deref(),
            Some("MacBook")
        );
        assert_eq!(
            bounded_optional_hs_text(Some("   ".into()), MAX_DEVICE_DISPLAY_NAME_CHARS),
            None
        );
        assert_eq!(
            bounded_optional_hs_text(
                Some("n".repeat(MAX_DEVICE_DISPLAY_NAME_CHARS + 1)),
                MAX_DEVICE_DISPLAY_NAME_CHARS
            ),
            None
        );
        assert_eq!(
            bounded_optional_hs_text(Some("1".repeat(80)), MAX_DEVICE_LAST_SEEN_IP_CHARS),
            None
        );
        assert_eq!(
            bounded_optional_hs_text(Some("192.0.2.1".into()), MAX_DEVICE_LAST_SEEN_IP_CHARS)
                .as_deref(),
            Some("192.0.2.1")
        );
    }
}
