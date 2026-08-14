//! Minimal UniFFI construction facade for the shared Core.
//!
//! P4-S2 intentionally exposes only construction. It installs the Rust-owned
//! [`IosFailClosedPlatform`] and retains the resulting [`Core`], but exposes
//! neither a Platform callback nor any session, attachment, credential, or
//! command API. P4-S3 will design live iOS ownership separately.

use std::sync::Arc;

use crate::core::Core;
use crate::platform::IosFailClosedPlatform;

/// Retained shared Core for the iOS UniFFI boundary.
///
/// The held core is intentionally private: Swift can prove construction, but
/// cannot dispatch commands or mutate session/owner state in P4-S2.
pub struct SharedCore {
    core: Core,
}

impl SharedCore {
    /// Construct a real Core with the P4-S2 iOS fail-closed Platform.
    pub fn new() -> Self {
        Self {
            core: Core::new(Arc::new(IosFailClosedPlatform)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_core_constructs_and_retains_the_built_in_core() {
        let shared_core = SharedCore::new();

        assert!(
            !shared_core.core.registered_commands().is_empty(),
            "P4-S2 must retain a real Core with its built-in registry"
        );
    }
}
