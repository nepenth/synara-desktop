//! Desktop bridge for presence subscribe/unsubscribe through `Core::command`.
//!
//! Core owns the live `NativePresenceOwner` after the shell attaches it. These
//! adapters build the envelopes and map closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_presence_subscribe`
//! and `matrix_presence_unsubscribe`.

use synara_core::app::presence::NativePresenceSubscription;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn presence_subscribe(
    core: &Core,
    user_id: String,
) -> Result<NativePresenceSubscription, MatrixAuthCommandError> {
    let response = core
        .presence_subscribe(synara_core::core_api::MatrixPresenceSubscribeRequest { user_id })
        .await
        .map_err(map_presence_subscribe_core_error)?;
    Ok(response)
}

pub(crate) async fn presence_unsubscribe(
    core: &Core,
    subscription_id: String,
) -> Result<(), MatrixAuthCommandError> {
    core.presence_unsubscribe(synara_core::core_api::MatrixPresenceUnsubscribeRequest {
        subscription_id,
    })
    .await
    .map_err(map_presence_unsubscribe_core_error)?;
    Ok(())
}

fn map_presence_subscribe_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-presence-user-owner-missing",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix presence request is invalid.",
            "v-presence-invalid-user-id",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix presence session changed.",
            "v-presence-stale-session-generation",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix presence is unavailable.",
            "v-presence-store-read-failed",
        ),
    }
}

fn map_presence_unsubscribe_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-presence-user-owner-missing",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix presence request is invalid.",
            "v-presence-invalid-subscription-id",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix presence session changed.",
            "v-presence-stale-session-generation",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix presence is unavailable.",
            "v-presence-store-read-failed",
        ),
    }
}
