//! Pure media-cache retention and privacy policy (P7.6 harness).
//!
//! Inputs and outputs contain opaque local handle ids and timestamps only.
//! They never contain media bytes, event content, credentials, or key material.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Cache retention limits supplied by the product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// Maximum entries retained after age and privacy rules are applied.
    pub max_entries: usize,
    /// Optional time-to-live measured from an entry's last access.
    pub max_age_secs: Option<u64>,
    /// On logout, purge entries known to contain encrypted-room media.
    pub purge_on_logout: bool,
}

/// Metadata needed to evaluate one cache entry.
#[derive(Clone, PartialEq, Eq)]
pub struct EntryMeta {
    /// Opaque local cache handle. It is used only to identify a deletion target.
    pub handle_id: String,
    /// Last access time in seconds since the Unix epoch (host-provided).
    pub last_access_secs: u64,
    /// Whether the cached object is known to originate only from encrypted media.
    pub encrypted_only: bool,
}

/// Opaque handles the cache host should delete.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct PrivacyPurgePlan {
    pub handle_ids: Vec<String>,
}

impl PrivacyPurgePlan {
    pub fn is_empty(&self) -> bool {
        self.handle_ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.handle_ids.len()
    }
}

// Keep opaque handles out of accidental debug logs while retaining useful
// structural diagnostics for tests and harness integration.
impl fmt::Debug for EntryMeta {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntryMeta")
            .field("handle_id", &"<opaque>")
            .field("last_access_secs", &self.last_access_secs)
            .field("encrypted_only", &self.encrypted_only)
            .finish()
    }
}

impl fmt::Debug for PrivacyPurgePlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivacyPurgePlan")
            .field("handle_count", &self.handle_ids.len())
            .finish()
    }
}

/// Build a deterministic, metadata-only purge plan.
///
/// Rules are combined as a union:
///
/// - entries at or beyond `max_age_secs` are expired;
/// - when `purge_on_logout` is set, encrypted-only entries are privacy-purged;
/// - if the remaining distinct entries exceed `max_entries`, least-recently
///   accessed entries are added until the cap is met.
///
/// Future access timestamps are treated as age zero. Duplicate handle metadata
/// is normalized conservatively: the freshest access wins, while
/// `encrypted_only` is combined with logical OR. Returned handles are unique
/// and ordered by oldest access first, then handle id.
pub fn plan_purge(
    entries_meta: &[EntryMeta],
    policy: &RetentionPolicy,
    now_secs: u64,
) -> PrivacyPurgePlan {
    let mut by_handle: BTreeMap<&str, (u64, bool)> = BTreeMap::new();
    for entry in entries_meta {
        by_handle
            .entry(entry.handle_id.as_str())
            .and_modify(|(last_access_secs, encrypted_only)| {
                *last_access_secs = (*last_access_secs).max(entry.last_access_secs);
                *encrypted_only |= entry.encrypted_only;
            })
            .or_insert((entry.last_access_secs, entry.encrypted_only));
    }

    let mut ordered: Vec<(&str, u64, bool)> = by_handle
        .into_iter()
        .map(|(handle_id, (last_access_secs, encrypted_only))| {
            (handle_id, last_access_secs, encrypted_only)
        })
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(right.0)));

    let mut drop_handles = BTreeSet::new();
    for (handle_id, last_access_secs, encrypted_only) in &ordered {
        let expired = policy
            .max_age_secs
            .is_some_and(|max_age_secs| now_secs.saturating_sub(*last_access_secs) >= max_age_secs);
        let privacy_purge = policy.purge_on_logout && *encrypted_only;
        if expired || privacy_purge {
            drop_handles.insert(*handle_id);
        }
    }

    let retained = ordered.len().saturating_sub(drop_handles.len());
    let excess = retained.saturating_sub(policy.max_entries);
    if excess > 0 {
        let mut added = 0;
        for (handle_id, _, _) in &ordered {
            if added == excess {
                break;
            }
            if drop_handles.insert(*handle_id) {
                added += 1;
            }
        }
    }

    PrivacyPurgePlan {
        handle_ids: ordered
            .into_iter()
            .filter(|(handle_id, _, _)| drop_handles.contains(handle_id))
            .map(|(handle_id, _, _)| handle_id.to_owned())
            .collect(),
    }
}
