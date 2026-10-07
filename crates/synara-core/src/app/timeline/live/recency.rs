//! Bounded, least-recently-used caches for the timeline registry.
//!
//! `HashMap::keys().next()` evicted an arbitrary entry, so a permalink the
//! user just opened could be dropped while a stale one stayed. Reads here
//! take `&self` and record use through an atomic stamp, which keeps every
//! existing read site borrow-compatible.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};

/// Process-wide use clock. Only the relative order of stamps matters.
static USE_CLOCK: AtomicU64 = AtomicU64::new(1);

pub(super) fn next_use_stamp() -> u64 {
    USE_CLOCK.fetch_add(1, Ordering::Relaxed)
}

/// Last-use stamp carried by a cached value.
#[derive(Debug)]
pub(super) struct UseStamp(AtomicU64);

impl UseStamp {
    pub(super) fn now() -> Self {
        Self(AtomicU64::new(next_use_stamp()))
    }

    pub(super) fn touch(&self) {
        self.0.store(next_use_stamp(), Ordering::Relaxed);
    }

    pub(super) fn demote(&self) {
        self.0.store(0, Ordering::Relaxed);
    }

    pub(super) fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// A capacity-bounded map that evicts its least recently used entry.
pub(super) struct RecencyMap<K, V> {
    capacity: usize,
    entries: HashMap<K, (V, UseStamp)>,
}

impl<K: Eq + Hash + Clone, V> RecencyMap<K, V> {
    pub(super) fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "recency map capacity must be positive");
        Self {
            capacity,
            entries: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.entries.contains_key(key)
    }

    /// Read a value and mark it as the most recently used.
    pub(super) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.entries.get(key).map(|(value, stamp)| {
            stamp.touch();
            value
        })
    }

    /// Insert as most recently used, evicting the least recently used entry
    /// first when the map is full. Returns the evicted key, if any.
    pub(super) fn insert(&mut self, key: K, value: V) -> Option<K> {
        let evicted = if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            self.least_recent_key().inspect(|oldest| {
                self.entries.remove(oldest);
            })
        } else {
            None
        };
        self.entries.insert(key, (value, UseStamp::now()));
        evicted
    }

    /// Iterate without changing recency.
    pub(super) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(key, (value, _))| (key, value))
    }

    fn least_recent_key(&self) -> Option<K> {
        self.entries
            .iter()
            .min_by_key(|(_, (_, stamp))| stamp.get())
            .map(|(key, _)| key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_the_least_recently_used_entry_not_an_arbitrary_one() {
        let mut map = RecencyMap::new(3);
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("c", 3);
        // Reading "a" makes "b" the least recently used.
        assert_eq!(map.get("a"), Some(&1));
        assert_eq!(map.insert("d", 4), Some("b"));
        assert!(map.contains_key("a"));
        assert!(!map.contains_key("b"));
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn reinserting_an_existing_key_does_not_evict() {
        let mut map = RecencyMap::new(2);
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.insert("a", 10), None);
        assert_eq!(map.get("a"), Some(&10));
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn iteration_does_not_refresh_recency() {
        let mut map = RecencyMap::new(2);
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.iter().count(), 2);
        assert_eq!(map.insert("c", 3), Some("a"));
    }

    #[test]
    fn demoted_stamp_sorts_before_every_live_stamp() {
        let live = UseStamp::now();
        let demoted = UseStamp::now();
        demoted.demote();
        assert!(demoted.get() < live.get());
    }
}
