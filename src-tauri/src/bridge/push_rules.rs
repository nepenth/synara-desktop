//! Desktop bridges for homeserver push rules through `Core::command`.

use synara_core::app::notifications::{MatrixPushRulesSnapshot, MatrixPushRulesWriteResult};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn push_rules_snapshot(
    core: &Core,
) -> Result<MatrixPushRulesSnapshot, MatrixAuthCommandError> {
    let payload = core
        .push_rules_snapshot()
        .await
        .map_err(map_push_core_error)?;
    Ok(payload)
}

pub(crate) async fn push_rules_set_default(
    core: &Core,
    encrypted: bool,
    one_to_one: bool,
    mode: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = core
        .push_rules_set_default(synara_core::core_api::MatrixPushRulesSetDefaultRequest {
            encrypted,
            one_to_one,
            mode,
        })
        .await
        .map_err(map_push_core_error)?;
    Ok(payload)
}

pub(crate) async fn push_rules_set_mention(
    core: &Core,
    rule_id: String,
    enabled: bool,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = core
        .push_rules_set_mention(synara_core::core_api::MatrixPushRulesSetMentionRequest {
            rule_id,
            enabled,
        })
        .await
        .map_err(map_push_core_error)?;
    Ok(payload)
}

pub(crate) async fn push_rules_add_keyword(
    core: &Core,
    keyword: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = core
        .push_rules_add_keyword(synara_core::core_api::MatrixPushRulesKeywordRequest { keyword })
        .await
        .map_err(map_push_core_error)?;
    Ok(payload)
}

pub(crate) async fn push_rules_remove_keyword(
    core: &Core,
    keyword: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = core
        .push_rules_remove_keyword(synara_core::core_api::MatrixPushRulesKeywordRequest { keyword })
        .await
        .map_err(map_push_core_error)?;
    Ok(payload)
}

fn map_push_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-push.sdk-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native push-rule request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native push-rule editor is unavailable.",
            diagnostic,
        ),
    }
}
