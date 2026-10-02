//! P6.6 — User profile / ignore list foundation (harness).
//!
//! Pure projection of own + peer profiles and ignore list. No avatar bytes,
//! no SDK profile writes, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.6-user-profile.md`

mod error;
mod index;
mod ipc;
mod live;
mod own_profile;

pub use error::UserProfileError;
pub use index::{
    UserProfile, UserProfileIndex, MAX_AVATAR_URL_CHARS, MAX_CACHED_PROFILES,
    MAX_DISPLAY_NAME_CHARS, MAX_IGNORED_USERS,
};
pub use ipc::{
    MatrixIgnoredUsersSnapshot, MatrixIgnoredUsersWriteResult, MatrixOwnProfile,
    MatrixProfileWriteResult, MatrixThreepidAddResult, MatrixThreepidEmail,
    MatrixThreepidEmailTokenResult, MatrixThreepidSnapshot, MatrixThreepidWriteResult,
    MatrixUploadAvatarResult, MatrixUserDirectoryHit, MatrixUserDirectorySearchResult,
};
pub use live::{
    add_threepid_email, add_threepid_email_password, delete_threepid_email, get_own_profile,
    ignore_user, parse_avatar_upload_mime, parse_own_avatar_mxc, parse_own_display_name,
    parse_user_directory_limit, parse_user_directory_term, request_threepid_email_token,
    search_user_directory, set_own_avatar, set_own_display_name, snapshot_ignored_users,
    snapshot_threepids, unignore_user, upload_avatar, PendingThreepid,
    DEFAULT_USER_DIRECTORY_LIMIT, MAX_AVATAR_UPLOAD_BYTES, MAX_USER_DIRECTORY_LIMIT,
    MAX_USER_DIRECTORY_TERM_CHARS,
};
pub use own_profile::{
    map_own_profile_stream_fields, NativeOwnProfileOwner, OwnProfileStreamMap,
    OwnProfileUpdateEmit, OWN_PROFILE_CHANGED_EVENT,
};

#[cfg(test)]
mod tests;
