//! P6.7 account-data foundation + V-ROOMS.5 live `m.direct` ownership.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.7-account-data.md`
//! Product vertical: `docs/matrix-rust-sdk/v-rooms-5-mdirect.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
pub mod live;

pub use error::AccountDataError;
pub use index::{
    AccountDataEntry, AccountDataIndex, MAX_CONTENT_FIELDS, MAX_GLOBAL_TYPES, MAX_KEY_LEN,
    MAX_ROOMS_WITH_ACCOUNT_DATA, MAX_ROOM_TYPES, MAX_VALUE_LEN, TYPE_DIRECT, TYPE_FULLY_READ,
    TYPE_IGNORED_USER_LIST, TYPE_PUSH_RULES, TYPE_TAG,
};
pub use live::{snapshot_mdirect, NativeMDirectSnapshot};

/// Static marker for link / schema smoke.
pub const MATRIX_ACCOUNT_DATA_MARKER: &str = "matrix-account-data-p6.7";

/// Touch account-data paths so they remain linked in non-test builds.
pub fn matrix_account_data_markers() -> &'static str {
    let idx = AccountDataIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(TYPE_FULLY_READ, "m.fully_read");
    debug_assert_eq!(MATRIX_ACCOUNT_DATA_MARKER, "matrix-account-data-p6.7");
    MATRIX_ACCOUNT_DATA_MARKER
}

#[cfg(test)]
mod tests;
