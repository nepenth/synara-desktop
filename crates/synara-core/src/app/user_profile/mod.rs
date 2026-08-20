//! P6.6 — User profile / ignore list foundation (harness).
//!
//! Pure projection of own + peer profiles and ignore list. No avatar bytes,
//! no SDK profile writes, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.6-user-profile.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod ipc;
mod live;

pub use error::UserProfileError;
pub use index::{
    UserProfile, UserProfileIndex, MAX_AVATAR_URL_CHARS, MAX_CACHED_PROFILES,
    MAX_DISPLAY_NAME_CHARS, MAX_IGNORED_USERS,
};
pub use ipc::{MatrixOwnProfile, MatrixProfileWriteResult};
pub use live::{
    get_own_profile, parse_own_avatar_mxc, parse_own_display_name, set_own_avatar,
    set_own_display_name,
};

/// Static marker for link / schema smoke.
pub const MATRIX_USER_PROFILE_MARKER: &str = "matrix-user-profile-p6.6";

/// Touch user-profile paths so they remain linked in non-test builds.
pub fn matrix_user_profile_markers() -> &'static str {
    let idx = UserProfileIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(MAX_CACHED_PROFILES, 512);
    debug_assert_eq!(MAX_IGNORED_USERS, 1024);
    debug_assert_eq!(MATRIX_USER_PROFILE_MARKER, "matrix-user-profile-p6.6");
    MATRIX_USER_PROFILE_MARKER
}

#[cfg(test)]
mod tests;
