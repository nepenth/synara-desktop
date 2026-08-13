//! Live `m.direct` Client load/store owned by the shared native core.

use std::collections::BTreeSet;

use matrix_sdk::{
    ruma::{
        events::direct::{DirectEventContent, OwnedDirectUserIdentifier},
        OwnedRoomId, UserId,
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

pub async fn add_room_to_mdirect(
    client: &Client,
    room_id: &str,
    user_id: &str,
) -> Result<NativeMDirectMutationResult, &'static str> {
    let room_id = parse_mdirect_room_id(room_id)?;
    let user_key = parse_direct_user(user_id)?;
    let mut content = fetch_mdirect_content(client).await?;
    apply_add_room(&mut content, &room_id, user_key);
    client
        .account()
        .set_account_data(content)
        .await
        .map_err(|_| "v-rooms.5-mdirect-set-failed")?;
    Ok(NativeMDirectMutationResult {
        room_id: room_id.to_string(),
        status: "updated",
    })
}

pub async fn remove_room_from_mdirect(
    client: &Client,
    room_id: &str,
) -> Result<NativeMDirectMutationResult, &'static str> {
    let room_id = parse_mdirect_room_id(room_id)?;
    let mut content = fetch_mdirect_content(client).await?;
    apply_remove_room(&mut content, &room_id);
    client
        .account()
        .set_account_data(content)
        .await
        .map_err(|_| "v-rooms.5-mdirect-set-failed")?;
    Ok(NativeMDirectMutationResult {
        room_id: room_id.to_string(),
        status: "updated",
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

fn parse_direct_user(user_id: &str) -> Result<OwnedDirectUserIdentifier, &'static str> {
    let user = UserId::parse(user_id.trim()).map_err(|_| "v-rooms.5-mdirect-invalid-user")?;
    Ok(user.into())
}

fn apply_add_room(
    content: &mut DirectEventContent,
    room_id: &OwnedRoomId,
    user_key: OwnedDirectUserIdentifier,
) {
    for (key, rooms) in content.iter_mut() {
        if key == &user_key {
            continue;
        }
        rooms.retain(|id| id != room_id);
    }
    let rooms = content.entry(user_key).or_default();
    if !rooms.iter().any(|id| id == room_id) {
        rooms.push(room_id.clone());
    }
}

fn apply_remove_room(content: &mut DirectEventContent, room_id: &OwnedRoomId) {
    for rooms in content.values_mut() {
        rooms.retain(|id| id != room_id);
    }
    content.retain(|_, rooms| !rooms.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::{owned_room_id, user_id};

    #[test]
    fn add_moves_room_to_single_user() {
        let alice: OwnedDirectUserIdentifier = user_id!("@alice:example.org").into();
        let bob: OwnedDirectUserIdentifier = user_id!("@bob:example.org").into();
        let room = owned_room_id!("!dm:example.org");
        let mut content = DirectEventContent::default();
        content.insert(bob.clone(), vec![room.clone()]);
        apply_add_room(&mut content, &room, alice.clone());
        assert_eq!(content.get(&alice), Some(&vec![room.clone()]));
        assert!(content.get(&bob).map(|r| r.is_empty()).unwrap_or(true));
    }

    #[test]
    fn remove_clears_room_from_all_users() {
        let alice: OwnedDirectUserIdentifier = user_id!("@alice:example.org").into();
        let room = owned_room_id!("!dm:example.org");
        let mut content = DirectEventContent::default();
        content.insert(
            alice.clone(),
            vec![room.clone(), owned_room_id!("!other:example.org")],
        );
        apply_remove_room(&mut content, &room);
        assert_eq!(
            content.get(&alice),
            Some(&vec![owned_room_id!("!other:example.org")])
        );
    }
}
