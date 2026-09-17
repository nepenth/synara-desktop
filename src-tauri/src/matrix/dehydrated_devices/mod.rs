//! Desktop adapter for the Core MSC3814 dehydrated-device owner.

pub mod live;
pub use live::start as start_dehydrated_devices_owner;
pub use synara_core::app::dehydrated_devices::*;
