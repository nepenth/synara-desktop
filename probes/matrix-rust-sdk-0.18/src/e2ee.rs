//! Devices / E2EE / verification / recovery compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.
//! Requires resolved `matrix-sdk` feature `e2e-encryption` (unified via
//! `matrix-sdk-ui` in this probe graph).

use matrix_sdk::Client;
use matrix_sdk::encryption::backups::Backups;
use matrix_sdk::encryption::identities::UserDevices;
use matrix_sdk::encryption::recovery::Recovery;
use matrix_sdk::encryption::verification::VerificationRequest;
use matrix_sdk::encryption::{CrossSigningStatus, Encryption};
use matrix_sdk::ruma::UserId;
use matrix_sdk::ruma::api::client::device::get_devices;

/// P0.3b-encryption-type — `matrix_sdk::encryption::Encryption`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs` (`pub struct Encryption`),
/// feature-gated by `e2e-encryption`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_encryption_type() -> &'static str {
    std::any::type_name::<Encryption>()
}

/// P0.3b-client-encryption — `Client::encryption() -> Encryption`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn encryption`),
/// feature-gated by `e2e-encryption`.
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_encryption() {
    fn _shape(client: &Client) -> Encryption {
        client.encryption()
    }
    let _ = _shape;
}

/// P0.3b-encryption-get-user-devices — `Encryption::get_user_devices`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs`
/// (`pub async fn get_user_devices`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_encryption_get_user_devices() {
    async fn _shape(
        encryption: &Encryption,
        user_id: &UserId,
    ) -> matrix_sdk::Result<UserDevices, matrix_sdk::Error> {
        encryption.get_user_devices(user_id).await
    }
    let _ = _shape;
}

/// P0.3b-encryption-cross-signing-status — `Encryption::cross_signing_status`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs`
/// (`pub async fn cross_signing_status`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_encryption_cross_signing_status() {
    async fn _shape(encryption: &Encryption) -> Option<CrossSigningStatus> {
        encryption.cross_signing_status().await
    }
    let _ = _shape;
}

/// P0.3b-encryption-recovery — `Encryption::recovery() -> Recovery`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs` (`pub fn recovery`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_encryption_recovery() {
    fn _shape(encryption: &Encryption) -> Recovery {
        encryption.recovery()
    }
    let _ = _shape;
}

/// P0.3b-encryption-backups — `Encryption::backups() -> Backups`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs` (`pub fn backups`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_encryption_backups() {
    fn _shape(encryption: &Encryption) -> Backups {
        encryption.backups()
    }
    let _ = _shape;
}

/// P0.3b-client-devices — `Client::devices`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub async fn devices`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_devices() {
    async fn _shape(client: &Client) -> matrix_sdk::HttpResult<get_devices::v3::Response> {
        client.devices().await
    }
    let _ = _shape;
}

/// P0.3b-encryption-get-verification-request — `Encryption::get_verification_request`.
///
/// Source: `crates/matrix-sdk/src/encryption/mod.rs`
/// (`pub async fn get_verification_request` → `Option<VerificationRequest>`).
///
/// Compile-only API-shape probe; does not prove runtime/network/SAS semantics.
pub fn probe_encryption_get_verification_request() {
    async fn _shape(
        encryption: &Encryption,
        user_id: &UserId,
        flow_id: &str,
    ) -> Option<VerificationRequest> {
        encryption.get_verification_request(user_id, flow_id).await
    }
    let _ = _shape;
}

/// Run every E2EE probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    let _ = probe_encryption_type();
    probe_client_encryption();
    probe_encryption_get_user_devices();
    probe_encryption_cross_signing_status();
    probe_encryption_recovery();
    probe_encryption_backups();
    probe_client_devices();
    probe_encryption_get_verification_request();
}
