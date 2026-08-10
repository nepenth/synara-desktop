//! P0.3c feature-gate, typed-request, experimental-surface, and gap probes.
//!
//! Compile-only API-shape evidence against matrix-sdk / matrix-sdk-ui **0.18.0**.
//! Upstream pin: tag `matrix-sdk-0.18.0`, commit
//! `1c44fb66214667c6d00acaf72ab592493653708b`.
//!
//! **These probes do not prove runtime, network, store, or UI semantics.**
//! They never construct a live client that performs I/O, open a store, or
//! handle secrets.
//!
//! Enable exactly one local `profile-*` feature per validation run.

#![forbid(unsafe_code)]
#![deny(unreachable_pub)]

pub mod catalog;

#[cfg(feature = "profile-stable-typed")]
pub mod stable_typed;

#[cfg(feature = "profile-experimental-search")]
pub mod experimental_search;

#[cfg(feature = "profile-experimental-widgets")]
pub mod experimental_widgets;

#[cfg(feature = "profile-sqlite")]
pub mod sqlite;

/// Local Cargo feature profile names used by this probe crate.
pub const PROFILE_NAMES: &[&str] = &[
    "profile-stable-typed",
    "profile-experimental-search",
    "profile-experimental-widgets",
    "profile-sqlite",
];

/// Upstream release commit pin for all source anchors in this crate.
pub const UPSTREAM_COMMIT: &str = "1c44fb66214667c6d00acaf72ab592493653708b";

/// Upstream release tag pin.
pub const UPSTREAM_TAG: &str = "matrix-sdk-0.18.0";

/// Run every probe enabled for the current Cargo feature profile.
///
/// Compile-only: no network, stores, or secrets.
pub fn run_enabled_probes() {
    #[cfg(feature = "profile-stable-typed")]
    stable_typed::run_all();
    #[cfg(feature = "profile-experimental-search")]
    experimental_search::run_all();
    #[cfg(feature = "profile-experimental-widgets")]
    experimental_widgets::run_all();
    #[cfg(feature = "profile-sqlite")]
    sqlite::run_all();
}

/// Probe IDs expected to compile under the currently enabled profile.
pub fn enabled_probe_ids() -> &'static [&'static str] {
    #[cfg(feature = "profile-stable-typed")]
    {
        return stable_typed::PROBE_IDS;
    }
    #[cfg(feature = "profile-experimental-search")]
    {
        return experimental_search::PROBE_IDS;
    }
    #[cfg(feature = "profile-experimental-widgets")]
    {
        return experimental_widgets::PROBE_IDS;
    }
    #[cfg(feature = "profile-sqlite")]
    {
        return sqlite::PROBE_IDS;
    }
    #[cfg(not(any(
        feature = "profile-stable-typed",
        feature = "profile-experimental-search",
        feature = "profile-experimental-widgets",
        feature = "profile-sqlite"
    )))]
    {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{ALL_PROBE_IDS, PROBE_CATALOG, profile_for_probe};

    #[test]
    fn catalog_probe_ids_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for id in ALL_PROBE_IDS {
            assert!(seen.insert(*id), "duplicate probe id: {id}");
        }
        assert_eq!(ALL_PROBE_IDS.len(), PROBE_CATALOG.len());
    }

    #[test]
    fn catalog_entries_are_one_to_one_with_profiles() {
        for entry in PROBE_CATALOG {
            assert!(
                PROFILE_NAMES.contains(&entry.profile),
                "unknown profile {} for {}",
                entry.profile,
                entry.id
            );
            assert_eq!(
                profile_for_probe(entry.id),
                Some(entry.profile),
                "catalog profile mismatch for {}",
                entry.id
            );
        }
    }

    #[test]
    fn enabled_profile_probes_compile_and_run() {
        // Compile-only API-shape probe execution; does not prove runtime semantics.
        run_enabled_probes();
        let enabled = enabled_probe_ids();
        let mut seen = std::collections::BTreeSet::new();
        for id in enabled {
            assert!(seen.insert(*id), "duplicate enabled probe id: {id}");
            assert!(
                ALL_PROBE_IDS.contains(id),
                "enabled probe {id} missing from global catalog"
            );
            let profile = profile_for_probe(id).expect("catalog profile");
            #[cfg(feature = "profile-stable-typed")]
            assert_eq!(profile, "profile-stable-typed");
            #[cfg(feature = "profile-experimental-search")]
            assert_eq!(profile, "profile-experimental-search");
            #[cfg(feature = "profile-experimental-widgets")]
            assert_eq!(profile, "profile-experimental-widgets");
            #[cfg(feature = "profile-sqlite")]
            assert_eq!(profile, "profile-sqlite");
        }
    }

    #[test]
    fn upstream_pin_constants() {
        assert_eq!(UPSTREAM_COMMIT, "1c44fb66214667c6d00acaf72ab592493653708b");
        assert_eq!(UPSTREAM_TAG, "matrix-sdk-0.18.0");
    }
}
