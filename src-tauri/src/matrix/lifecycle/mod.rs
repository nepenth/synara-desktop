//! Desktop native lifecycle vault adapters and shared persist/restore/wipe APIs.
//! Production logout, recovery, and owner teardown run through Core and native
//! auth command paths; the obsolete pure supervisor orchestration is retired.

mod session_material;

pub use session_material::KeyringSessionMaterialVault;

pub use synara_core::app::lifecycle::*;

#[cfg(test)]
mod tests;
