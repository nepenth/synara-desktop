//! Versioned Matrix IPC envelope and kind-discriminated messages.

use serde::{Deserialize, Serialize};

use super::error::MatrixIpcError;
use super::stream::{
    CancelPayload, DeltaPayload, HelloAckPayload, HelloPayload, PingPayload, PongPayload,
    ResyncRequiredPayload, SnapshotPayload, SubscribePayload, SubscribedPayload,
    UnsubscribePayload, UnsubscribedPayload,
};
use super::stream_body::validate_stream_topic_body;
use super::version::MATRIX_IPC_PROTOCOL_VERSION;
use super::wire_counter::{
    deserialize_wire_counter, is_valid_wire_counter, serialize_wire_counter,
};

/// Stable wire `kind` discriminators (snake_case).
pub const KIND_HELLO: &str = "hello";
pub const KIND_HELLO_ACK: &str = "hello_ack";
pub const KIND_SUBSCRIBE: &str = "subscribe";
pub const KIND_UNSUBSCRIBE: &str = "unsubscribe";
pub const KIND_SUBSCRIBED: &str = "subscribed";
pub const KIND_UNSUBSCRIBED: &str = "unsubscribed";
pub const KIND_SNAPSHOT: &str = "snapshot";
pub const KIND_DELTA: &str = "delta";
pub const KIND_RESYNC_REQUIRED: &str = "resync_required";
pub const KIND_CANCEL: &str = "cancel";
pub const KIND_ERROR: &str = "error";
pub const KIND_PING: &str = "ping";
pub const KIND_PONG: &str = "pong";

/// Exhaustive list of P1.3 control kinds.
pub const MATRIX_IPC_KINDS: &[&str] = &[
    KIND_HELLO,
    KIND_HELLO_ACK,
    KIND_SUBSCRIBE,
    KIND_UNSUBSCRIBE,
    KIND_SUBSCRIBED,
    KIND_UNSUBSCRIBED,
    KIND_SNAPSHOT,
    KIND_DELTA,
    KIND_RESYNC_REQUIRED,
    KIND_CANCEL,
    KIND_ERROR,
    KIND_PING,
    KIND_PONG,
];

/// Kind-discriminated message body (adjacent tagging: `kind` + `payload`).
///
/// Unknown future kinds: **reject at the command boundary**. For stream events
/// with invalid generation/sequence, emit `resync_required` rather than
/// silently forward.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum MatrixIpcMessage {
    Hello(HelloPayload),
    HelloAck(HelloAckPayload),
    Subscribe(SubscribePayload),
    Unsubscribe(UnsubscribePayload),
    Subscribed(SubscribedPayload),
    Unsubscribed(UnsubscribedPayload),
    Snapshot(SnapshotPayload),
    Delta(DeltaPayload),
    ResyncRequired(ResyncRequiredPayload),
    Cancel(CancelPayload),
    Error(MatrixIpcError),
    Ping(PingPayload),
    Pong(PongPayload),
}

impl MatrixIpcMessage {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Hello(_) => KIND_HELLO,
            Self::HelloAck(_) => KIND_HELLO_ACK,
            Self::Subscribe(_) => KIND_SUBSCRIBE,
            Self::Unsubscribe(_) => KIND_UNSUBSCRIBE,
            Self::Subscribed(_) => KIND_SUBSCRIBED,
            Self::Unsubscribed(_) => KIND_UNSUBSCRIBED,
            Self::Snapshot(_) => KIND_SNAPSHOT,
            Self::Delta(_) => KIND_DELTA,
            Self::ResyncRequired(_) => KIND_RESYNC_REQUIRED,
            Self::Cancel(_) => KIND_CANCEL,
            Self::Error(_) => KIND_ERROR,
            Self::Ping(_) => KIND_PING,
            Self::Pong(_) => KIND_PONG,
        }
    }
}

/// Versioned IPC envelope (plan §6.3, R0.3 wire freeze).
///
/// Required fields: `protocolVersion`, `sessionGeneration`, `sequence`, `kind`
/// (via message), bounded payload. Optional: `streamId`, `requestId`.
/// Counters are wire-safe integers (`0..=MAX_WIRE_COUNTER`). Stream-scoped
/// kinds require a single authoritative `streamId` on the envelope that matches
/// any payload `streamId` (REV-005).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixIpcEnvelope {
    pub protocol_version: u32,
    #[serde(
        serialize_with = "serialize_wire_counter",
        deserialize_with = "deserialize_wire_counter"
    )]
    pub session_generation: u64,
    /// Present for stream-scoped messages (subscribe, snapshot, delta, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    /// Monotonically increasing per stream (or per control channel when no stream).
    #[serde(
        serialize_with = "serialize_wire_counter",
        deserialize_with = "deserialize_wire_counter"
    )]
    pub sequence: u64,
    /// Optional correlation / request id for request-response pairing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(flatten)]
    pub message: MatrixIpcMessage,
}

