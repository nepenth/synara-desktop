//! Credential-free V-CRYPTO.1 verification presentation DTOs.
//!
//! Live Client request/SAS ownership stays in the desktop shell.

use std::cmp::Ordering;

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
    KeysExchanging,
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
        NativeVerificationPhase::KeysExchanging => 3,
        NativeVerificationPhase::SasReady => 4,
        NativeVerificationPhase::Confirmed => 5,
        NativeVerificationPhase::Done => 6,
        NativeVerificationPhase::Mismatched => 7,
        NativeVerificationPhase::Cancelled => 8,
    }
}

/// Order action phases first, then prefer the newest request within a phase.
/// A freshly delivered verification must not be hidden behind an abandoned
/// request from an earlier app run.
pub fn compare_for_inbox(
    left: &NativeVerificationRequest,
    right: &NativeVerificationRequest,
) -> Ordering {
    phase_rank(left.phase)
        .cmp(&phase_rank(right.phase))
        .then_with(|| right.started_ts.cmp(&left.started_ts))
        .then_with(|| left.flow_id.cmp(&right.flow_id))
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
    fn inbox_order_prefers_newest_request_within_the_same_phase() {
        let request = |flow_id: &str, started_ts| NativeVerificationRequest {
            flow_id: flow_id.to_owned(),
            other_user_id: "@alice:example.org".to_owned(),
            other_device_id: Some("DEVICE".to_owned()),
            direction: NativeVerificationDirection::Incoming,
            phase: NativeVerificationPhase::Requested,
            started_ts: Some(started_ts),
            sas: None,
        };
        let mut requests = [request("older", 1), request("newer", 2)];
        requests.sort_by(compare_for_inbox);
        assert_eq!(requests[0].flow_id, "newer");
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
