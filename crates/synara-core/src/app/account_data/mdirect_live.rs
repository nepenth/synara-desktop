//! Live `m.direct` Client load/store owned by the shared native core.

use std::collections::BTreeSet;

use matrix_sdk::{
    ruma::{
        events::direct::{DirectEventContent, OwnedDirectUserIdentifier},
        OwnedRoomId, OwnedUserId, UserId,
    },
    Client, RoomState,
};

use super::{
    snapshot_from_mdirect_rooms, MDirectRooms, NativeMDirectMutationResult, NativeMDirectSnapshot,
};

pub async fn snapshot_mdirect(
    client: &Client,
    session_generation: u64,
) -> Result<NativeMDirectSnapshot, &'static str> {
    let content = load_mdirect_content(client).await?;
    let rooms = mdirect_rooms_from_content(&content);
    let joined_rooms = joined_room_ids(client)
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    Ok(snapshot_from_mdirect_rooms(
        &rooms,
        &joined_rooms,
        session_generation,
    ))
}

/// Mark `room_id` as a DM with `user_id` through the SDK.
///
/// `Account::mark_as_dm` re-fetches `m.direct` and writes it under the SDK's
/// own lock, the one it also takes when it marks a created or joined DM, so
/// concurrent writes from this client cannot drop each other's entries. A
/// room is a DM for at most one user key (product parity): when another user
/// lists it, `Room::set_is_direct(false)` clears it first.
pub async fn add_room_to_mdirect(
    client: &Client,
    room_id: &str,
    user_id: &str,
) -> Result<NativeMDirectMutationResult, &'static str> {
    let room_id = parse_mdirect_room_id(room_id)?;
    let user_id = parse_direct_user(user_id)?;
    let user_key = OwnedDirectUserIdentifier::from(user_id.clone());
    let content = fetch_mdirect_content(client).await?;
    let listed_for_user = content
        .get(&user_key)
        .is_some_and(|rooms| rooms.contains(&room_id));
    let listed_for_others = content
        .iter()
        .any(|(key, rooms)| key != &user_key && rooms.contains(&room_id));
    if listed_for_others {
        client
            .get_room(&room_id)
            .ok_or("v-rooms.5-mdirect-room-not-found")?
            .set_is_direct(false)
            .await
            .map_err(|_| "v-rooms.5-mdirect-set-failed")?;
    }
    if !listed_for_user || listed_for_others {
        client
            .account()
            .mark_as_dm(&room_id, &[user_id])
            .await
            .map_err(|_| "v-rooms.5-mdirect-set-failed")?;
    }
    Ok(NativeMDirectMutationResult {
        room_id: room_id.to_string(),
        status: crate::dto::MutationStatus::Updated,
    })
}

/// Unmark `room_id` as a DM for every user, through `Room::set_is_direct`.
pub async fn remove_room_from_mdirect(
    client: &Client,
    room_id: &str,
) -> Result<NativeMDirectMutationResult, &'static str> {
    let room_id = parse_mdirect_room_id(room_id)?;
    client
        .get_room(&room_id)
        .ok_or("v-rooms.5-mdirect-room-not-found")?
        .set_is_direct(false)
        .await
        .map_err(|_| "v-rooms.5-mdirect-set-failed")?;
    Ok(NativeMDirectMutationResult {
        room_id: room_id.to_string(),
        status: crate::dto::MutationStatus::Updated,
    })
}

async fn load_mdirect_content(client: &Client) -> Result<DirectEventContent, &'static str> {
    let raw = client
        .account()
        .account_data::<DirectEventContent>()
        .await
        .map_err(|_| "v-rooms.5-mdirect-fetch-failed")?;
    Ok(match raw {
        Some(raw) => raw
            .deserialize()
            .map_err(|_| "v-rooms.5-mdirect-deserialize-failed")?,
        None => DirectEventContent::default(),
    })
}

async fn fetch_mdirect_content(client: &Client) -> Result<DirectEventContent, &'static str> {
    let raw = client
        .account()
        .fetch_account_data_static::<DirectEventContent>()
        .await
        .map_err(|_| "v-rooms.5-mdirect-fetch-failed")?;
    Ok(match raw {
        Some(raw) => raw
            .deserialize()
            .map_err(|_| "v-rooms.5-mdirect-deserialize-failed")?,
        None => DirectEventContent::default(),
    })
}

fn joined_room_ids(client: &Client) -> BTreeSet<OwnedRoomId> {
    client
        .joined_rooms()
        .into_iter()
        .filter(|room| room.state() == RoomState::Joined)
        .map(|room| room.room_id().to_owned())
        .collect()
}

fn mdirect_rooms_from_content(content: &DirectEventContent) -> MDirectRooms {
    content
        .iter()
        .map(|(user, rooms)| {
            (
                user.to_string(),
                rooms.iter().map(|id| id.to_string()).collect(),
            )
        })
        .collect()
}

