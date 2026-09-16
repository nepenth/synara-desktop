//! Live MSC4426 status owner. Uses Account set/clear/read APIs.

use matrix_sdk::{
    ruma::{
        api::client::error::ErrorKind,
        profile::{ProfileFieldName, ProfileFieldValue},
        UserId,
    },
    Client,
};

use super::{
    parse_status_write, project_status_field, NativeInCall, NativeUserStatusSnapshot,
    NativeUserStatusWriteResult, StatusWrite,
};

enum UserStatusSource {
    Client(Client),
    Static {
        snapshot: NativeUserStatusSnapshot,
        can_set_status: bool,
    },
}

/// Owns one authenticated session's MSC4426 status / in-call reads and own writes.
pub struct NativeUserStatusOwner {
    source: UserStatusSource,
    session_generation: u64,
}

impl NativeUserStatusOwner {
    pub fn start(client: &Client, session_generation: u64) -> Self {
        Self {
            source: UserStatusSource::Client(client.clone()),
            session_generation,
        }
    }

    /// Test / fail-closed helper: project a snapshot without a homeserver.
    pub fn from_static_snapshot(
        session_generation: u64,
        snapshot: NativeUserStatusSnapshot,
        can_set_status: bool,
    ) -> Self {
        Self {
            source: UserStatusSource::Static {
                snapshot,
                can_set_status,
            },
            session_generation,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub async fn snapshot(&self, user_id: &str) -> Result<NativeUserStatusSnapshot, &'static str> {
        let user_id = UserId::parse(user_id).map_err(|_| "v-user-status-invalid-user-id")?;
        let user_id_string = user_id.to_string();
        match &self.source {
            UserStatusSource::Static { snapshot, .. } => {
                let mut snapshot = snapshot.clone();
                snapshot.user_id = user_id_string;
                snapshot.session_generation = self.session_generation;
                Ok(snapshot)
            }
            UserStatusSource::Client(client) => {
                Ok(fetch_user_status_snapshot(client, self.session_generation, user_id).await)
            }
        }
    }

    pub async fn set(
        &self,
        emoji: &str,
        text: &str,
    ) -> Result<NativeUserStatusWriteResult, &'static str> {
        let write = parse_status_write(emoji, text)?;
        match &self.source {
            UserStatusSource::Static { can_set_status, .. } => {
                if !*can_set_status {
                    return Err("v-user-status-unsupported");
                }
                Ok(NativeUserStatusWriteResult {
                    status: "ok".to_owned(),
                })
            }
            UserStatusSource::Client(client) => {
                ensure_can_set_status(client).await?;
                match write {
                    StatusWrite::Clear => client
                        .account()
                        .clear_status()
                        .await
                        .map_err(|_| "v-user-status-set-sdk-failed")?,
                    StatusWrite::Set { emoji, text } => client
                        .account()
                        .set_status(emoji, text)
                        .await
                        .map_err(|_| "v-user-status-set-sdk-failed")?,
                }
                Ok(NativeUserStatusWriteResult {
                    status: "ok".to_owned(),
                })
            }
        }
    }

    pub async fn clear(&self) -> Result<NativeUserStatusWriteResult, &'static str> {
        self.set("", "").await
    }
}

async fn ensure_can_set_status(client: &Client) -> Result<(), &'static str> {
    let fields = client
        .homeserver_capabilities()
        .extended_profile_fields()
        .await
        .map_err(|_| "v-user-status-unsupported")?;
    if fields.can_set_field(&ProfileFieldName::Status) {
        Ok(())
    } else {
        Err("v-user-status-unsupported")
    }
}

async fn fetch_user_status_snapshot(
    client: &Client,
    session_generation: u64,
    user_id: matrix_sdk::ruma::OwnedUserId,
) -> NativeUserStatusSnapshot {
    let user_id_string = user_id.to_string();
    NativeUserStatusSnapshot {
        session_generation,
        user_id: user_id_string,
        user_status: fetch_status_field(client, user_id.clone()).await,
        in_call: fetch_call_field(client, user_id).await,
    }
}

