//! Supervised async work kinds for Matrix lifecycle tasks.

/// Kind of background work tracked by [`super::TaskSupervisor`].
///
/// Product sync/listener/upload/search loops will use these labels for
/// diagnostics (P2.5) and selective cancellation. No kind starts a production
/// homeserver session by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskKind {
    /// Matrix sync / sync-service loop.
    Sync,
    /// Room/timeline/account-data listeners and stream publishers.
    Listener,
    /// Media or attachment upload.
    Upload,
    /// Message / room search request.
    Search,
    /// Catch-all for harness or unclassified short-lived work.
    Generic,
}

impl TaskKind {
    pub const ALL: &'static [TaskKind] = &[
        Self::Sync,
        Self::Listener,
        Self::Upload,
        Self::Search,
        Self::Generic,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sync => "sync",
            Self::Listener => "listener",
            Self::Upload => "upload",
            Self::Search => "search",
            Self::Generic => "generic",
        }
    }
}
