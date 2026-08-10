//! Stream topics, lifecycle states, and control payloads for Matrix IPC.

use serde::{Deserialize, Serialize};

use super::wire_counter::{
    deserialize_optional_wire_counter, deserialize_wire_counter, serialize_wire_counter,
};

/// Stream topics for IPC subscriptions. Snapshot/delta bodies are topic-typed
/// (see `stream_body::validate_stream_topic_body`, R0.3 / REV-005).
/// Wire form is a stable snake_case string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamTopic {
    /// Room list / space hierarchy projection.
    RoomList,
    /// Per-room timeline projection.
    Timeline,
    /// Room member list.
    Members,
    /// Typing notifications.
    Typing,
    /// Read receipts.
    Receipts,
    /// Account data stream.
    AccountData,
    /// Presence (if product-enabled).
    Presence,
    /// Notification candidates for desktop OS notifications.
    NotificationCandidates,
    /// Crypto / verification status projection.
    CryptoStatus,
    /// Send-queue / local-echo status.
    SendQueue,
}

impl StreamTopic {
    pub const ALL: &'static [StreamTopic] = &[
        Self::RoomList,
        Self::Timeline,
        Self::Members,
        Self::Typing,
        Self::Receipts,
        Self::AccountData,
        Self::Presence,
        Self::NotificationCandidates,
        Self::CryptoStatus,
        Self::SendQueue,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RoomList => "room_list",
            Self::Timeline => "timeline",
            Self::Members => "members",
            Self::Typing => "typing",
            Self::Receipts => "receipts",
            Self::AccountData => "account_data",
            Self::Presence => "presence",
            Self::NotificationCandidates => "notification_candidates",
            Self::CryptoStatus => "crypto_status",
            Self::SendQueue => "send_queue",
        }
    }
}

/// Deterministic stream lifecycle states (type-level; no live supervisor yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamLifecycleState {
    /// No subscription.
    Idle,
    /// Subscribe request in flight.
    Subscribing,
    /// Waiting for initial snapshot after subscribe ack.
    SnapshotPending,
    /// Snapshot applied; accepting ordered deltas.
    Live,
    /// Gap or stale generation detected; client must resubscribe for snapshot.
    ResyncRequired,
    /// Unsubscribe in flight; Rust side must free resources.
    Unsubscribing,
    /// Terminal clean close.
    Closed,
    /// Terminal failure (see error envelope).
    Failed,
}

/// Why a stream requires resync / snapshot resubscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResyncReason {
    SequenceGap,
    StaleSessionGeneration,
    UnknownKind,
    SnapshotRequired,
    SupervisorReset,
}

/// Reason for cancel messages (type-level cancellation tokens).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    ClientRequest,
    Timeout,
    SessionEnded,
    StreamClosed,
    Superseded,
}

// --- Control payloads (strongly typed; snapshot/delta bodies topic-validated) ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloPayload {
    /// Client-proposed protocol version (must equal server support set).
    pub client_protocol_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HelloAckPayload {
    /// Negotiated protocol version.
    pub protocol_version: u32,
    /// Session generation assigned/confirmed by the Rust host (wire-safe).
    #[serde(
        serialize_with = "serialize_wire_counter",
        deserialize_with = "deserialize_wire_counter"
    )]
    pub session_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubscribePayload {
    pub topic: StreamTopic,
    /// Opaque stream id chosen by the client (stable for the subscription).
    pub stream_id: String,
    /// Optional filter/params; domain schema in P1.4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsubscribePayload {
    pub stream_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubscribedPayload {
    pub stream_id: String,
    pub topic: StreamTopic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsubscribedPayload {
    pub stream_id: String,
    /// True when Rust-side resources for this stream are released.
    #[serde(default)]
    pub resources_released: bool,
}

fn default_stream_body() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotPayload {
    pub stream_id: String,
    pub topic: StreamTopic,
    /// Opaque snapshot identity for debugging / resync correlation.
    pub snapshot_id: String,
    /// Topic-bound domain body (validated via `validate_stream_topic_body`).
    #[serde(default = "default_stream_body")]
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeltaPayload {
    pub stream_id: String,
    pub topic: StreamTopic,
    /// Optional idempotency key for duplicate-delta coalescing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Topic-bound domain body (validated via `validate_stream_topic_body`).
    #[serde(default = "default_stream_body")]
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResyncRequiredPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_id: Option<String>,
    pub reason: ResyncReason,
    /// Last sequence the sender believes the peer applied (if known).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_wire_counter"
    )]
    pub last_applied_sequence: Option<u64>,
    /// Sequence that triggered the gap / rejection (if known).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_wire_counter"
    )]
    pub observed_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelPayload {
    /// Client- or host-issued cancellation token for long-running work.
    pub cancellation_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CancelReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PingPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PongPayload {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}
