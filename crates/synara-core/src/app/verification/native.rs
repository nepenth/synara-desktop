//! Credential-free V-CRYPTO.1 verification presentation DTOs.
//!
//! Live Client request/SAS ownership stays in the desktop shell.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeVerificationDirection {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeVerificationPhase {
    Requested,
    Ready,
    Started,
    SasReady,
    Confirmed,
    Done,
    Mismatched,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVerificationEmoji {
    pub symbol: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVerificationSas {
    pub emoji: Option<Vec<NativeVerificationEmoji>>,
    pub decimals: Option<[u16; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVerificationRequest {
    pub flow_id: String,
    pub other_user_id: String,
    pub other_device_id: Option<String>,
    pub direction: NativeVerificationDirection,
    pub phase: NativeVerificationPhase,
    pub started_ts: Option<u64>,
    pub sas: Option<NativeVerificationSas>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeVerificationInbox {
    pub session_generation: u64,
    pub requests: Vec<NativeVerificationRequest>,
}

pub fn phase_rank(phase: NativeVerificationPhase) -> u8 {
    match phase {
        NativeVerificationPhase::Requested => 0,
        NativeVerificationPhase::Ready => 1,
        NativeVerificationPhase::Started => 2,
        NativeVerificationPhase::SasReady => 3,
        NativeVerificationPhase::Confirmed => 4,
        NativeVerificationPhase::Done => 5,
        NativeVerificationPhase::Mismatched => 6,
        NativeVerificationPhase::Cancelled => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_order_keeps_actionable_requests_first() {
        assert!(
            phase_rank(NativeVerificationPhase::Requested)
                < phase_rank(NativeVerificationPhase::SasReady)
        );
        assert!(
            phase_rank(NativeVerificationPhase::SasReady)
                < phase_rank(NativeVerificationPhase::Done)
        );
    }

    #[test]
    fn projected_types_serialize_without_crypto_fields() {
        let request = NativeVerificationRequest {
            flow_id: "flow".to_owned(),
            other_user_id: "@alice:example.org".to_owned(),
            other_device_id: Some("DEVICE".to_owned()),
            direction: NativeVerificationDirection::Incoming,
            phase: NativeVerificationPhase::SasReady,
            started_ts: Some(1),
            sas: Some(NativeVerificationSas {
                emoji: Some(vec![NativeVerificationEmoji {
                    symbol: "🐶".to_owned(),
                    description: "Dog".to_owned(),
                }]),
                decimals: Some([1234, 5678, 9012]),
            }),
        };
        let value = serde_json::to_value(request).expect("serialize projection");
        assert_eq!(value["phase"], "sas_ready");
        let serialized = value.to_string();
        for forbidden in ["key", "token", "mac", "secret", "ciphertext", "recovery"] {
            assert!(!serialized.to_ascii_lowercase().contains(forbidden));
        }
    }
}
