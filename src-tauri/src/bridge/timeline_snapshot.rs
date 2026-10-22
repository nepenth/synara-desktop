//! Desktop bridge for a read-only snapshot of an existing native timeline.

use serde::Serialize;
use synara_core::app::timeline::TimelineViewSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

/// A snapshot read. A safety-net poll that names the revision it already
/// shows gets only `{ revision }` back while the stream has not moved, so an
/// idle room's poll serializes no rows.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum TimelineSnapshotReply {
    Snapshot(Box<TimelineViewSnapshot>),
    Unchanged { revision: u64 },
}

pub(crate) async fn timeline_snapshot(
    core: &Core,
    stream_id: String,
    known_revision: Option<u64>,
) -> Result<TimelineSnapshotReply, MatrixAuthCommandError> {
    let request = synara_core::core_api::MatrixTimelineSnapshotRequest { stream_id };
    let Some(known_revision) = known_revision else {
        let snapshot = core
            .timeline_snapshot(request)
            .await
            .map_err(map_timeline_snapshot_core_error)?;
        return Ok(TimelineSnapshotReply::Snapshot(Box::new(snapshot)));
    };
    let snapshot = core
        .timeline_snapshot_after(request, known_revision)
        .await
        .map_err(map_timeline_snapshot_core_error)?;
    Ok(match snapshot {
        Some(snapshot) => TimelineSnapshotReply::Snapshot(Box::new(snapshot)),
        None => TimelineSnapshotReply::Unchanged {
            revision: known_revision,
        },
    })
}

/// Acknowledges an SDK retry request; the live timeline reports its outcome.
pub(crate) async fn timeline_retry_decryption(
    core: &Core,
    stream_id: String,
) -> Result<bool, MatrixAuthCommandError> {
    let response = core
        .timeline_retry_decryption(synara_core::core_api::MatrixTimelineSnapshotRequest {
            stream_id,
        })
        .await
        .map_err(map_timeline_snapshot_core_error)?;
    Ok(response)
}

fn map_timeline_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.3-timeline-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix timeline request is invalid.",
            "v-timeline-view-not-open",
        ),
        _ => timeline_snapshot_response_error(),
    }
}

fn timeline_snapshot_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix timeline is unavailable.",
        "v-timeline-view-snapshot-failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_session_owner_maps_to_timeline_requires_session() {
        let error = map_timeline_snapshot_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-timeline-snapshot-no-session"),
        );
        assert_eq!(
            error.diagnostic_id.as_str(),
            "d0.3-timeline-requires-session"
        );
    }

    #[test]
    fn an_unchanged_probe_serializes_only_the_revision() {
        let reply = TimelineSnapshotReply::Unchanged { revision: 7 };
        assert_eq!(
            serde_json::to_value(reply).unwrap(),
            serde_json::json!({ "revision": 7 })
        );
    }

    #[test]
    fn missing_room_view_stays_a_view_error() {
        let error = map_timeline_snapshot_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-timeline-view-not-open"),
        );
        assert_eq!(error.diagnostic_id.as_str(), "v-timeline-view-not-open");
    }
}
