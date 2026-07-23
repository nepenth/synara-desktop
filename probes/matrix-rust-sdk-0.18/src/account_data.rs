//! Account-data compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::Account;
use matrix_sdk::ruma::api::client::config::set_global_account_data;
use matrix_sdk::ruma::events::{AnyGlobalAccountDataEventContent, GlobalAccountDataEventType};
use matrix_sdk::ruma::serde::Raw;

/// P0.3b-account-account-data-raw — `Account::account_data_raw`.
///
/// Source: `crates/matrix-sdk/src/account.rs` (`pub async fn account_data_raw`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_account_account_data_raw() {
    async fn _shape(
        account: &Account,
        event_type: GlobalAccountDataEventType,
    ) -> matrix_sdk::Result<Option<Raw<AnyGlobalAccountDataEventContent>>> {
        account.account_data_raw(event_type).await
    }
    let _ = _shape;
}

/// P0.3b-account-set-account-data-raw — `Account::set_account_data_raw`.
///
/// Source: `crates/matrix-sdk/src/account.rs` (`pub async fn set_account_data_raw`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_account_set_account_data_raw() {
    async fn _shape(
        account: &Account,
        event_type: GlobalAccountDataEventType,
        content: Raw<AnyGlobalAccountDataEventContent>,
    ) -> matrix_sdk::Result<set_global_account_data::v3::Response> {
        account.set_account_data_raw(event_type, content).await
    }
    let _ = _shape;
}

/// Run every account-data probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    probe_account_account_data_raw();
    probe_account_set_account_data_raw();
}
