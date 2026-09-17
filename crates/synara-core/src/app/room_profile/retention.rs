//! Read-only MSC1763 retention copy. Unknown homeserver config is not forever.

use std::time::Duration;

use super::native::MatrixRoomRetentionSnapshot;

pub const RETENTION_HISTORY_VISIBILITY_DISTINCTION: &str = "This is separate from history visibility, which only controls who may see past messages, not how long the server keeps them.";

pub const RETENTION_UNKNOWN_SUMMARY: &str = "The homeserver does not advertise a retention policy. History may still be deleted by the server without telling this client.";

pub const RETENTION_NO_LOCAL_COPY: &str =
    "Synara does not keep a local copy after the server purges.";

pub const MEDIA_CACHE_DEFAULT_SUMMARY: &str = "Media cache on this device expires after 30 days unless a joined room has a shorter retention.";

const DEFAULT_MEDIA_EXPIRY: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionCopy {
    pub advertised: bool,
    pub summary: String,
    pub distinction: &'static str,
    pub media_cache_summary: String,
}

/// Format a duration for settings copy. Prefers whole days, then hours, then minutes.
pub fn format_retention_duration(lifetime: Duration) -> String {
    let secs = lifetime.as_secs();
    const DAY: u64 = 86_400;
    const HOUR: u64 = 3_600;
    const MINUTE: u64 = 60;
    if secs >= DAY {
        let days = secs / DAY;
        if days == 1 {
            "1 day".to_owned()
        } else {
            format!("{days} days")
        }
    } else if secs >= HOUR {
        let hours = secs / HOUR;
        if hours == 1 {
            "1 hour".to_owned()
        } else {
            format!("{hours} hours")
        }
    } else {
        let minutes = (secs / MINUTE).max(1);
        if minutes == 1 {
            "1 minute".to_owned()
        } else {
            format!("{minutes} minutes")
        }
    }
}

pub fn format_retention_copy(
    max_lifetime: Option<Duration>,
    min_lifetime: Option<Duration>,
    shortest_joined_max_lifetime: Option<Duration>,
) -> RetentionCopy {
    let distinction = RETENTION_HISTORY_VISIBILITY_DISTINCTION;
    let media_cache_summary = format_media_cache_summary(shortest_joined_max_lifetime);
    match (max_lifetime, min_lifetime) {
        (None, None) => RetentionCopy {
            advertised: false,
            summary: RETENTION_UNKNOWN_SUMMARY.to_owned(),
            distinction,
            media_cache_summary,
        },
        (Some(max_lifetime), _) => RetentionCopy {
            advertised: true,
            summary: format!(
                "This room's server may delete messages older than {}. {RETENTION_NO_LOCAL_COPY}",
                format_retention_duration(max_lifetime)
            ),
            distinction,
            media_cache_summary,
        },
        (None, Some(min_lifetime)) => RetentionCopy {
            advertised: true,
            summary: format!(
                "This room's server advertises a minimum retention of {}, but no maximum lifetime. History may still be deleted without a published deadline.",
                format_retention_duration(min_lifetime)
            ),
            distinction,
            media_cache_summary,
        },
    }
}

pub fn format_media_cache_summary(shortest_joined_max_lifetime: Option<Duration>) -> String {
    match shortest_joined_max_lifetime {
        Some(lifetime) if lifetime < DEFAULT_MEDIA_EXPIRY => {
            format!(
                "Media cache on this device follows the shortest room retention among rooms you are in, currently {}.",
                format_retention_duration(lifetime)
            )
        }
        _ => MEDIA_CACHE_DEFAULT_SUMMARY.to_owned(),
    }
}

pub fn duration_to_ms(lifetime: Option<Duration>) -> Option<u64> {
    lifetime.and_then(|value| u64::try_from(value.as_millis()).ok())
}

pub fn retention_snapshot(
    room_id: String,
    session_generation: u64,
    max_lifetime: Option<Duration>,
    min_lifetime: Option<Duration>,
    shortest_joined_max_lifetime: Option<Duration>,
) -> MatrixRoomRetentionSnapshot {
    let copy = format_retention_copy(max_lifetime, min_lifetime, shortest_joined_max_lifetime);
    MatrixRoomRetentionSnapshot {
        status: "ok".to_owned(),
        room_id,
        session_generation,
        advertised: copy.advertised,
        max_lifetime_ms: duration_to_ms(max_lifetime),
        min_lifetime_ms: duration_to_ms(min_lifetime),
        summary: copy.summary,
        distinction: copy.distinction.to_owned(),
        media_cache_summary: copy.media_cache_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_not_retained_forever() {
        let copy = format_retention_copy(None, None, None);
        assert!(!copy.advertised);
        assert_eq!(copy.summary, RETENTION_UNKNOWN_SUMMARY);
        assert!(!copy.summary.to_ascii_lowercase().contains("forever"));
        assert!(!copy
            .summary
            .to_ascii_lowercase()
            .contains("history visibility"));
        assert_eq!(copy.distinction, RETENTION_HISTORY_VISIBILITY_DISTINCTION);
        assert_eq!(copy.media_cache_summary, MEDIA_CACHE_DEFAULT_SUMMARY);
    }

    #[test]
    fn max_lifetime_days() {
        let copy = format_retention_copy(Some(Duration::from_millis(7 * 86_400_000)), None, None);
        assert!(copy.advertised);
        assert!(copy.summary.contains("7 days"));
        assert!(copy.summary.contains(RETENTION_NO_LOCAL_COPY));
        assert_eq!(
            duration_to_ms(Some(Duration::from_millis(7 * 86_400_000))),
            Some(7 * 86_400_000)
        );
    }

    #[test]
    fn min_only_does_not_claim_a_deadline() {
        let copy = format_retention_copy(None, Some(Duration::from_secs(86_400)), None);
        assert!(copy.advertised);
        assert!(copy.summary.contains("1 day"));
        assert!(copy.summary.contains("no maximum lifetime"));
        assert!(!copy.summary.contains("may delete messages older than"));
    }

    #[test]
    fn media_cache_follows_stricter_joined_room() {
        let copy = format_retention_copy(None, None, Some(Duration::from_secs(24 * 60 * 60)));
        assert!(copy.media_cache_summary.contains("1 day"));
        assert!(copy.media_cache_summary.contains("shortest room retention"));
    }
}
