//! Stable typed-request and stable gap API-shape probes (no experimental feature).
//!
//! Profile: `profile-stable-typed`.
//! Compile-only; does not prove runtime/network/store/UI semantics.

use matrix_sdk::authentication::matrix::MatrixAuth;
use matrix_sdk::authentication::oauth::OAuth;
use matrix_sdk::config::RequestConfig;
use matrix_sdk::encryption::Encryption;
use matrix_sdk::encryption::backups::Backups;
use matrix_sdk::encryption::identities::Device;
use matrix_sdk::encryption::recovery::Recovery;
use matrix_sdk::encryption::verification::{SasVerification, VerificationRequest};
use matrix_sdk::room::edit::EditedContent;
use matrix_sdk::ruma::api::client::delayed_events::delayed_message_event;
use matrix_sdk::ruma::api::client::filter::RoomEventFilter;
use matrix_sdk::ruma::api::client::receipt::create_receipt;
use matrix_sdk::ruma::api::client::search::search_events;
use matrix_sdk::ruma::api::client::uiaa;
use matrix_sdk::ruma::events::poll::unstable_start::{
    UnstablePollStartContentBlock, UnstablePollStartEventContent,
};
use matrix_sdk::ruma::events::receipt::ReceiptThread;
use matrix_sdk::ruma::events::{
    AnyGlobalAccountDataEventContent, AnyMessageLikeEventContent, GlobalAccountDataEventType,
};
use matrix_sdk::ruma::serde::{JsonObject, Raw};
use matrix_sdk::ruma::{DeviceId, EventId, OwnedDeviceId, OwnedEventId, OwnedRoomId, UInt, UserId};
use matrix_sdk::utils::UrlOrQuery;
use matrix_sdk::{Account, Client, Room};
use matrix_sdk_ui::timeline::PollState;
use serde_json::json;

/// Probe IDs compiled under `profile-stable-typed`.
pub const PROBE_IDS: &[&str] = &[
    "P0.3c-typed-search-request-type",
    "P0.3c-typed-search-response-type",
    "P0.3c-typed-search-criteria-type",
    "P0.3c-typed-search-client-send",
    "P0.3c-typed-search-next-batch-field",
    "P0.3c-typed-search-filter-limit-fields",
    "P0.3c-oauth-type",
    "P0.3c-client-oauth",
    "P0.3c-oauth-finish-login",
    "P0.3c-oauth-refresh-access-token",
    "P0.3c-client-refresh-access-token",
    "P0.3c-matrix-auth-get-sso-login-url",
    "P0.3c-matrix-auth-login-with-sso-callback",
    "P0.3c-matrix-auth-login-token",
    "P0.3c-matrix-auth-login-custom",
    "P0.3c-matrix-auth-refresh-access-token",
    "P0.3c-client-delete-devices",
    "P0.3c-client-rename-device",
    "P0.3c-verification-request-accept",
    "P0.3c-verification-request-cancel",
    "P0.3c-verification-request-start-sas",
    "P0.3c-sas-verification-accept",
    "P0.3c-sas-verification-confirm",
    "P0.3c-encryption-get-verification-request",
    "P0.3c-device-request-verification",
    "P0.3c-recovery-enable",
    "P0.3c-recovery-recover",
    "P0.3c-backups-create",
    "P0.3c-room-send-raw",
    "P0.3c-room-send-state-event-raw",
    "P0.3c-account-account-data-raw",
    "P0.3c-account-set-account-data-raw",
    "P0.3c-room-set-unread-flag",
    "P0.3c-room-send-single-receipt",
    "P0.3c-room-event-with-context",
    "P0.3c-client-rtc-foci",
    "P0.3c-room-has-active-room-call",
    "P0.3c-room-send-message-like",
    "P0.3c-unstable-poll-start-event-content-type",
    "P0.3c-timeline-poll-state-type",
    "P0.3c-edited-content-poll-start-variant",
    "P0.3c-typed-delayed-message-event-request",
];

