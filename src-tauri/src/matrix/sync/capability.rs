//! Sliding-sync server capability detection (P1 preflight, 2026-08-10).
//!
//! `SyncService` (matrix-sdk-ui 0.18) requires a server that implements
//! MSC3575 / MSC4186 sliding sync; without it the request fails with a 404
//! `M_UNRECOGNIZED`. This module probes the server's advertised capabilities
//! (the standard client-versions endpoint) so the product can fail loudly — and
//! diagnose — BEFORE the opaque sync error, instead of after.
//!
//! Additive and parallel to the live sync path: nothing here changes how the
//! SyncService is built or started.

use std::collections::BTreeMap;
use std::time::Duration;

use matrix_sdk::Client;
use tokio::time::timeout;

/// Capability markers (any one suffices) a server may advertise for the
/// interface-based / sliding syncs the SDK's `SyncService` depends on.
const SLIDING_SYNC_MARKERS: &[&str] = &[
    "org.matrix.simplified_msc3575", // SDK 0.18 native / MSC4186 endpoint
    "org.matrix.msc3575",            // classic sliding-sync proxy feature
    "org.matrix.msc4186",            // legacy/server alias for interface-based sync
    "org.matrix.simplified_sliding_sync", // legacy/server alias
];

/// Best-effort probe of whether the configured homeserver supports the
/// sliding syncs required by `SyncService`.
///
/// Returns `None` when the probe itself cannot complete (network/parse/timeout
/// before a verdict) — the product should then proceed and report sync errors
/// normally rather than block on an unknown environment.
pub async fn probe_sliding_sync(client: &Client) -> Option<bool> {
    let probe = timeout(Duration::from_secs(3), client.fetch_server_versions(None)).await;
    let response = probe.ok()?.ok()?;
    Some(server_supports_sliding_sync(
        &response.versions,
        &response.unstable_features,
    ))
}

/// Pure decision: does the server advertise any sliding-sync capability marker?
pub fn server_supports_sliding_sync(
    versions: &[String],
    unstable_features: &BTreeMap<String, bool>,
) -> bool {
    let in_versions = versions
        .iter()
        .any(|v| SLIDING_SYNC_MARKERS.iter().any(|m| v == *m));
    let in_unstable = SLIDING_SYNC_MARKERS
        .iter()
        .any(|m| unstable_features.get(*m).copied().unwrap_or(false));
    in_versions || in_unstable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }
    fn unstable(pairs: &[(&str, bool)]) -> BTreeMap<String, bool> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn empty_server_response_is_not_supported() {
        assert!(!server_supports_sliding_sync(&[], &BTreeMap::new()));
    }

    #[test]
    fn unstable_feature_marker_enables_support() {
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.msc3575", true)])
        ));
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.msc4186", true)])
        ));
        // matrix-sdk 0.18's native SyncService uses
        // /unstable/org.matrix.simplified_msc3575/sync (MSC4186).
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.simplified_msc3575", true)])
        ));
    }

    #[test]
    fn versions_list_marker_enables_support() {
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11", "org.matrix.msc3575"]),
            &BTreeMap::new()
        ));
    }

    #[test]
    fn absent_or_false_markers_are_not_supported() {
        assert!(!server_supports_sliding_sync(
            &versions(&["v1.11", "v1.12"]),
            &unstable(&[("org.matrix.msc3575", false), ("org.matrix.thing", true)])
        ));
    }
}
