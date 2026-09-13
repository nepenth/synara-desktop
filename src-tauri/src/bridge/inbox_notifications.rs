//! Desktop notification history through the authenticated SharedCore owner.
use crate::matrix::auth::product::MatrixAuthCommandError;
use synara_core::app::notifications::MatrixInboxNotificationsPage;
use synara_core::transport::{CommandEnvelope, MatrixIpcErrorCategory};
use synara_core::Core;

pub(crate) async fn inbox_notifications(
    core: &Core,
    from: Option<String>,
    limit: Option<u16>,
    only: Option<String>,
) -> Result<MatrixInboxNotificationsPage, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_inbox_notifications".to_owned(),
            session_generation: 0,
            request_id: None,
            payload: serde_json::json!({ "from": from, "limit": limit, "only": only }),
        })
        .await
        .map_err(|error| {
            let (code, message) = match error.category {
                MatrixIpcErrorCategory::Forbidden => {
                    ("Forbidden", "No native Matrix session is active.")
                }
                MatrixIpcErrorCategory::SdkInvariant => {
                    ("InvalidRequest", "The notifications request is invalid.")
                }
                _ => (
                    "NotificationsUnavailable",
                    "Notifications could not be loaded. Please try again.",
                ),
            };
            MatrixAuthCommandError::new(
                code,
                message,
                error
                    .diagnostic_id
                    .as_deref()
                    .unwrap_or("inbox-notifications.request-failed"),
            )
        })?;
    serde_json::from_value(response.payload).map_err(|_| {
        MatrixAuthCommandError::new(
            "NotificationsUnavailable",
            "Notifications could not be loaded. Please try again.",
            "inbox-notifications.invalid-response",
        )
    })
}
