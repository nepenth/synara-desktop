//! P6.5 — Room profile / alias / directory / join-history / upgrade foundation.

pub use synara_core::app::room_profile::*;

pub mod live;
pub use live::start as start_join_rule_owner;
