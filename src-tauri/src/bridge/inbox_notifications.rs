//! Desktop notification history through the authenticated SharedCore owner.
use crate::matrix::auth::product::MatrixAuthCommandError;
use synara_core::app::notifications::MatrixInboxNotificationsPage;
use synara_core::transport::MatrixIpcErrorCategory;
use synara_core::Core;

pub(crate) async fn inbox_notifications(
    core: &Core,
    from: Option<String>,
    limit: Option<u16>,
    only: Option<String>,
) -> Result<MatrixInboxNotificationsPage, MatrixAuthCommandError> {
    let response = core
        .inbox_notifications(
            synara_core::app::notifications::MatrixInboxNotificationsRequest { from, limit, only },
        )
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
    Ok(response)
}
