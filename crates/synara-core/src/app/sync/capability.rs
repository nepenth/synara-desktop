//! Sliding-sync server capability detection (P1 preflight, 2026-08-10).
//!
//! `SyncService` (matrix-sdk-ui 0.19) requires a server that implements
//! MSC4186 simplified sliding sync; without it the request fails with a 404
//! `M_UNRECOGNIZED`. This module probes the server's advertised capabilities
//! (the standard client-versions endpoint) so the product can fail loudly — and
//! diagnose — BEFORE the opaque sync error, instead of after.
//!
//! Additive and parallel to the live sync path: nothing here changes how the
//! SyncService is built or started. Sign-in uses the same rule to refuse
//! servers without it.

use std::collections::BTreeMap;
use std::time::Duration;

use matrix_sdk::ruma::api::SupportedVersions;
use matrix_sdk::sliding_sync::VersionBuilder as SlidingSyncVersionBuilder;
use matrix_sdk::Client;
use tokio::time::timeout;

/// Best-effort probe of whether the configured homeserver supports the
/// sliding syncs required by `SyncService`.
///
/// Returns `None` when the probe itself cannot complete (network/parse/timeout
/// before a verdict) — the product should then proceed and report sync errors
/// normally rather than block on an unknown environment.
pub async fn probe_sliding_sync(client: &Client) -> Option<bool> {
    let probe = timeout(
        Duration::from_secs(3),
        client.fetch_server_versions(Some(matrix_sdk::config::RequestConfig::new().skip_auth())),
    )
    .await;
    let response = probe.ok()?.ok()?;
    Some(server_supports_sliding_sync(
        &response.versions,
        &response.unstable_features,
    ))
}

/// Pure decision on raw `/versions` parts; see [`supports_native_sliding_sync`].
pub fn server_supports_sliding_sync(
    versions: &[String],
    unstable_features: &BTreeMap<String, bool>,
) -> bool {
    supports_native_sliding_sync(&SupportedVersions::from_parts(versions, unstable_features))
}

/// Whether the SDK would select native simplified sliding sync (MSC4186) for
/// this server. This is the SDK's own `DiscoverNative` rule: `/versions` must
/// enable `org.matrix.simplified_msc3575` in `unstable_features`. Proxy and
/// alias markers do not count because the SDK never syncs with them.
pub fn supports_native_sliding_sync(supported: &SupportedVersions) -> bool {
    SlidingSyncVersionBuilder::DiscoverNative
        .build(Some(supported))
        .is_ok()
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
    fn only_the_sdk_native_marker_enables_support() {
        // matrix-sdk 0.19's native SyncService uses
        // /unstable/org.matrix.simplified_msc3575/sync (MSC4186).
        assert!(server_supports_sliding_sync(
            &versions(&["v1.11"]),
            &unstable(&[("org.matrix.simplified_msc3575", true)])
        ));
        // Markers the SDK does not use would report "capable" for a server
        // whose sync endpoint then 404s.
        for marker in [
            "org.matrix.msc3575",
            "org.matrix.msc4186",
            "org.matrix.simplified_sliding_sync",
        ] {
            assert!(
                !server_supports_sliding_sync(&versions(&["v1.11"]), &unstable(&[(marker, true)])),
                "{marker} must not count as native sliding sync"
            );
        }
    }

    #[test]
    fn versions_list_entries_are_not_features() {
        assert!(!server_supports_sliding_sync(
            &versions(&["v1.11", "org.matrix.simplified_msc3575"]),
            &BTreeMap::new()
        ));
    }

    #[test]
    fn absent_or_false_markers_are_not_supported() {
        assert!(!server_supports_sliding_sync(
            &versions(&["v1.11", "v1.12"]),
            &unstable(&[
                ("org.matrix.simplified_msc3575", false),
                ("org.matrix.thing", true)
            ])
        ));
    }
}
