//! Desktop bridges for homeserver push rules through `Core::command`.

use synara_core::app::notifications::{MatrixPushRulesSnapshot, MatrixPushRulesWriteResult};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn push_rules_snapshot(
    core: &Core,
) -> Result<MatrixPushRulesSnapshot, MatrixAuthCommandError> {
    let payload = dispatch(core, "matrix_push_rules_snapshot", serde_json::Value::Null).await?;
    serde_json::from_value(payload).map_err(|_| push_response_error())
}

pub(crate) async fn push_rules_set_default(
    core: &Core,
    encrypted: bool,
    one_to_one: bool,
    mode: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_push_rules_set_default",
        serde_json::json!({
            "encrypted": encrypted,
            "oneToOne": one_to_one,
            "mode": mode,
        }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn push_rules_set_mention(
    core: &Core,
    rule_id: String,
    enabled: bool,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_push_rules_set_mention",
        serde_json::json!({
            "ruleId": rule_id,
            "enabled": enabled,
        }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn push_rules_add_keyword(
    core: &Core,
    keyword: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_push_rules_add_keyword",
        serde_json::json!({ "keyword": keyword }),
    )
    .await?;
    parse_write(payload)
}

pub(crate) async fn push_rules_remove_keyword(
    core: &Core,
    keyword: String,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    let payload = dispatch(
        core,
        "matrix_push_rules_remove_keyword",
        serde_json::json!({ "keyword": keyword }),
    )
    .await?;
    parse_write(payload)
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: command.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload,
    })
    .await
    .map(|response| response.payload)
    .map_err(map_push_core_error)
}

fn parse_write(
    payload: serde_json::Value,
) -> Result<MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    #[derive(serde::Deserialize)]
    struct Wire {
        status: String,
    }
    let wire: Wire = serde_json::from_value(payload).map_err(|_| push_response_error())?;
    if wire.status != "ok" {
        return Err(push_response_error());
    }
    Ok(MatrixPushRulesWriteResult { status: "ok" })
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

fn push_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native push-rule editor is unavailable.",
        "v-push.sdk-failed",
    )
}
