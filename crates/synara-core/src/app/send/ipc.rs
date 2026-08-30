//! Credential-free send IPC result DTOs.
//!
//! Live Client send I/O stays in the desktop shell.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendTextResult {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendAttachmentResult {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: &'static str,
}

/// Dedicated Core attachment send ack. Event id and status only — no bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendRoomAttachmentResult {
    pub event_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSendPollResult {
    pub room_id: String,
    pub event_id: String,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixPollRespondResult {
    pub room_id: String,
    pub poll_event_id: String,
    pub event_id: String,
    pub status: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_result_serialization_is_privacy_safe() {
        let result = MatrixSendTextResult {
            room_id: "!room:example.org".into(),
            event_id: "$event:example.org".into(),
            local_txn_id: "local-txn-1".into(),
            status: "sent",
        };
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(
            json,
            r#"{"roomId":"!room:example.org","eventId":"$event:example.org","localTxnId":"local-txn-1","status":"sent"}"#
        );
        assert!(!json.contains("token"));
        assert!(!json.contains("ciphertext"));
    }
}
