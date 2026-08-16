//! Desktop bridge for `matrix_room_key_transfer_status` through `Core::command`.

use synara_core::app::room_keys::{
    NativeRoomKeyTransferKind, NativeRoomKeyTransferPhase, NativeRoomKeyTransferStatus,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_key_transfer_status(
    core: &Core,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_room_key_transfer_status".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_room_key_status_core_error)?;
    parse_status(response.payload)
}

fn parse_status(
    payload: serde_json::Value,
) -> Result<NativeRoomKeyTransferStatus, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Wire {
        session_generation: u64,
        kind: Option<NativeRoomKeyTransferKind>,
        phase: NativeRoomKeyTransferPhase,
        progress_percent: Option<u8>,
        keys_processed: u32,
        rooms_touched: u32,
        file_label: Option<String>,
        failure_diagnostic_id: Option<String>,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| status_response_error())?;
    Ok(NativeRoomKeyTransferStatus {
        session_generation: wire.session_generation,
        kind: wire.kind,
        phase: wire.phase,
        progress_percent: wire.progress_percent,
        keys_processed: wire.keys_processed,
        rooms_touched: wire.rooms_touched,
        file_label: wire.file_label,
        failure_diagnostic_id: wire.failure_diagnostic_id.map(intern_diagnostic),
    })
}

fn intern_diagnostic(id: String) -> &'static str {
    match id.as_str() {
        "p8.6-wrong-passphrase" => "p8.6-wrong-passphrase",
        "p8.6-stale-generation-cancelled" => "p8.6-stale-generation-cancelled",
        _ => Box::leak(id.into_boxed_str()),
    }
}

fn map_room_key_status_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.5-room-keys-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room-key transfer status is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-crypto.5-status-unavailable"),
        ),
    }
}

fn status_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room-key transfer status is unavailable.",
        "v-crypto.5-status-unavailable",
    )
}
