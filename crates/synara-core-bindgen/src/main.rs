//! Project-owned, lockfile-pinned UniFFI generator entry point (P4-1).
//!
//! This `xtask`-style package is deliberately separate from the synara-core
//! library API. The Apple generator invokes it from source rather than relying
//! on a globally installed or third-party prebuilt binding generator.

fn main() {
    uniffi::uniffi_bindgen_main();
}
