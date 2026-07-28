//! Media cache / retention index foundation (P7.3 harness).
//!
//! Tracks local media cache entries by handle id with size and last-access
//! timestamps for eviction policy. **No file bytes**, no disk I/O, no SDK
//! media network, no dual-backend, no tokens.

use std::collections::HashMap;

use super::error::MediaCacheError;

/// Soft cap on tracked cache entries.
pub const MAX_CACHE_ENTRIES: usize = 4_096;

/// Soft cap on total declared size_bytes sum (2 GiB logical budget).
pub const MAX_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Soft cap on handle / mxc string length (chars).
pub const MAX_ID_CHARS: usize = 2_048;

/// One local cache entry (metadata only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    /// Product local handle / cache key — never raw bytes.
    pub handle_id: String,
    /// Optional source mxc URI string.
    pub mxc_uri: Option<String>,
    /// Declared content size when known (not verified against disk here).
    pub size_bytes: u64,
    /// Last access time in milliseconds since Unix epoch (host-provided).
    pub last_access_ts: u64,
    /// Optional mime type string.
    pub mime_type: Option<String>,
}

/// Session-generation-stamped media cache index + retention helpers.
#[derive(Debug, Default)]
pub struct MediaCacheIndex {
    session_generation: u64,
    entries: HashMap<String, CacheEntry>,
    total_bytes: u64,
}

impl MediaCacheIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            entries: HashMap::new(),
            total_bytes: 0,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    pub fn get(&self, handle_id: &str) -> Option<&CacheEntry> {
        self.entries.get(handle_id)
    }

    /// Insert or replace a cache entry. Replaces adjust total_bytes.
    pub fn upsert(&mut self, entry: CacheEntry) -> Result<(), MediaCacheError> {
        validate_entry(&entry)?;
        if !self.entries.contains_key(&entry.handle_id) && self.entries.len() >= MAX_CACHE_ENTRIES {
            return Err(MediaCacheError::Invalid {
                diagnostic_id: "p7.3-entry-cap",
            });
        }

        let new_total = if let Some(prev) = self.entries.get(&entry.handle_id) {
            self.total_bytes
                .saturating_sub(prev.size_bytes)
                .saturating_add(entry.size_bytes)
        } else {
            self.total_bytes.saturating_add(entry.size_bytes)
        };
        if new_total > MAX_TOTAL_BYTES {
            return Err(MediaCacheError::Invalid {
                diagnostic_id: "p7.3-total-bytes-cap",
            });
        }

        self.total_bytes = new_total;
        self.entries.insert(entry.handle_id.clone(), entry);
        Ok(())
    }

    /// Touch last_access_ts (host clock).
    pub fn touch(&mut self, handle_id: &str, now_ts: u64) -> Result<(), MediaCacheError> {
        let e = self
            .entries
            .get_mut(handle_id)
            .ok_or(MediaCacheError::NotFound {
                diagnostic_id: "p7.3-not-found",
            })?;
        e.last_access_ts = now_ts;
        Ok(())
    }

    pub fn remove(&mut self, handle_id: &str) -> bool {
        if let Some(prev) = self.entries.remove(handle_id) {
            self.total_bytes = self.total_bytes.saturating_sub(prev.size_bytes);
            true
        } else {
            false
        }
    }

    /// LRU eviction candidates: oldest last_access first, until under `byte_budget`
    /// and/or under `entry_budget`. Returns handle ids that host should delete on disk.
    pub fn plan_eviction(&self, byte_budget: u64, entry_budget: usize) -> Vec<String> {
        if self.total_bytes <= byte_budget && self.entries.len() <= entry_budget {
            return Vec::new();
        }
        let mut ordered: Vec<&CacheEntry> = self.entries.values().collect();
        ordered.sort_by(|a, b| {
            a.last_access_ts
                .cmp(&b.last_access_ts)
                .then_with(|| a.handle_id.cmp(&b.handle_id))
        });

        let mut drop_ids = Vec::new();
        let mut bytes = self.total_bytes;
        let mut count = self.entries.len();
        for e in ordered {
            if bytes <= byte_budget && count <= entry_budget {
                break;
            }
            drop_ids.push(e.handle_id.clone());
            bytes = bytes.saturating_sub(e.size_bytes);
            count = count.saturating_sub(1);
        }
        drop_ids
    }

    /// Apply eviction plan (remove planned handles).
    pub fn apply_eviction(&mut self, handle_ids: &[String]) -> usize {
        let mut n = 0;
        for id in handle_ids {
            if self.remove(id) {
                n += 1;
            }
        }
        n
    }

    /// Remove entries not accessed since `before_ts` (privacy / retention cleanup).
    pub fn purge_older_than(&mut self, before_ts: u64) -> usize {
        let stale: Vec<String> = self
            .entries
            .values()
            .filter(|e| e.last_access_ts < before_ts)
            .map(|e| e.handle_id.clone())
            .collect();
        self.apply_eviction(&stale)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }

    /// Bump generation and wipe (logout / account switch / privacy wipe).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }

    /// Handles sorted by last_access ascending (LRU order).
    pub fn list_lru(&self) -> Vec<&CacheEntry> {
        let mut v: Vec<_> = self.entries.values().collect();
        v.sort_by(|a, b| {
            a.last_access_ts
                .cmp(&b.last_access_ts)
                .then_with(|| a.handle_id.cmp(&b.handle_id))
        });
        v
    }
}

fn validate_entry(entry: &CacheEntry) -> Result<(), MediaCacheError> {
    validate_id(&entry.handle_id, "p7.3-invalid-handle")?;
    if let Some(ref mxc) = entry.mxc_uri {
        validate_id(mxc, "p7.3-invalid-mxc")?;
        let lower = mxc.to_ascii_lowercase();
        if !lower.starts_with("mxc://") && !lower.starts_with("local:") {
            // Allow product local: keys; mxc:// preferred for remote.
            if lower.starts_with("data:") || lower.starts_with("javascript:") {
                return Err(MediaCacheError::Invalid {
                    diagnostic_id: "p7.3-forbidden-mxc-scheme",
                });
            }
        }
        if lower.starts_with("data:") || lower.starts_with("javascript:") {
            return Err(MediaCacheError::Invalid {
                diagnostic_id: "p7.3-forbidden-mxc-scheme",
            });
        }
    }
    if let Some(ref mime) = entry.mime_type {
        if mime.chars().count() > 256 {
            return Err(MediaCacheError::Invalid {
                diagnostic_id: "p7.3-mime-cap",
            });
        }
    }
    Ok(())
}

fn validate_id(id: &str, empty_diag: &'static str) -> Result<(), MediaCacheError> {
    if id.is_empty() {
        return Err(MediaCacheError::Invalid {
            diagnostic_id: empty_diag,
        });
    }
    if id.chars().count() > MAX_ID_CHARS {
        return Err(MediaCacheError::Invalid {
            diagnostic_id: "p7.3-id-cap",
        });
    }
    let lower = id.to_ascii_lowercase();
    if lower.starts_with("data:") || lower.starts_with("javascript:") {
        return Err(MediaCacheError::Invalid {
            diagnostic_id: "p7.3-forbidden-scheme",
        });
    }
    if lower.contains("access_token") || lower.contains("refresh_token") {
        return Err(MediaCacheError::Invalid {
            diagnostic_id: "p7.3-forbidden-id",
        });
    }
    Ok(())
}
