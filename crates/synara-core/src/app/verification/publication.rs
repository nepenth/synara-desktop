//! Reconcile the requesting device's public keys before interactive verification.
//!
//! Synara retains the SDK crypto store across logout and reuses its device ID.
//! The homeserver removes the device keys at logout, but SDK 0.18's persisted
//! Account still has `shared = true`, so ordinary sync does not upload them
//! again. A session in `/devices` is therefore not evidence of an E2EE device.
//! Publish only the SDK's existing signed public device object, never private
//! keys, one-time keys, or a replacement identity. The SDK continues to own all
//! cryptographic generation, signing, trust and outgoing one-time-key traffic.

use matrix_sdk::{
    config::RequestConfig,
    ruma::api::client::keys::{get_keys, upload_keys},
    Client,
};
use matrix_sdk_crypto::types::DeviceKeys;
use std::time::Duration;

/// Returns only after the server has the same identity keys as the SDK store.
/// A malformed, failed, or conflicting lookup must never authorize an upload.
pub(super) async fn ensure_own_device_keys_published(client: &Client) -> Result<(), &'static str> {
    let device = client
        .encryption()
        .get_own_device()
        .await
        .map_err(|_| "v-crypto.1-own-device-store-failed")?
        .ok_or("v-crypto.1-own-device-unavailable")?;
    let keys = device.as_device_keys();
    if client.user_id() != Some(keys.user_id.as_ref())
        || client.device_id() != Some(keys.device_id.as_ref())
    {
        return Err("v-crypto.1-own-device-identity-mismatch");
    }
    if server_has_device_keys(client, keys).await? {
        return Ok(());
    }

    let mut upload = upload_keys::v3::Request::new();
    upload.device_keys = Some(keys.to_raw());
    client
        .send(upload)
        .with_request_config(publication_request_config())
        .await
        .map_err(|_| "v-crypto.1-own-device-publication-failed")?;
    if !server_has_device_keys(client, keys).await? {
        return Err("v-crypto.1-own-device-publication-missing");
    }
    Ok(())
}

fn publication_request_config() -> RequestConfig {
    RequestConfig::new()
        .timeout(Duration::from_secs(8))
        .retry_limit(0)
}

async fn server_has_device_keys(
    client: &Client,
    expected: &DeviceKeys,
) -> Result<bool, &'static str> {
    let mut query = get_keys::v3::Request::new();
    query
        .device_keys
        .insert(expected.user_id.clone(), vec![expected.device_id.clone()]);
    let response = client
        .send(query)
        .with_request_config(publication_request_config())
        .await
        .map_err(|_| "v-crypto.1-own-device-publication-query-failed")?;
    if !response.failures.is_empty() {
        return Err("v-crypto.1-own-device-publication-query-incomplete");
    }
    let Some(raw) = response
        .device_keys
        .get(&expected.user_id)
        .and_then(|devices| devices.get(&expected.device_id))
    else {
        return Ok(false);
    };
    let actual = raw
        .deserialize_as::<DeviceKeys>()
        .map_err(|_| "v-crypto.1-own-device-publication-invalid")?;
    if actual.user_id != expected.user_id
        || actual.device_id != expected.device_id
        || actual.keys != expected.keys
    {
        return Err("v-crypto.1-own-device-publication-conflict");
    }
    actual
        .check_self_signature()
        .map_err(|_| "v-crypto.1-own-device-publication-invalid")?;
    Ok(true)
}