impl MatrixIpcEnvelope {
    pub fn new(session_generation: u64, sequence: u64, message: MatrixIpcMessage) -> Self {
        debug_assert!(
            is_valid_wire_counter(session_generation) && is_valid_wire_counter(sequence),
            "envelope counters must be wire-safe"
        );
        Self {
            protocol_version: MATRIX_IPC_PROTOCOL_VERSION,
            session_generation,
            stream_id: None,
            sequence,
            request_id: None,
            message,
        }
    }

    pub fn with_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.stream_id = Some(stream_id.into());
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn kind(&self) -> &'static str {
        self.message.kind()
    }

    /// Serialize to JSON value (for tests / boundary checks).
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Parse a JSON value into a typed envelope.
    ///
    /// Unknown `kind` values fail deserialization (reject-at-boundary policy).
    /// Non-object `payload` values are rejected so empty structs cannot accept
    /// arrays/scalars (mirrors the TypeScript `isObject(payload)` guard).
    /// Wire counters outside `0..=MAX_WIRE_COUNTER` fail (REV-004).
    /// Stream-scoped kinds enforce a single authoritative stream id (REV-005).
    /// Snapshot/delta bodies are topic-typed and reject secrets/media bytes.
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        validate_payload_is_object(&value)?;
        let env: Self = serde_json::from_value(value)?;
        env.validate_stream_id_authority()?;
        env.validate_stream_bodies()?;
        Ok(env)
    }

    /// R0.3 residual / REV-005: bind snapshot/delta body to topic DTOs.
    fn validate_stream_bodies(&self) -> Result<(), serde_json::Error> {
        match &self.message {
            MatrixIpcMessage::Snapshot(p) => validate_stream_topic_body(p.topic, &p.body),
            MatrixIpcMessage::Delta(p) => validate_stream_topic_body(p.topic, &p.body),
            _ => Ok(()),
        }
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(s)?;
        Self::from_json_value(value)
    }

    /// R0.3 / REV-005: envelope `streamId` is authoritative for stream-scoped kinds.
    fn validate_stream_id_authority(&self) -> Result<(), serde_json::Error> {
        let payload_stream = payload_stream_id(&self.message);
        match (self.kind(), self.stream_id.as_deref(), payload_stream) {
            // Kinds that always carry a payload stream id: require envelope match.
            (
                KIND_SUBSCRIBE | KIND_UNSUBSCRIBE | KIND_SUBSCRIBED | KIND_UNSUBSCRIBED
                | KIND_SNAPSHOT | KIND_DELTA,
                Some(env_id),
                Some(pay_id),
            ) if env_id == pay_id => Ok(()),
            (
                KIND_SUBSCRIBE | KIND_UNSUBSCRIBE | KIND_SUBSCRIBED | KIND_UNSUBSCRIBED
                | KIND_SNAPSHOT | KIND_DELTA,
                _,
                _,
            ) => Err(serde::de::Error::custom(
                "stream-scoped envelope requires matching envelope.streamId and payload.streamId",
            )),
            // resync_required: optional; when both present they must match.
            (KIND_RESYNC_REQUIRED, Some(env_id), Some(pay_id)) if env_id == pay_id => Ok(()),
            (KIND_RESYNC_REQUIRED, Some(_), Some(_)) => Err(serde::de::Error::custom(
                "resync_required streamId mismatch between envelope and payload",
            )),
            (KIND_RESYNC_REQUIRED, None, Some(_)) => Err(serde::de::Error::custom(
                "resync_required payload.streamId requires envelope.streamId",
            )),
            _ => Ok(()),
        }
    }
}

fn payload_stream_id(message: &MatrixIpcMessage) -> Option<&str> {
    match message {
        MatrixIpcMessage::Subscribe(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::Unsubscribe(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::Subscribed(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::Unsubscribed(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::Snapshot(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::Delta(p) => Some(p.stream_id.as_str()),
        MatrixIpcMessage::ResyncRequired(p) => p.stream_id.as_deref(),
        _ => None,
    }
}

/// Reject envelopes whose `payload` is not a JSON object (arrays/scalars/null).
/// Serde can otherwise deserialize empty structs from `[]`, which would diverge
/// from the TypeScript boundary parser.
fn validate_payload_is_object(value: &serde_json::Value) -> Result<(), serde_json::Error> {
    match value.get("payload") {
        None => Ok(()), // missing payload: let serde report the real error
        Some(payload) if payload.is_object() => Ok(()),
        Some(_) => Err(serde::de::Error::custom(
            "matrix ipc envelope payload must be a JSON object",
        )),
    }
}
