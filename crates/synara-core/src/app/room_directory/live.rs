//! Live public-directory protocol listing from the managed Matrix client.

use std::collections::BTreeMap;

use matrix_sdk::{
    ruma::{api::client::thirdparty::get_protocols, thirdparty::Protocol},
    Client,
};

use super::{
    DirectoryProtocolInstance, NativeRoomDirectoryProtocols, MAX_PROTOCOL_INSTANCES, MAX_TEXT_CHARS,
};

pub async fn fetch_protocols(
    client: &Client,
    session_generation: u64,
) -> Result<NativeRoomDirectoryProtocols, &'static str> {
    let response = client
        .send(get_protocols::v3::Request::new())
        .await
        .map_err(|_| "v-rooms.directory-protocols-sdk-failed")?;
    Ok(NativeRoomDirectoryProtocols {
        session_generation,
        instances: project_protocols(response.protocols)?,
    })
}

pub fn project_protocols(
    protocols: BTreeMap<String, Protocol>,
) -> Result<Vec<DirectoryProtocolInstance>, &'static str> {
    let mut instances = Vec::new();
    for (protocol_id, protocol) in protocols {
        if protocol_id.trim().is_empty() || protocol_id.chars().count() > MAX_TEXT_CHARS {
            return Err("v-rooms.directory-protocol-id-cap");
        }
        for instance in protocol.instances {
            let Some(instance_id) = instance.instance_id else {
                continue;
            };
            if instance_id.trim().is_empty()
                || instance_id.trim() != instance_id
                || instance_id.chars().count() > MAX_TEXT_CHARS
                || instance.desc.trim().is_empty()
                || instance.desc.chars().count() > MAX_TEXT_CHARS
            {
                return Err("v-rooms.directory-protocol-instance-invalid");
            }
            instances.push(DirectoryProtocolInstance {
                protocol_id: protocol_id.clone(),
                instance_id,
                description: instance.desc,
            });
            if instances.len() > MAX_PROTOCOL_INSTANCES {
                return Err("v-rooms.directory-protocol-instance-cap");
            }
        }
    }
    Ok(instances)
}
