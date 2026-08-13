//! Live V-ROOMS.5 `m.direct` Client load/store.
//!
//! Implementation lives in synara-core. This module keeps the desktop
//! `crate::matrix::account_data::live::*` path resolving.

pub use synara_core::app::account_data::{
    add_room_to_mdirect, remove_room_from_mdirect, snapshot_mdirect, NativeMDirectMutationResult,
    NativeMDirectSnapshot,
};