async fn fetch_status_field(
    client: &Client,
    user_id: matrix_sdk::ruma::OwnedUserId,
) -> Option<super::NativeUserStatus> {
    match client
        .account()
        .fetch_profile_field_of(user_id, ProfileFieldName::Status)
        .await
    {
        Ok(Some(ProfileFieldValue::Status(status))) => {
            project_status_field(status.emoji, status.text)
        }
        Ok(_) => None,
        Err(error) if profile_field_is_absent(&error) => None,
        Err(_) => None,
    }
}

async fn fetch_call_field(
    client: &Client,
    user_id: matrix_sdk::ruma::OwnedUserId,
) -> Option<NativeInCall> {
    match client
        .account()
        .fetch_profile_field_of(user_id, ProfileFieldName::Call)
        .await
    {
        Ok(Some(ProfileFieldValue::Call(call))) => Some(NativeInCall {
            call_joined_ts: call.call_joined_ts.map(|ts| u64::from(ts.get())),
        }),
        Ok(_) => None,
        Err(error) if profile_field_is_absent(&error) => None,
        Err(_) => None,
    }
}

fn profile_field_is_absent(error: &matrix_sdk::Error) -> bool {
    error.client_api_error_kind() == Some(&ErrorKind::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use serde_json::json;
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    fn empty_snapshot(generation: u64) -> NativeUserStatusSnapshot {
        NativeUserStatusSnapshot {
            session_generation: generation,
            user_id: "@alice:example.org".to_owned(),
            user_status: None,
            in_call: None,
        }
    }

    #[tokio::test]
    async fn static_snapshot_without_homeserver_stays_absent() {
        let owner = NativeUserStatusOwner::from_static_snapshot(7, empty_snapshot(7), false);
        let snapshot = owner
            .snapshot("@bob:example.org")
            .await
            .expect("static snapshot");
        assert_eq!(snapshot.session_generation, 7);
        assert_eq!(snapshot.user_id, "@bob:example.org");
        assert!(snapshot.user_status.is_none());
        assert!(snapshot.in_call.is_none());
    }

    #[tokio::test]
    async fn static_write_fails_closed_when_capability_missing() {
        let owner = NativeUserStatusOwner::from_static_snapshot(1, empty_snapshot(1), false);
        let error = owner.set("☕", "secret-status-text").await.unwrap_err();
        assert_eq!(error, "v-user-status-unsupported");
        assert!(!error.contains("secret-status-text"));
        assert!(!error.contains("☕"));
    }

    #[tokio::test]
    async fn static_write_ack_has_no_emoji_or_text() {
        let owner = NativeUserStatusOwner::from_static_snapshot(1, empty_snapshot(1), true);
        let ack = owner.set("☕", "in a meeting").await.expect("set");
        assert_eq!(ack.status, "ok");
        let raw = serde_json::to_string(&ack).expect("serialize");
        assert!(!raw.contains("☕"));
        assert!(!raw.contains("in a meeting"));
        assert!(!raw.contains("emoji"));
        assert!(!raw.contains("text"));
    }

    #[tokio::test]
    async fn mock_http_capabilities_missing_fails_closed() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "capabilities": {}
            })))
            .mount(server.server())
            .await;
        let owner = NativeUserStatusOwner::start(&client, 3);
        let error = owner.set("☕", "secret-status-text").await.unwrap_err();
        assert_eq!(error, "v-user-status-unsupported");
        let text = format!("{error:?}");
        assert!(!text.contains("secret-status-text"));
        assert!(!text.contains("☕"));
    }

    #[tokio::test]
    async fn mock_http_success_ack_has_no_emoji_or_text() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let user_id = client.user_id().expect("logged-in mock user").to_string();
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/capabilities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "capabilities": {
                    "m.profile_fields": { "enabled": true }
                }
            })))
            .mount(server.server())
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(server.server())
            .await;
        let owner = NativeUserStatusOwner::start(&client, 3);
        let ack = owner.set("☕", "in a meeting").await.expect("set");
        assert_eq!(ack.status, "ok");
        let raw = serde_json::to_string(&ack).expect("serialize");
        assert!(!raw.contains("☕"));
        assert!(!raw.contains("in a meeting"));
        assert!(!raw.contains(&user_id));
        assert!(!source_mentions_forbidden_call_writes());
    }

    fn source_mentions_forbidden_call_writes() -> bool {
        let source = include_str!("live.rs");
        source.contains("set_call(") || source.contains("enable_automatic_call_status")
    }
}
