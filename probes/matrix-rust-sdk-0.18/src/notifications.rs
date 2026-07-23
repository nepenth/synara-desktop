//! Push rules / notification-settings compile-only API-shape probes.
//!
//! Compile-only API-shape probes; do not prove runtime/network semantics.

use matrix_sdk::notification_settings::{NotificationSettings, RoomNotificationMode};
use matrix_sdk::ruma::RoomId;
use matrix_sdk::ruma::push::Ruleset;
use matrix_sdk::{Account, Client};

/// P0.3b-notification-settings-type — `NotificationSettings` is a public type.
///
/// Source: `crates/matrix-sdk/src/notification_settings/mod.rs`
/// (`pub struct NotificationSettings`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_notification_settings_type() -> &'static str {
    std::any::type_name::<NotificationSettings>()
}

/// P0.3b-client-notification-settings — `Client::notification_settings`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs`
/// (`pub async fn notification_settings`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_client_notification_settings() {
    async fn _shape(client: &Client) -> NotificationSettings {
        client.notification_settings().await
    }
    let _ = _shape;
}

/// P0.3b-account-push-rules — `Account::push_rules`.
///
/// Source: `crates/matrix-sdk/src/account.rs` (`pub async fn push_rules`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_account_push_rules() {
    async fn _shape(account: &Account) -> matrix_sdk::Result<Ruleset> {
        account.push_rules().await
    }
    let _ = _shape;
}

/// P0.3b-notification-set-room-notification-mode —
/// `NotificationSettings::set_room_notification_mode`.
///
/// Source: `crates/matrix-sdk/src/notification_settings/mod.rs`
/// (`pub async fn set_room_notification_mode`).
///
/// Compile-only API-shape probe; does not prove runtime/network semantics.
pub fn probe_notification_set_room_notification_mode() {
    async fn _shape(
        settings: &NotificationSettings,
        room_id: &RoomId,
        mode: RoomNotificationMode,
    ) -> Result<(), matrix_sdk::NotificationSettingsError> {
        settings.set_room_notification_mode(room_id, mode).await
    }
    let _ = _shape;
}

/// Run every notification probe (compile-only; no network, stores, or secrets).
pub fn run_all() {
    let _ = probe_notification_settings_type();
    probe_client_notification_settings();
    probe_account_push_rules();
    probe_notification_set_room_notification_mode();
}
