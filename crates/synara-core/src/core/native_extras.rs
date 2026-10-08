//! Typed commands for raw account data, room aliases, mutual rooms, room
//! upgrade and bulk redaction.

use super::*;

pub use crate::app::account_data::{
    NativeAccountDataContent, NativeAccountDataGetRequest, NativeAccountDataSetRequest,
    NativeAccountDataTypes, NativeAccountDataTypesRequest, RawAccountDataError,
};
pub use crate::app::room_profile::{
    NativeBulkRedactRequest, NativeBulkRedactResult, NativeMutualRooms, NativeMutualRoomsRequest,
    NativeRoomAliasAvailability, NativeRoomAliasCheck, NativeRoomAliasCreateRequest,
    NativeRoomAliasRequest, NativeRoomIdRequest, NativeRoomLocalAliases, NativeRoomUpgradeRequest,
    NativeRoomUpgradeResult,
};

fn account_data_owner(state: &Arc<CoreState>) -> Result<Arc<NativeImagePackOwner>, MatrixIpcError> {
    state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("v-account-data.requires-session")
    })
}

fn room_owner(state: &Arc<CoreState>) -> Result<Arc<NativeRoomJoinRuleOwner>, MatrixIpcError> {
    state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("v-rooms-extras-requires-session")
    })
}

fn raw_account_data_error(error: RawAccountDataError) -> MatrixIpcError {
    // Forbidden is reserved for "no session" so shells can tell the two apart.
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error.diagnostic_id())
}

