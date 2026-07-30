//! Live V-ROOMS.5 `m.direct` projection for DM nav filters.

use std::collections::BTreeSet;

use matrix_sdk::{ruma::events::direct::DirectEventContent, Client};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMDirectSnapshot {
    pub session_generation: u64,
    pub room_ids: Vec<String>,
}

pub async fn snapshot_mdirect(
    client: &Client,
    session_generation: u64,
) -> Result<NativeMDirectSnapshot, &'static str> {
    let raw = client
        .account()
        .account_data::<DirectEventContent>()
        .await
        .map_err(|_| "v-rooms.5-mdirect-fetch-failed")?;

    let mut room_ids = BTreeSet::new();
    if let Some(raw) = raw {
        let content = raw
            .deserialize()
            .map_err(|_| "v-rooms.5-mdirect-deserialize-failed")?;
        for rooms in content.0.values() {
            for room_id in rooms {
                room_ids.insert(room_id.to_string());
            }
        }
    }

    Ok(NativeMDirectSnapshot {
        session_generation,
        room_ids: room_ids.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeMDirectSnapshot {
            session_generation: 4,
            room_ids: vec!["!dm:example.org".into()],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 4);
        assert_eq!(value["roomIds"][0], "!dm:example.org");
    }
}