// ---------------------------------------------------------------------------
// SC-071 — typed server-side room-message search via Client::send
// ---------------------------------------------------------------------------

/// P0.3c-typed-search-request-type
pub fn probe_typed_search_request_type() -> &'static str {
    std::any::type_name::<search_events::v3::Request>()
}

/// P0.3c-typed-search-response-type
pub fn probe_typed_search_response_type() -> &'static str {
    std::any::type_name::<search_events::v3::Response>()
}

/// P0.3c-typed-search-criteria-type
pub fn probe_typed_search_criteria_type() -> &'static str {
    std::any::type_name::<search_events::v3::Criteria>()
}

/// P0.3c-typed-search-client-send — public Ruma request through `Client::send`.
///
/// Source: `crates/matrix-sdk/src/client/mod.rs` (`pub fn send`).
/// Never uses raw URLs or hand-written Matrix HTTP.
pub fn probe_typed_search_client_send() {
    async fn _shape(client: &Client, request: search_events::v3::Request) {
        let _response: search_events::v3::Response = client.send(request).await.expect("shape");
    }
    let _ = _shape;
}

/// P0.3c-typed-search-next-batch-field — pagination token on the request.
pub fn probe_typed_search_next_batch_field() {
    fn _shape(mut request: search_events::v3::Request, token: Option<String>) {
        request.next_batch = token;
        let _ = request.next_batch;
    }
    let _ = _shape;
}