fn room_extras_error(diagnostic_id: &'static str) -> MatrixIpcError {
    // Forbidden is reserved for "no session"; a power-level refusal keeps its
    // own closed id under SdkInvariant.
    let category = if diagnostic_id == "v-send.r-room-profile-join-rule-requires-session" {
        MatrixIpcErrorCategory::Forbidden
    } else {
        MatrixIpcErrorCategory::SdkInvariant
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) async fn account_data_types(
    state: &Arc<CoreState>,
    payload: NativeAccountDataTypesRequest,
) -> Result<NativeAccountDataTypes, MatrixIpcError> {
    account_data_owner(state)?
        .account_data_types(payload.room_id.as_deref())
        .map_err(raw_account_data_error)
}

pub(super) async fn account_data_get(
    state: &Arc<CoreState>,
    payload: NativeAccountDataGetRequest,
) -> Result<NativeAccountDataContent, MatrixIpcError> {
    account_data_owner(state)?
        .account_data_get(&payload.event_type, payload.room_id.as_deref())
        .await
        .map_err(raw_account_data_error)
}

pub(super) async fn account_data_set(
    state: &Arc<CoreState>,
    payload: NativeAccountDataSetRequest,
) -> Result<NativeAccountDataContent, MatrixIpcError> {
    account_data_owner(state)?
        .account_data_set(
            &payload.event_type,
            payload.room_id.as_deref(),
            payload.content,
        )
        .await
        .map_err(raw_account_data_error)
}

pub(super) async fn room_local_aliases(
    state: &Arc<CoreState>,
    payload: NativeRoomIdRequest,
) -> Result<NativeRoomLocalAliases, MatrixIpcError> {
    room_owner(state)?
        .local_aliases(&payload.room_id)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn room_alias_create(
    state: &Arc<CoreState>,
    payload: NativeRoomAliasCreateRequest,
) -> Result<(), MatrixIpcError> {
    room_owner(state)?
        .alias_create(&payload.alias, &payload.room_id)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn room_alias_delete(
    state: &Arc<CoreState>,
    payload: NativeRoomAliasRequest,
) -> Result<(), MatrixIpcError> {
    room_owner(state)?
        .alias_delete(&payload.alias)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn room_alias_check(
    state: &Arc<CoreState>,
    payload: NativeRoomAliasRequest,
) -> Result<NativeRoomAliasCheck, MatrixIpcError> {
    room_owner(state)?
        .alias_check(&payload.alias)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn user_mutual_rooms(
    state: &Arc<CoreState>,
    payload: NativeMutualRoomsRequest,
) -> Result<NativeMutualRooms, MatrixIpcError> {
    room_owner(state)?
        .mutual_rooms(&payload.user_id)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn room_upgrade(
    state: &Arc<CoreState>,
    payload: NativeRoomUpgradeRequest,
) -> Result<NativeRoomUpgradeResult, MatrixIpcError> {
    room_owner(state)?
        .upgrade_room(&payload.room_id, &payload.new_version)
        .await
        .map_err(room_extras_error)
}

pub(super) async fn room_bulk_redact(
    state: &Arc<CoreState>,
    payload: NativeBulkRedactRequest,
) -> Result<NativeBulkRedactResult, MatrixIpcError> {
    room_owner(state)?
        .bulk_redact(payload)
        .await
        .map_err(room_extras_error)
}

impl Core {
    /// Typed `matrix_account_data_types`.
    pub async fn account_data_types(
        &self,
        request: NativeAccountDataTypesRequest,
    ) -> Result<NativeAccountDataTypes, MatrixIpcError> {
        account_data_types(&self.state, request).await
    }

    /// Typed `matrix_account_data_get`.
    pub async fn account_data_get(
        &self,
        request: NativeAccountDataGetRequest,
    ) -> Result<NativeAccountDataContent, MatrixIpcError> {
        account_data_get(&self.state, request).await
    }

    /// Typed `matrix_account_data_set`.
    pub async fn account_data_set(
        &self,
        request: NativeAccountDataSetRequest,
    ) -> Result<NativeAccountDataContent, MatrixIpcError> {
        account_data_set(&self.state, request).await
    }

    /// Typed `matrix_room_local_aliases`.
    pub async fn room_local_aliases(
        &self,
        request: NativeRoomIdRequest,
    ) -> Result<NativeRoomLocalAliases, MatrixIpcError> {
        room_local_aliases(&self.state, request).await
    }

    /// Typed `matrix_room_alias_create`.
    pub async fn room_alias_create(
        &self,
        request: NativeRoomAliasCreateRequest,
    ) -> Result<(), MatrixIpcError> {
        room_alias_create(&self.state, request).await
    }

    /// Typed `matrix_room_alias_delete`.
    pub async fn room_alias_delete(
        &self,
        request: NativeRoomAliasRequest,
    ) -> Result<(), MatrixIpcError> {
        room_alias_delete(&self.state, request).await
    }

    /// Typed `matrix_room_alias_check`.
    pub async fn room_alias_check(
        &self,
        request: NativeRoomAliasRequest,
    ) -> Result<NativeRoomAliasCheck, MatrixIpcError> {
        room_alias_check(&self.state, request).await
    }

    /// Typed `matrix_user_mutual_rooms`.
    pub async fn user_mutual_rooms(
        &self,
        request: NativeMutualRoomsRequest,
    ) -> Result<NativeMutualRooms, MatrixIpcError> {
        user_mutual_rooms(&self.state, request).await
    }

    /// Typed `matrix_room_upgrade`.
    pub async fn room_upgrade(
        &self,
        request: NativeRoomUpgradeRequest,
    ) -> Result<NativeRoomUpgradeResult, MatrixIpcError> {
        room_upgrade(&self.state, request).await
    }

    /// Typed `matrix_room_bulk_redact`.
    pub async fn room_bulk_redact(
        &self,
        request: NativeBulkRedactRequest,
    ) -> Result<NativeBulkRedactResult, MatrixIpcError> {
        room_bulk_redact(&self.state, request).await
    }
}

#[cfg(test)]
macro_rules! typed_json_adapter {
    ($name:ident, $request:ty, $call:ident, $invalid:literal) => {
        pub(super) fn $name(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
            Box::pin(async move {
                let payload: $request = serde_json::from_value(request.payload)
                    .map_err(|_| core_state_error($invalid))?;
                let response = $call(&state, payload).await?;
                serde_json::to_value(response)
                    .map_err(|_| core_state_error("v-native-extras-serialization-failed"))
            })
        }
    };
}

#[cfg(test)]
typed_json_adapter!(
    matrix_account_data_types,
    NativeAccountDataTypesRequest,
    account_data_types,
    "v-account-data.invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_account_data_get,
    NativeAccountDataGetRequest,
    account_data_get,
    "v-account-data.invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_account_data_set,
    NativeAccountDataSetRequest,
    account_data_set,
    "v-account-data.invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_local_aliases,
    NativeRoomIdRequest,
    room_local_aliases,
    "v-rooms-alias-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_alias_create,
    NativeRoomAliasCreateRequest,
    room_alias_create,
    "v-rooms-alias-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_alias_delete,
    NativeRoomAliasRequest,
    room_alias_delete,
    "v-rooms-alias-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_alias_check,
    NativeRoomAliasRequest,
    room_alias_check,
    "v-rooms-alias-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_user_mutual_rooms,
    NativeMutualRoomsRequest,
    user_mutual_rooms,
    "v-rooms-mutual-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_upgrade,
    NativeRoomUpgradeRequest,
    room_upgrade,
    "v-rooms-upgrade-invalid-payload"
);
#[cfg(test)]
typed_json_adapter!(
    matrix_room_bulk_redact,
    NativeBulkRedactRequest,
    room_bulk_redact,
    "v-rooms-bulk-redact-invalid-payload"
);

#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    for (name, handler) in [
        (
            "matrix_account_data_types",
            matrix_account_data_types as fn(Arc<CoreState>, CommandEnvelope) -> CommandFuture,
        ),
        ("matrix_account_data_get", matrix_account_data_get),
        ("matrix_account_data_set", matrix_account_data_set),
        ("matrix_room_local_aliases", matrix_room_local_aliases),
        ("matrix_room_alias_create", matrix_room_alias_create),
        ("matrix_room_alias_delete", matrix_room_alias_delete),
        ("matrix_room_alias_check", matrix_room_alias_check),
        ("matrix_user_mutual_rooms", matrix_user_mutual_rooms),
        ("matrix_room_upgrade", matrix_room_upgrade),
        ("matrix_room_bulk_redact", matrix_room_bulk_redact),
    ] {
        registry
            .register(name, handler)
            .expect("built-in native extras must remain in the command census");
    }
}
