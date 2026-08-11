//! Privacy-safe errors for room-list projection (P4.2).

use crate::transport::MatrixIpcErrorCategory;

/// Room-list snapshot/delta foundation failure.
#[derive(Debug)]
pub enum RoomListError {
    /// Delta sequence gap or stale generation → client must resync.
    ResyncRequired {
        diagnostic_id: &'static str,
        category: MatrixIpcErrorCategory,
    },
    /// Index / operation out of bounds for current projection.
    InvalidDelta { diagnostic_id: &'static str },
    /// Generation stamp mismatch.
    StaleGeneration {
        diagnostic_id: &'static str,
        expected: u64,
        observed: u64,
    },
    /// Invalid operator input.
    Invalid { diagnostic_id: &'static str },
}

impl RoomListError {
    pub fn diagnostic_id(&self) -> &'static str {
        match self {
            Self::ResyncRequired { diagnostic_id, .. }
            | Self::InvalidDelta { diagnostic_id }
            | Self::StaleGeneration { diagnostic_id, .. }
            | Self::Invalid { diagnostic_id } => diagnostic_id,
        }
    }

    pub fn category(&self) -> MatrixIpcErrorCategory {
        match self {
            Self::ResyncRequired { category, .. } => *category,
            Self::InvalidDelta { .. } | Self::Invalid { .. } => {
                MatrixIpcErrorCategory::SdkInvariant
            }
            Self::StaleGeneration { .. } => MatrixIpcErrorCategory::StaleSessionGeneration,
        }
    }

    pub fn requires_resync(&self) -> bool {
        matches!(
            self,
            Self::ResyncRequired { .. } | Self::StaleGeneration { .. }
        )
    }
}

impl std::fmt::Display for RoomListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ResyncRequired {
                diagnostic_id,
                category,
            } => write!(f, "room list resync required ({category:?}, {diagnostic_id})"),
            Self::InvalidDelta { diagnostic_id } => {
                write!(f, "invalid room list delta ({diagnostic_id})")
            }
            Self::StaleGeneration {
                diagnostic_id,
                expected,
                observed,
            } => write!(
                f,
                "stale room list generation ({diagnostic_id}): expected {expected}, observed {observed}"
            ),
            Self::Invalid { diagnostic_id } => {
                write!(f, "invalid room list operation ({diagnostic_id})")
            }
        }
    }
}

impl std::error::Error for RoomListError {}
