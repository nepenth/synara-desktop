//! P6.5 — Room profile / alias / directory / join-history / upgrade foundation.
#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::room_profile::*;

pub mod live;
pub use live::{
    project_join_rule, NativeRoomJoinRuleOwner, NativeRoomJoinRuleUpdate,
    ROOM_JOIN_RULE_UPDATED_EVENT,
};
