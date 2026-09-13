//! Notification history is fetched by the authenticated native Matrix client.
//! The renderer receives only the fields needed to render the Inbox, never a transport.

use matrix_sdk::{ruma::api::client::push::get_notifications::v3, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixInboxNotificationsRequest {
    pub from: Option<String>,
    pub limit: Option<u16>,
    pub only: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixInboxNotificationEvent {
    pub event_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub sender: String,
    pub origin_server_ts: u64,
    pub content: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixInboxNotification {
    pub event: MatrixInboxNotificationEvent,
    pub room_id: String,
    pub ts: u64,
    pub read: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatrixInboxNotificationsPage {
    pub notifications: Vec<MatrixInboxNotification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

impl MatrixInboxNotificationsRequest {
    fn into_sdk(self) -> Result<v3::Request, &'static str> {
        if self.from.as_ref().is_some_and(|token| token.len() > 4096)
            || self.limit.is_some_and(|limit| limit == 0 || limit > 100)
            || self.only.as_deref().is_some_and(|only| only != "highlight")
        {
            return Err("inbox-notifications.invalid-request");
        }
        let mut request = v3::Request::new();
        request.from = self.from;
        request.limit = Some(self.limit.unwrap_or(50).into());
        request.only = self.only;
        Ok(request)
    }
}

fn project_page(response: v3::Response) -> MatrixInboxNotificationsPage {
    MatrixInboxNotificationsPage {
        notifications: response
            .notifications
            .into_iter()
            .filter_map(|item| {
                // Match the renderer boundary: a malformed event cannot hide valid siblings.
                let event = serde_json::from_str(item.event.json().get()).ok()?;
                Some(MatrixInboxNotification {
                    event,
                    room_id: item.room_id.to_string(),
                    ts: item.ts.0.into(),
                    read: item.read,
                })
            })
            .collect(),
        next_token: response.next_token,
    }
}

pub async fn fetch_inbox_notifications(
    client: &Client,
    request: MatrixInboxNotificationsRequest,
) -> Result<MatrixInboxNotificationsPage, &'static str> {
    let request = request.into_sdk()?;
    let response = client
        .send(request)
        .await
        .map_err(|_| "inbox-notifications.request-failed")?;
    Ok(project_page(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::{room_id, serde::Raw, uint, MilliSecondsSinceUnixEpoch};

    #[test]
    fn request_preserves_pagination_and_highlight_with_bounded_limits() {
        let request = MatrixInboxNotificationsRequest {
            from: Some("next/token+exact".into()),
            limit: Some(30),
            only: Some("highlight".into()),
        }
        .into_sdk()
        .unwrap();
        assert_eq!(request.from.as_deref(), Some("next/token+exact"));
        assert_eq!(request.only.as_deref(), Some("highlight"));
        assert_eq!(request.limit, Some(uint!(30)));
        for request in [
            MatrixInboxNotificationsRequest {
                limit: Some(0),
                ..Default::default()
            },
            MatrixInboxNotificationsRequest {
                limit: Some(101),
                ..Default::default()
            },
            MatrixInboxNotificationsRequest {
                only: Some("all".into()),
                ..Default::default()
            },
            MatrixInboxNotificationsRequest {
                from: Some("a".repeat(4097)),
                ..Default::default()
            },
        ] {
            assert!(request.into_sdk().is_err());
        }
    }

    #[test]
    fn page_projects_display_fields_and_keeps_pagination_when_events_are_invalid() {
        let item = |event| {
            v3::Notification::new(
                vec![],
                Raw::from_json_string(event).unwrap(),
                false,
                room_id!("!room:example.org").to_owned(),
                MilliSecondsSinceUnixEpoch(uint!(123)),
            )
        };
        let mut response = v3::Response::new(vec![
            item(r#"{"event_id":"$event","type":"m.room.message","sender":"@alice:example.org","origin_server_ts":123,"content":{"msgtype":"m.text","body":"hello"},"unsigned":{"age":10},"extra":"excluded"}"#.into()),
            item(r#"{"type":"m.room.message"}"#.into()),
        ]);
        response.next_token = Some("next".into());
        let value = serde_json::to_value(project_page(response)).unwrap();
        assert_eq!(value["notifications"].as_array().unwrap().len(), 1);
        assert_eq!(
            value["notifications"][0]["event"]["content"]["body"],
            "hello"
        );
        assert_eq!(value["notifications"][0]["read"], false);
        assert_eq!(value["notifications"][0]["ts"], 123);
        assert!(value["notifications"][0]["event"].get("extra").is_none());
        assert_eq!(value["next_token"], "next");
        assert_eq!(
            serde_json::to_value(project_page(v3::Response::new(vec![]))).unwrap(),
            serde_json::json!({"notifications": []})
        );
    }
}
