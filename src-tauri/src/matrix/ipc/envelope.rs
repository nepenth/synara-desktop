//! Versioned Matrix IPC envelope and kind-discriminated messages.

use serde::{Deserialize, Serialize};

use super::error::MatrixIpcError;
use super::stream::{
    CancelPayload, DeltaPayload, HelloAckPayload, HelloPayload, PingPayload, PongPayload,
    ResyncRequiredPayload, SnapshotPayload, SubscribePayload, SubscribedPayload,
    UnsubscribePayload, UnsubscribedPayload,
};
use super::version::MATRIX_IPC_PROTOCOL_VERSION;

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

/// Versioned IPC envelope (plan §6.3).
///
/// Required fields: `protocolVersion`, `sessionGeneration`, `sequence`, `kind`
/// (via message), bounded payload. Optional: `streamId`, `requestId`.
/// Timestamp is omitted unless a future product semantic requires it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixIpcEnvelope {
    pub protocol_version: u32,
    pub session_generation: u64,
    /// Present for stream-scoped messages (subscribe, snapshot, delta, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    /// Monotonically increasing per stream (or per control channel when no stream).
    pub sequence: u64,
    /// Optional correlation / request id for request-response pairing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(flatten)]
    pub message: MatrixIpcMessage,
}

impl MatrixIpcEnvelope {
    pub fn new(session_generation: u64, sequence: u64, message: MatrixIpcMessage) -> Self {
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
    /// Unknown `kind` values fail deserialization (reject-at-boundary policy).
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
