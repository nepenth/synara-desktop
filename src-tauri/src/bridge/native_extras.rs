//! Desktop bridges for raw account data, room aliases, mutual rooms, room
//! upgrade and bulk redaction.
//!
//! Each call goes to a typed Core method. Errors keep the existing Tauri shape
//! and only closed diagnostic ids; anything unrecognised becomes the fallback.

use serde_json::Value as JsonValue;
use synara_core::core_api::{
    NativeAccountDataContent, NativeAccountDataGetRequest, NativeAccountDataSetRequest,
    NativeAccountDataTypes, NativeAccountDataTypesRequest, NativeBulkRedactRequest,
    NativeBulkRedactResult, NativeMutualRooms, NativeMutualRoomsRequest, NativeRoomAliasCheck,
    NativeRoomAliasCreateRequest, NativeRoomAliasRequest, NativeRoomIdRequest,
    NativeRoomLocalAliases, NativeRoomUpgradeRequest, NativeRoomUpgradeResult,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const FALLBACK: &str = "v-native-extras-failed";

/// Closed ids Core may return from these commands.
const KNOWN_DIAGNOSTICS: &[&str] = &[
    "v-account-data.requires-session",
    "v-account-data.invalid-type",
    "v-account-data.secret-type",
    "v-account-data.invalid-room",
    "v-account-data.room-not-found",
    "v-account-data.invalid-content",
    "v-account-data.content-too-large",
    "v-account-data.fetch-failed",
    "v-account-data.set-failed",
    "v-rooms-extras-requires-session",
    "v-rooms-alias-invalid",
    "v-rooms-alias-invalid-room",
    "v-rooms-alias-list-failed",
    "v-rooms-alias-forbidden",
    "v-rooms-alias-taken",
    "v-rooms-alias-not-found",
    "v-rooms-alias-create-failed",
    "v-rooms-alias-delete-failed",
    "v-rooms-alias-check-failed",
    "v-rooms-mutual-invalid-user",
    "v-rooms-mutual-failed",
    "v-rooms-upgrade-invalid-room",
    "v-rooms-upgrade-invalid-version",
    "v-rooms-upgrade-room-not-found",
    "v-rooms-upgrade-not-joined",
    "v-rooms-upgrade-forbidden",
    "v-rooms-upgrade-unsupported-version",
    "v-rooms-upgrade-failed",
    "v-rooms-bulk-redact-invalid-room",
    "v-rooms-bulk-redact-invalid-users",
    "v-rooms-bulk-redact-invalid-reason",
    "v-rooms-bulk-redact-invalid-since",
    "v-rooms-bulk-redact-room-not-found",
    "v-rooms-bulk-redact-not-joined",
    "v-rooms-bulk-redact-history-failed",
];

fn map_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic_id = error
        .diagnostic_id
        .as_deref()
        .and_then(|id| {
            KNOWN_DIAGNOSTICS
                .iter()
                .find(|known| **known == id)
                .copied()
        })
        .unwrap_or(FALLBACK);
    if error.category == MatrixIpcErrorCategory::Forbidden {
        return MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            diagnostic_id,
        );
    }
    let code = if diagnostic_id.contains("invalid") {
        "InvalidRequest"
    } else if diagnostic_id.ends_with("forbidden") {
        "Forbidden"
    } else {
        "Unknown"
    };
    MatrixAuthCommandError::new(code, "The native Matrix request failed.", diagnostic_id)
}

pub(crate) async fn account_data_types(
    core: &Core,
    room_id: Option<String>,
) -> Result<NativeAccountDataTypes, MatrixAuthCommandError> {
    core.account_data_types(NativeAccountDataTypesRequest { room_id })
        .await
        .map_err(map_error)
}

pub(crate) async fn account_data_get(
    core: &Core,
    event_type: String,
    room_id: Option<String>,
) -> Result<NativeAccountDataContent, MatrixAuthCommandError> {
    core.account_data_get(NativeAccountDataGetRequest {
        event_type,
        room_id,
    })
    .await
    .map_err(map_error)
}

pub(crate) async fn account_data_set(
    core: &Core,
    event_type: String,
    room_id: Option<String>,
    content: JsonValue,
) -> Result<NativeAccountDataContent, MatrixAuthCommandError> {
    core.account_data_set(NativeAccountDataSetRequest {
        event_type,
        room_id,
        content,
    })
    .await
    .map_err(map_error)
}

pub(crate) async fn room_local_aliases(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomLocalAliases, MatrixAuthCommandError> {
    core.room_local_aliases(NativeRoomIdRequest { room_id })
        .await
        .map_err(map_error)
}

pub(crate) async fn room_alias_create(
    core: &Core,
    alias: String,
    room_id: String,
) -> Result<(), MatrixAuthCommandError> {
    core.room_alias_create(NativeRoomAliasCreateRequest { alias, room_id })
        .await
        .map_err(map_error)
}

pub(crate) async fn room_alias_delete(
    core: &Core,
    alias: String,
) -> Result<(), MatrixAuthCommandError> {
    core.room_alias_delete(NativeRoomAliasRequest { alias })
        .await
        .map_err(map_error)
}

pub(crate) async fn room_alias_check(
    core: &Core,
    alias: String,
) -> Result<NativeRoomAliasCheck, MatrixAuthCommandError> {
    core.room_alias_check(NativeRoomAliasRequest { alias })
        .await
        .map_err(map_error)
}

pub(crate) async fn user_mutual_rooms(
    core: &Core,
    user_id: String,
) -> Result<NativeMutualRooms, MatrixAuthCommandError> {
    core.user_mutual_rooms(NativeMutualRoomsRequest { user_id })
        .await
        .map_err(map_error)
}

pub(crate) async fn room_upgrade(
    core: &Core,
    room_id: String,
    new_version: String,
    additional_creators: Vec<String>,
) -> Result<NativeRoomUpgradeResult, MatrixAuthCommandError> {
    core.room_upgrade(NativeRoomUpgradeRequest {
        room_id,
        new_version,
        additional_creators,
    })
    .await
    .map_err(map_error)
}

pub(crate) async fn room_bulk_redact(
    core: &Core,
    request: NativeBulkRedactRequest,
) -> Result<NativeBulkRedactResult, MatrixAuthCommandError> {
    core.room_bulk_redact(request).await.map_err(map_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_keep_closed_ids_and_hide_unknown_text() {
        let no_session = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("v-account-data.requires-session"),
        );
        assert_eq!(no_session.code, "Forbidden");
        assert_eq!(no_session.diagnostic_id, "v-account-data.requires-session");

        let secret = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-account-data.secret-type"),
        );
        assert_eq!(secret.diagnostic_id, "v-account-data.secret-type");

        let forbidden = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms-upgrade-forbidden"),
        );
        assert_eq!(forbidden.code, "Forbidden");

        let invalid = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms-alias-invalid"),
        );
        assert_eq!(invalid.code, "InvalidRequest");

        let unknown = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("https://example.org/token=secret"),
        );
        assert_eq!(unknown.diagnostic_id, FALLBACK);
    }
}