fn parse_mdirect_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "v-rooms.5-mdirect-invalid-room")
}

fn parse_direct_user(user_id: &str) -> Result<OwnedUserId, &'static str> {
    UserId::parse(user_id.trim()).map_err(|_| "v-rooms.5-mdirect-invalid-user")
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use matrix_sdk_test::{event_factory::EventFactory, JoinedRoomBuilder};
    use ruma::{room_id, user_id};
    use wiremock::{
        matchers::{method, path},
        Mock, ResponseTemplate,
    };

    const DIRECT_PATH: &str = "/_matrix/client/v3/user/@example:localhost/account_data/m.direct";

    async fn mount_fetched_direct(server: &MatrixMockServer, content: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(DIRECT_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(content))
            .mount(server.server())
            .await;
    }

    async fn mount_direct_put(server: &MatrixMockServer) {
        Mock::given(method("PUT"))
            .and(path(DIRECT_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(server.server())
            .await;
    }

    async fn direct_puts(server: &MatrixMockServer) -> Vec<serde_json::Value> {
        server
            .server()
            .received_requests()
            .await
            .unwrap()
            .into_iter()
            .filter(|request| request.method.as_str() == "PUT" && request.url.path() == DIRECT_PATH)
            .map(|request| serde_json::from_slice(&request.body).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn add_keeps_other_dms_through_mark_as_dm() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        mount_fetched_direct(
            &server,
            serde_json::json!({"@carol:localhost": ["!other:localhost"]}),
        )
        .await;
        mount_direct_put(&server).await;

        add_room_to_mdirect(&client, "!dm:localhost", "@bob:localhost")
            .await
            .unwrap();

        assert_eq!(
            direct_puts(&server).await,
            [serde_json::json!({
                "@carol:localhost": ["!other:localhost"],
                "@bob:localhost": ["!dm:localhost"],
            })]
        );
    }

    #[tokio::test]
    async fn add_for_a_listed_user_writes_nothing() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        mount_fetched_direct(
            &server,
            serde_json::json!({"@bob:localhost": ["!dm:localhost"]}),
        )
        .await;
        mount_direct_put(&server).await;

        add_room_to_mdirect(&client, "!dm:localhost", "@bob:localhost")
            .await
            .unwrap();

        assert!(direct_puts(&server).await.is_empty());
    }

    #[tokio::test]
    async fn add_moves_a_room_listed_for_another_user() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let room_id = room_id!("!dm:localhost");
        let f = EventFactory::new();
        server
            .mock_sync()
            .ok_and_run(&client, |builder| {
                builder.add_joined_room(JoinedRoomBuilder::new(room_id));
                builder.add_global_account_data(
                    f.direct()
                        .add_user(user_id!("@carol:localhost").to_owned().into(), room_id),
                );
            })
            .await;
        mount_fetched_direct(
            &server,
            serde_json::json!({"@carol:localhost": ["!dm:localhost"]}),
        )
        .await;
        mount_direct_put(&server).await;

        add_room_to_mdirect(&client, room_id.as_str(), "@bob:localhost")
            .await
            .unwrap();

        let puts = direct_puts(&server).await;
        assert_eq!(puts.len(), 2);
        assert_eq!(puts[0], serde_json::json!({}), "cleared for carol first");
        assert_eq!(
            puts[1]["@bob:localhost"],
            serde_json::json!(["!dm:localhost"])
        );
    }

    #[tokio::test]
    async fn remove_clears_the_room_for_every_user_and_keeps_the_rest() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let room_id = room_id!("!dm:localhost");
        let f = EventFactory::new();
        server
            .mock_sync()
            .ok_and_run(&client, |builder| {
                builder.add_joined_room(JoinedRoomBuilder::new(room_id));
                builder.add_global_account_data(
                    f.direct()
                        .add_user(user_id!("@bob:localhost").to_owned().into(), room_id)
                        .add_user(
                            user_id!("@bob:localhost").to_owned().into(),
                            room_id!("!keep:localhost"),
                        ),
                );
            })
            .await;
        mount_direct_put(&server).await;

        remove_room_from_mdirect(&client, room_id.as_str())
            .await
            .unwrap();

        assert_eq!(
            direct_puts(&server).await,
            [serde_json::json!({"@bob:localhost": ["!keep:localhost"]})]
        );
    }

    #[tokio::test]
    async fn remove_for_an_unknown_room_fails_without_writing() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        mount_direct_put(&server).await;

        assert_eq!(
            remove_room_from_mdirect(&client, "!unknown:localhost")
                .await
                .unwrap_err(),
            "v-rooms.5-mdirect-room-not-found"
        );
        assert!(direct_puts(&server).await.is_empty());
    }
}
