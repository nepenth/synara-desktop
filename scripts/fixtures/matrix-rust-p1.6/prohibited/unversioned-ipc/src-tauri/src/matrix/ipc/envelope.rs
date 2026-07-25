//! PROHIBITED FIXTURE — P1.6 guardrail must reject this file.
//! Envelope shape with session_generation + sequence but NO protocol_version.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixIpcEnvelope {
    pub session_generation: u64,
    pub sequence: u64,
    pub kind: String,
}
