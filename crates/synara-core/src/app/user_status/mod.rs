//! MSC4426 user status (`m.status`) and in-call (`m.call`) projection.
//!
//! Own writes go through `Account::set_status` / `clear_status` after a
//! capability check. This module never calls `set_call` / `clear_call` and
//! never enables `Client::enable_automatic_call_status`.
//!
//! Reads use `Account::fetch_profile_field_of` for `Status` and `Call`.
//! Missing fields stay absent. This is not `m.presence`.

mod live;
mod native;

pub use live::NativeUserStatusOwner;
pub use native::{
    parse_status_write, NativeInCall, NativeUserStatus, NativeUserStatusSnapshot,
    NativeUserStatusWriteResult, StatusWrite, MAX_STATUS_EMOJI_BYTES, MAX_STATUS_TEXT_BYTES,
    USER_STATUS_MARKER,
};

/// Touch MSC4426 paths so they remain linked in non-test builds.
pub fn matrix_user_status_markers() -> &'static str {
    debug_assert_eq!(USER_STATUS_MARKER, "matrix-user-status-msc4426");
    debug_assert_eq!(MAX_STATUS_EMOJI_BYTES, 32);
    debug_assert_eq!(MAX_STATUS_TEXT_BYTES, 256);
    USER_STATUS_MARKER
}
