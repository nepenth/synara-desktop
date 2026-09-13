//! Typed Apple-shell access to the same Core notification history command as desktop.
//! This wrapper owns no Matrix policy or client and cannot fetch independently.
use super::SharedCore;
use crate::app::notifications::MatrixInboxNotificationsPage;
use crate::transport::{
    CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory, MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxNotificationDto {
    pub room_id: String,
    pub event_id: String,
    pub sender: String,
    pub event_type: String,
    pub origin_server_ts: u64,
    pub content_json: String,
    pub unsigned_json: Option<String>,
    pub ts: u64,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxNotificationsPageDto {
    pub notifications: Vec<InboxNotificationDto>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxNotificationsError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InboxNotificationsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}
impl std::error::Error for InboxNotificationsError {}

fn failed(code: &'static str, description: &'static str) -> InboxNotificationsError {
    InboxNotificationsError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn unavailable() -> InboxNotificationsError {
    failed(
        "inbox-notifications.request-failed",
        "Notifications could not be loaded.",
    )
}

fn map_core_error(error: MatrixIpcError) -> InboxNotificationsError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => failed(
            "inbox-notifications.no-session",
            "No native Matrix session is active.",
        ),
        MatrixIpcErrorCategory::SdkInvariant => failed(
            "inbox-notifications.invalid-request",
            "The notifications request is invalid.",
        ),
        _ => unavailable(),
    }
}

fn page_dto(
    payload: serde_json::Value,
) -> Result<InboxNotificationsPageDto, InboxNotificationsError> {
    let page: MatrixInboxNotificationsPage =
        serde_json::from_value(payload).map_err(|_| unavailable())?;
    let notifications = page
        .notifications
        .into_iter()
        .map(|item| {
            Ok(InboxNotificationDto {
                room_id: item.room_id,
                event_id: item.event.event_id,
                sender: item.event.sender,
                event_type: item.event.event_type,
                origin_server_ts: item.event.origin_server_ts,
                content_json: serde_json::to_string(&item.event.content)
                    .map_err(|_| unavailable())?,
                unsigned_json: item
                    .event
                    .unsigned
                    .map(|value| serde_json::to_string(&value))
                    .transpose()
                    .map_err(|_| unavailable())?,
                ts: item.ts,
                read: item.read,
            })
        })
        .collect::<Result<Vec<_>, InboxNotificationsError>>()?;
    Ok(InboxNotificationsPageDto {
        notifications,
        next_token: page.next_token,
    })
}

impl SharedCore {
    pub async fn inbox_notifications(
        &self,
        from: Option<String>,
        limit: Option<u16>,
        only: Option<String>,
    ) -> Result<InboxNotificationsPageDto, InboxNotificationsError> {
        let payload = serde_json::json!({ "from": from, "limit": limit, "only": only });
        if serde_json::to_vec(&payload)
            .map_err(|_| unavailable())?
            .len()
            > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
        {
            return Err(failed(
                "inbox-notifications.invalid-request",
                "The notifications request is invalid.",
            ));
        }
        let response = self
            .core
            .command(CommandEnvelope {
                command: "matrix_inbox_notifications".to_owned(),
                session_generation: 0,
                request_id: None,
                payload,
            })
            .await
            .map_err(map_core_error)?;
        page_dto(response.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_page_preserves_native_fields_and_opaque_pagination() {
        let page = page_dto(serde_json::json!({
            "next_token": "opaque/token+1",
            "notifications": [{ "room_id": "!room:example.org", "ts": 123, "read": false,
                "event": { "event_id": "$event", "sender": "@alice:example.org", "type": "m.room.message", "origin_server_ts": 120,
                    "content": { "msgtype": "m.text", "body": "hello" }, "unsigned": { "age": 3 } } }]
        })).unwrap();
        assert_eq!(page.next_token.as_deref(), Some("opaque/token+1"));
        let item = &page.notifications[0];
        assert_eq!(item.event_id, "$event");
        assert_eq!(item.room_id, "!room:example.org");
        assert_eq!(item.origin_server_ts, 120);
        assert_eq!(item.ts, 123);
        assert!(!item.read);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&item.content_json).unwrap()["body"],
            "hello"
        );
        assert_eq!(item.unsigned_json.as_deref(), Some("{\"age\":3}"));
        assert!(page_dto(serde_json::json!({})).is_err());
        assert!(page_dto(serde_json::json!({ "notifications": [] }))
            .unwrap()
            .notifications
            .is_empty());
    }

    #[tokio::test]
    async fn ffi_inbox_dispatches_the_shared_command_and_preserves_query() {
        use crate::core::CoreState;
        use crate::transport::{CommandFuture, CommandRegistry};
        use std::sync::Arc;

        let mut registry = CommandRegistry::new();
        registry
            .register(
                "matrix_inbox_notifications",
                |_state: Arc<CoreState>, request: CommandEnvelope| -> CommandFuture {
                    Box::pin(async move {
                        assert_eq!(request.session_generation, 0);
                        assert_eq!(
                            request.payload,
                            serde_json::json!({
                                "from": "opaque/token+1", "limit": 24, "only": "highlight"
                            })
                        );
                        Ok(serde_json::json!({ "notifications": [], "next_token": "next-page" }))
                    })
                },
            )
            .unwrap();
        let mut shared = SharedCore::new();
        shared.core = crate::Core::with_registry(
            Arc::new(super::super::IosFailClosedPlatform::new()),
            registry,
        );
        let page = shared
            .inbox_notifications(
                Some("opaque/token+1".into()),
                Some(24),
                Some("highlight".into()),
            )
            .await
            .unwrap();
        assert!(page.notifications.is_empty());
        assert_eq!(page.next_token.as_deref(), Some("next-page"));
    }

    #[tokio::test]
    async fn ffi_inbox_without_session_fails_closed_without_echoing_token() {
        let shared = SharedCore::new();
        let error = shared
            .inbox_notifications(
                Some("private-token".into()),
                Some(24),
                Some("highlight".into()),
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            failed(
                "inbox-notifications.no-session",
                "No native Matrix session is active."
            )
        );
        assert!(!format!("{error:?}").contains("private-token"));
    }
}
