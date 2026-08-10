//! P0.5 toolchain coexistence probe.
//!
//! Proves that `tauri` 2.11 and `matrix-sdk` / `matrix-sdk-ui` 0.18.0 can
//! type-check and link together under Rust 1.93 / edition 2024 in one crate.
//! This is not a product shell and does not add SDK deps to `src-tauri`.

use matrix_sdk::Client;
use matrix_sdk_ui::timeline::Timeline;
use tauri::{AppHandle, Wry};

/// Touch Tauri + matrix-sdk symbols so both dependency graphs must resolve.
pub fn coexistence_type_markers() -> &'static str {
    // Keep unused-type references without requiring runtime construction.
    let _tauri: Option<AppHandle<Wry>> = None;
    let _client: Option<Client> = None;
    let _timeline: Option<Timeline> = None;
    "tauri-2.11 + matrix-sdk-0.18.0 coexistence"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_compile() {
        assert!(coexistence_type_markers().contains("matrix-sdk"));
    }
}