/// P0.3c-typed-search-filter-limit-fields — filters and limits on Criteria.
///
/// Result limit is `Criteria.filter.limit` (`RoomEventFilter::limit`).
/// Room scoping is `Criteria.filter.rooms`.
pub fn probe_typed_search_filter_limit_fields() {
    fn _shape(search_term: String, limit: Option<UInt>, rooms: Option<Vec<OwnedRoomId>>) {
        let mut criteria = search_events::v3::Criteria::new(search_term);
        // RoomEventFilter is non_exhaustive; mutate fields after Default.
        let mut filter = RoomEventFilter::default();
        filter.limit = limit;
        filter.rooms = rooms;
        criteria.filter = filter;
        let mut categories = search_events::v3::Categories::new();
        categories.room_events = Some(criteria);
        let _request = search_events::v3::Request::new(categories);
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// SSO / OAuth callback and token refresh
// ---------------------------------------------------------------------------

/// P0.3c-oauth-type
pub fn probe_oauth_type() -> &'static str {
    std::any::type_name::<OAuth>()
}

/// P0.3c-client-oauth
pub fn probe_client_oauth() {
    fn _shape(client: &Client) -> OAuth {
        client.oauth()
    }
    let _ = _shape;
}

/// P0.3c-oauth-finish-login
pub fn probe_oauth_finish_login() {
    async fn _shape(oauth: &OAuth, url_or_query: UrlOrQuery) {
        let _ = oauth.finish_login(url_or_query).await;
    }
    let _ = _shape;
}

/// P0.3c-oauth-refresh-access-token
pub fn probe_oauth_refresh_access_token() {
    async fn _shape(oauth: &OAuth) {
        let _ = oauth.refresh_access_token().await;
    }
    let _ = _shape;
}

/// P0.3c-client-refresh-access-token
pub fn probe_client_refresh_access_token() {
    async fn _shape(client: &Client) {
        let _ = client.refresh_access_token().await;
    }
    let _ = _shape;
}

/// P0.3c-matrix-auth-get-sso-login-url
///
/// Stable (not gated by `sso-login`); `sso-login` only gates local-server SSO helper.
pub fn probe_matrix_auth_get_sso_login_url() {
    async fn _shape(auth: &MatrixAuth, redirect_url: &str, idp_id: Option<&str>) {
        let _ = auth.get_sso_login_url(redirect_url, idp_id).await;
    }
    let _ = _shape;
}

/// P0.3c-matrix-auth-login-with-sso-callback
pub fn probe_matrix_auth_login_with_sso_callback() {
    fn _shape(auth: &MatrixAuth, url_or_query: UrlOrQuery) {
        let _ = auth.login_with_sso_callback(url_or_query);
    }
    let _ = _shape;
}

/// P0.3c-matrix-auth-login-token — SSO callback residual login with loginToken.
///
/// Source: `crates/matrix-sdk/src/authentication/matrix/mod.rs` (`pub fn login_token`).
pub fn probe_matrix_auth_login_token() {
    fn _shape(auth: &MatrixAuth, token: &str) -> matrix_sdk::authentication::matrix::LoginBuilder {
        auth.login_token(token)
    }
    let _ = _shape;
}

/// P0.3c-matrix-auth-login-custom — custom/UIA-adjacent login type entry.
///
/// Source: `crates/matrix-sdk/src/authentication/matrix/mod.rs` (`pub fn login_custom`).
pub fn probe_matrix_auth_login_custom() {
    fn _shape(
        auth: &MatrixAuth,
        login_type: &str,
        data: JsonObject,
    ) -> serde_json::Result<matrix_sdk::authentication::matrix::LoginBuilder> {
        auth.login_custom(login_type, data)
    }
    let _ = _shape;
}

/// P0.3c-matrix-auth-refresh-access-token
pub fn probe_matrix_auth_refresh_access_token() {
    async fn _shape(auth: &MatrixAuth) {
        let _ = auth.refresh_access_token().await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Device naming / deletion (UIA on delete)
// ---------------------------------------------------------------------------

/// P0.3c-client-delete-devices — UIA via optional `uiaa::AuthData`.
pub fn probe_client_delete_devices() {
    async fn _shape(client: &Client, devices: &[OwnedDeviceId], auth_data: Option<uiaa::AuthData>) {
        let _ = client.delete_devices(devices, auth_data).await;
    }
    let _ = _shape;
}

/// P0.3c-client-rename-device
pub fn probe_client_rename_device() {
    async fn _shape(client: &Client, device_id: &DeviceId, display_name: &str) {
        let _ = client.rename_device(device_id, display_name).await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// SAS verification accept/cancel/confirm (+ lookup / request entry)
// ---------------------------------------------------------------------------

/// P0.3c-verification-request-accept
pub fn probe_verification_request_accept() {
    async fn _shape(req: &VerificationRequest) {
        let _ = req.accept().await;
    }
    let _ = _shape;
}

/// P0.3c-verification-request-cancel
pub fn probe_verification_request_cancel() {
    async fn _shape(req: &VerificationRequest) {
        let _ = req.cancel().await;
    }
    let _ = _shape;
}

/// P0.3c-verification-request-start-sas
pub fn probe_verification_request_start_sas() {
    async fn _shape(req: &VerificationRequest) {
        let _: Option<SasVerification> = req.start_sas().await.expect("shape");
    }
    let _ = _shape;
}

/// P0.3c-sas-verification-accept
pub fn probe_sas_verification_accept() {
    async fn _shape(sas: &SasVerification) {
        let _ = sas.accept().await;
    }
    let _ = _shape;
}

/// P0.3c-sas-verification-confirm
pub fn probe_sas_verification_confirm() {
    async fn _shape(sas: &SasVerification) {
        let _ = sas.confirm().await;
    }
    let _ = _shape;
}

/// P0.3c-encryption-get-verification-request — lookup by user + flow (not a push inbox stream).
pub fn probe_encryption_get_verification_request() {
    async fn _shape(encryption: &Encryption, user_id: &UserId, flow_id: &str) {
        let _: Option<VerificationRequest> =
            encryption.get_verification_request(user_id, flow_id).await;
    }
    let _ = _shape;
}

/// P0.3c-device-request-verification — outbound verification request entry.
pub fn probe_device_request_verification() {
    async fn _shape(device: &Device) {
        let _: VerificationRequest = device.request_verification().await.expect("shape");
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Recovery / backup lifecycle entry points
// ---------------------------------------------------------------------------

/// P0.3c-recovery-enable
pub fn probe_recovery_enable() {
    fn _shape(recovery: &Recovery) {
        let _ = recovery.enable();
    }
    let _ = _shape;
}

/// P0.3c-recovery-recover
pub fn probe_recovery_recover() {
    async fn _shape(recovery: &Recovery, recovery_key: &str) {
        let _ = recovery.recover(recovery_key).await;
    }
    let _ = _shape;
}

/// P0.3c-backups-create
pub fn probe_backups_create() {
    async fn _shape(backups: &Backups) {
        let _ = backups.create().await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Custom raw account / state / message events
// ---------------------------------------------------------------------------

/// P0.3c-room-send-raw
pub fn probe_room_send_raw() {
    fn _shape(room: &Room, event_type: &str) {
        let _ = room.send_raw(event_type, json!({ "body": "shape-only" }));
    }
    let _ = _shape;
}

/// P0.3c-room-send-state-event-raw
pub fn probe_room_send_state_event_raw() {
    async fn _shape(room: &Room, event_type: &str, state_key: &str) {
        let _ = room
            .send_state_event_raw(event_type, state_key, json!({ "shape": true }))
            .await;
    }
    let _ = _shape;
}

/// P0.3c-account-account-data-raw
pub fn probe_account_account_data_raw() {
    async fn _shape(account: &Account, event_type: GlobalAccountDataEventType) {
        let _: Option<Raw<AnyGlobalAccountDataEventContent>> =
            account.account_data_raw(event_type).await.expect("shape");
    }
    let _ = _shape;
}

/// P0.3c-account-set-account-data-raw
pub fn probe_account_set_account_data_raw() {
    async fn _shape(
        account: &Account,
        event_type: GlobalAccountDataEventType,
        content: Raw<AnyGlobalAccountDataEventContent>,
    ) {
        let _ = account.set_account_data_raw(event_type, content).await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Read markers / marked-unread
// ---------------------------------------------------------------------------

/// P0.3c-room-set-unread-flag
pub fn probe_room_set_unread_flag() {
    async fn _shape(room: &Room, unread: bool) {
        let _ = room.set_unread_flag(unread).await;
    }
    let _ = _shape;
}

/// P0.3c-room-send-single-receipt — includes FullyRead read-marker receipt type.
pub fn probe_room_send_single_receipt() {
    async fn _shape(
        room: &Room,
        receipt_type: create_receipt::v3::ReceiptType,
        thread: ReceiptThread,
        event_id: OwnedEventId,
    ) {
        let _ = room
            .send_single_receipt(receipt_type, thread, event_id)
            .await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Event context navigation
// ---------------------------------------------------------------------------

/// P0.3c-room-event-with-context
pub fn probe_room_event_with_context() {
    async fn _shape(
        room: &Room,
        event_id: &EventId,
        lazy_load_members: bool,
        context_size: UInt,
        request_config: Option<RequestConfig>,
    ) {
        let _ = room
            .event_with_context(event_id, lazy_load_members, context_size, request_config)
            .await;
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// MatrixRTC membership entry (not full key exchange)
// ---------------------------------------------------------------------------

/// P0.3c-client-rtc-foci — well-known RTC foci discovery.
pub fn probe_client_rtc_foci() {
    async fn _shape(client: &Client) {
        let _ = client.rtc_foci().await;
    }
    let _ = _shape;
}

/// P0.3c-room-has-active-room-call — membership presence via BaseRoom Deref.
pub fn probe_room_has_active_room_call() {
    fn _shape(room: &Room) -> bool {
        room.has_active_room_call()
    }
    let _ = _shape;
}

// ---------------------------------------------------------------------------
// Polls via message-like send entry (content-type path; not a dedicated poll API)
// ---------------------------------------------------------------------------

/// P0.3c-room-send-message-like — polls use `Room::send` with poll event content.
pub fn probe_room_send_message_like() {
    fn _shape(room: &Room, content: AnyMessageLikeEventContent) {
        let _ = room.send(content);
    }
    let _ = _shape;
}

/// P0.3c-unstable-poll-start-event-content-type — ruma poll start content type.
///
/// Source: ruma events poll unstable_start (re-exported via matrix_sdk::ruma).
pub fn probe_unstable_poll_start_event_content_type() -> &'static str {
    std::any::type_name::<UnstablePollStartEventContent>()
}

/// P0.3c-timeline-poll-state-type — UI timeline poll aggregation type.
///
/// Source: `crates/matrix-sdk-ui/src/timeline/event_item/content/polls.rs` (`PollState`).
pub fn probe_timeline_poll_state_type() -> &'static str {
    std::any::type_name::<PollState>()
}

/// P0.3c-edited-content-poll-start-variant — room edit path for poll start.
///
/// Source: `crates/matrix-sdk/src/room/edit.rs` (`EditedContent::PollStart`).
pub fn probe_edited_content_poll_start_variant() {
    fn _shape(fallback_text: String, new_content: UnstablePollStartContentBlock) -> EditedContent {
        EditedContent::PollStart {
            fallback_text,
            new_content,
        }
    }
    let _ = _shape;
    let _ = std::any::type_name::<EditedContent>();
}

// ---------------------------------------------------------------------------
// Delayed events — typed residual path (no high-level Room delayed API)
// ---------------------------------------------------------------------------

/// P0.3c-typed-delayed-message-event-request
///
/// High-level Room delayed-event helpers are not public outside experimental
/// widgets; residual route is typed Ruma request through `Client::send`.
pub fn probe_typed_delayed_message_event_request() {
    async fn _shape(client: &Client, request: delayed_message_event::unstable::Request) {
        let _response: delayed_message_event::unstable::Response =
            client.send(request).await.expect("shape");
    }
    let _ = _shape;
}

/// Run every stable-typed probe (compile-only).
pub fn run_all() {
    let _ = probe_typed_search_request_type();
    let _ = probe_typed_search_response_type();
    let _ = probe_typed_search_criteria_type();
    probe_typed_search_client_send();
    probe_typed_search_next_batch_field();
    probe_typed_search_filter_limit_fields();
    let _ = probe_oauth_type();
    probe_client_oauth();
    probe_oauth_finish_login();
    probe_oauth_refresh_access_token();
    probe_client_refresh_access_token();
    probe_matrix_auth_get_sso_login_url();
    probe_matrix_auth_login_with_sso_callback();
    probe_matrix_auth_login_token();
    probe_matrix_auth_login_custom();
    probe_matrix_auth_refresh_access_token();
    probe_client_delete_devices();
    probe_client_rename_device();
    probe_verification_request_accept();
    probe_verification_request_cancel();
    probe_verification_request_start_sas();
    probe_sas_verification_accept();
    probe_sas_verification_confirm();
    probe_encryption_get_verification_request();
    probe_device_request_verification();
    probe_recovery_enable();
    probe_recovery_recover();
    probe_backups_create();
    probe_room_send_raw();
    probe_room_send_state_event_raw();
    probe_account_account_data_raw();
    probe_account_set_account_data_raw();
    probe_room_set_unread_flag();
    probe_room_send_single_receipt();
    probe_room_event_with_context();
    probe_client_rtc_foci();
    probe_room_has_active_room_call();
    probe_room_send_message_like();
    let _ = probe_unstable_poll_start_event_content_type();
    let _ = probe_timeline_poll_state_type();
    probe_edited_content_poll_start_variant();
    probe_typed_delayed_message_event_request();
}
