//! Product `MediaRetentionPolicy` inputs. The SDK store is global, not per-room.

use std::time::Duration;

use super::index::MAX_TOTAL_BYTES;
use crate::app::media::MAX_PLAIN_MEDIA_DOWNLOAD_BYTES;

/// Product last-access expiry when no joined room has a stricter max_lifetime.
pub const DEFAULT_LAST_ACCESS_EXPIRY: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Daily cleanup is enough for the sqlite media store.
pub const DEFAULT_CLEANUP_FREQUENCY: Duration = Duration::from_secs(24 * 60 * 60);

/// Bounded product policy before mapping onto the SDK type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRetentionPolicySpec {
    pub max_cache_size: u64,
    pub max_file_size: u64,
    pub last_access_expiry: Duration,
    pub cleanup_frequency: Duration,
}

/// Strictest joined-room `max_lifetime` shortens the global media TTL only
/// when it is stricter than the 30-day product default.
pub fn build_media_retention_policy_spec(
    joined_room_max_lifetimes: &[Duration],
    homeserver_max_upload_bytes: Option<u64>,
) -> MediaRetentionPolicySpec {
    let product_file = MAX_PLAIN_MEDIA_DOWNLOAD_BYTES as u64;
    let max_file_size = match homeserver_max_upload_bytes {
        Some(hs) if hs > 0 => hs.min(product_file),
        _ => product_file,
    };
    let last_access_expiry = joined_room_max_lifetimes
        .iter()
        .copied()
        .min()
        .map(|strictest| strictest.min(DEFAULT_LAST_ACCESS_EXPIRY))
        .unwrap_or(DEFAULT_LAST_ACCESS_EXPIRY);
    MediaRetentionPolicySpec {
        max_cache_size: MAX_TOTAL_BYTES,
        max_file_size,
        last_access_expiry,
        cleanup_frequency: DEFAULT_CLEANUP_FREQUENCY,
    }
}

pub fn shortest_joined_max_lifetime(joined_room_max_lifetimes: &[Duration]) -> Option<Duration> {
    joined_room_max_lifetimes.iter().copied().min()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_policy_keeps_thirty_day_default() {
        let spec = build_media_retention_policy_spec(&[], None);
        assert_eq!(spec.max_cache_size, MAX_TOTAL_BYTES);
        assert_eq!(spec.max_file_size, MAX_PLAIN_MEDIA_DOWNLOAD_BYTES as u64);
        assert_eq!(spec.last_access_expiry, DEFAULT_LAST_ACCESS_EXPIRY);
        assert_eq!(spec.cleanup_frequency, DEFAULT_CLEANUP_FREQUENCY);
        assert!(shortest_joined_max_lifetime(&[]).is_none());
    }

    #[test]
    fn longer_room_lifetime_does_not_extend_past_product_default() {
        let year = Duration::from_secs(365 * 24 * 60 * 60);
        let spec = build_media_retention_policy_spec(&[year], None);
        assert_eq!(spec.last_access_expiry, DEFAULT_LAST_ACCESS_EXPIRY);
    }

    #[test]
    fn strictest_of_n_lifetimes_wins_when_stricter_than_default() {
        let day = Duration::from_secs(24 * 60 * 60);
        let week = Duration::from_secs(7 * 24 * 60 * 60);
        let spec = build_media_retention_policy_spec(&[week, day], None);
        assert_eq!(spec.last_access_expiry, day);
        assert_eq!(shortest_joined_max_lifetime(&[week, day]), Some(day));
    }

    #[test]
    fn file_size_is_min_of_product_cap_and_homeserver_upload() {
        let spec = build_media_retention_policy_spec(&[], Some(10 * 1024 * 1024));
        assert_eq!(spec.max_file_size, 10 * 1024 * 1024);
        let spec = build_media_retention_policy_spec(&[], Some(u64::MAX));
        assert_eq!(spec.max_file_size, MAX_PLAIN_MEDIA_DOWNLOAD_BYTES as u64);
        let spec = build_media_retention_policy_spec(&[], Some(0));
        assert_eq!(spec.max_file_size, MAX_PLAIN_MEDIA_DOWNLOAD_BYTES as u64);
    }
}
