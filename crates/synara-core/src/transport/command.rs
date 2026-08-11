//! Request/response command envelopes (P2).
//!
//! This is deliberately distinct from [`super::MatrixIpcEnvelope`], which
//! carries stream-control messages (`hello`, `snapshot`, `delta`, …). Command
//! envelopes preserve React's existing `matrix_*` command names while letting
//! desktop and uniffi carry the same typed request/response boundary.

use serde::{Deserialize, Serialize};

use super::{is_valid_wire_counter, MAX_ENVELOPE_PAYLOAD_JSON_BYTES};

/// One UI → core product command request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandEnvelope {
    /// Exact stable React-facing native command name (e.g. `matrix_login_password`).
    pub command: String,
    /// Wire-safe session generation supplied by the shell.
    pub session_generation: u64,
    /// Optional opaque request correlation id (never a token or credential).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Command-specific JSON DTO. Handler validates the concrete shape.
    pub payload: serde_json::Value,
}

/// One core → UI command response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandResponseEnvelope {
    pub command: String,
    pub session_generation: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub payload: serde_json::Value,
}

/// Pure validation error. The core maps it to a static [`super::MatrixIpcError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEnvelopeError {
    InvalidCommandName,
    InvalidSessionGeneration,
    InvalidRequestId,
    PayloadTooLarge,
}

impl CommandEnvelope {
    pub fn validate(&self) -> Result<(), CommandEnvelopeError> {
        if !is_valid_command_name(&self.command) {
            return Err(CommandEnvelopeError::InvalidCommandName);
        }
        if !is_valid_wire_counter(self.session_generation) {
            return Err(CommandEnvelopeError::InvalidSessionGeneration);
        }
        if self
            .request_id
            .as_deref()
            .is_some_and(|id| id.is_empty() || id.len() > 128 || !id.is_ascii())
        {
            return Err(CommandEnvelopeError::InvalidRequestId);
        }
        let size = serde_json::to_vec(&self.payload)
            .map_err(|_| CommandEnvelopeError::PayloadTooLarge)?
            .len();
        if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
            return Err(CommandEnvelopeError::PayloadTooLarge);
        }
        Ok(())
    }

    pub fn response(&self, payload: serde_json::Value) -> CommandResponseEnvelope {
        CommandResponseEnvelope {
            command: self.command.clone(),
            session_generation: self.session_generation,
            request_id: self.request_id.clone(),
            payload,
        }
    }
}

/// Exact namespace of product commands retained from desktop's Tauri invoke API.
pub fn is_valid_command_name(command: &str) -> bool {
    command.len() >= "matrix_x".len()
        && command.len() <= 128
        && command.starts_with("matrix_")
        && command
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(command: &str) -> CommandEnvelope {
        CommandEnvelope {
            command: command.to_owned(),
            session_generation: 7,
            request_id: Some("request-7".to_owned()),
            payload: serde_json::json!({"roomId":"!opaque:example.org"}),
        }
    }

    #[test]
    fn command_envelope_accepts_exact_matrix_namespace() {
        assert!(request("matrix_room_list_snapshot").validate().is_ok());
        assert!(is_valid_command_name("matrix_login_password"));
    }

    #[test]
    fn command_envelope_rejects_unsafe_names_and_counters() {
        for name in ["", "matrix-foo", "Matrix_login", "desktop_show", "matrix_ä"] {
            assert_eq!(
                request(name).validate(),
                Err(CommandEnvelopeError::InvalidCommandName)
            );
        }
        let mut invalid_counter = request("matrix_login_password");
        invalid_counter.session_generation = u64::MAX;
        assert_eq!(
            invalid_counter.validate(),
            Err(CommandEnvelopeError::InvalidSessionGeneration)
        );
    }

    #[test]
    fn response_preserves_only_correlation_and_payload() {
        let request = request("matrix_login_flows");
        let response = request.response(serde_json::json!({"flows":[]}));
        assert_eq!(response.command, "matrix_login_flows");
        assert_eq!(response.request_id.as_deref(), Some("request-7"));
        assert_eq!(response.session_generation, 7);
    }
}
