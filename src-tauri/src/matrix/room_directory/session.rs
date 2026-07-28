//! Public room directory search session (P6.10 harness foundation).
//!
//! Pure projection of directory hits for product UI. No SDK directory
//! network, no dual-backend, no tokens. Stale request ids ignored.

use super::error::RoomDirectoryError;

/// Soft cap on accumulated directory hits per active search.
pub const MAX_DIRECTORY_HITS: usize = 200;

/// Soft cap on query / name / topic length (chars).
pub const MAX_TEXT_CHARS: usize = 256;

/// Soft cap on alias length (chars).
pub const MAX_ALIAS_CHARS: usize = 255;

/// Lifecycle of one directory search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectorySearchState {
    Idle,
    InFlight,
    Ready,
    Cancelled,
    Failed,
}

/// One public-room directory hit (privacy-safe product projection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryRoomHit {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub canonical_alias: Option<String>,
    /// mxc or product media handle URI only — never bytes.
    pub avatar_url: Option<String>,
    pub num_joined_members: u32,
    pub world_readable: bool,
    pub guest_can_join: bool,
}

/// Session-generation-stamped room directory session.
#[derive(Debug)]
pub struct RoomDirectorySession {
    session_generation: u64,
    request_id: u64,
    state: DirectorySearchState,
    query: String,
    /// Optional server name filter (e.g. `matrix.org`) — not a token.
    server_name: Option<String>,
    hits: Vec<DirectoryRoomHit>,
    next_batch: Option<String>,
    failure_diagnostic_id: Option<&'static str>,
}

impl RoomDirectorySession {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            request_id: 0,
            state: DirectorySearchState::Idle,
            query: String::new(),
            server_name: None,
            hits: Vec::new(),
            next_batch: None,
            failure_diagnostic_id: None,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn state(&self) -> DirectorySearchState {
        self.state
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn server_name(&self) -> Option<&str> {
        self.server_name.as_deref()
    }

    pub fn hits(&self) -> &[DirectoryRoomHit] {
        &self.hits
    }

    pub fn next_batch(&self) -> Option<&str> {
        self.next_batch.as_deref()
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    /// Begin a directory search. Empty query allowed for browse-all pages.
    pub fn begin(
        &mut self,
        query: impl Into<String>,
        server_name: Option<String>,
    ) -> Result<u64, RoomDirectoryError> {
        let query = query.into().trim().to_owned();
        if query.chars().count() > MAX_TEXT_CHARS {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-query-cap",
            });
        }
        if let Some(ref s) = server_name {
            let s = s.trim();
            if s.is_empty() || s.chars().count() > MAX_TEXT_CHARS {
                return Err(RoomDirectoryError::Invalid {
                    diagnostic_id: "p6.10-invalid-server-name",
                });
            }
            if s.contains("access_token") || s.contains("refresh_token") {
                return Err(RoomDirectoryError::Invalid {
                    diagnostic_id: "p6.10-forbidden-server-name",
                });
            }
            self.server_name = Some(s.to_owned());
        } else {
            self.server_name = None;
        }
        self.request_id = self.request_id.saturating_add(1);
        self.state = DirectorySearchState::InFlight;
        self.query = query;
        self.hits.clear();
        self.next_batch = None;
        self.failure_diagnostic_id = None;
        Ok(self.request_id)
    }

    /// Apply a page of results. Stale request_id is ignored (Ok(())).
    pub fn apply_page(
        &mut self,
        request_id: u64,
        page: Vec<DirectoryRoomHit>,
        next_batch: Option<String>,
        replace: bool,
    ) -> Result<(), RoomDirectoryError> {
        if request_id != self.request_id {
            return Ok(());
        }
        if self.state == DirectorySearchState::Cancelled {
            return Err(RoomDirectoryError::Cancelled {
                diagnostic_id: "p6.10-cancelled",
            });
        }
        if self.state != DirectorySearchState::InFlight && self.state != DirectorySearchState::Ready
        {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-apply-invalid-state",
            });
        }
        for hit in &page {
            validate_hit(hit)?;
        }
        if replace {
            self.hits = page;
        } else {
            // Dedup by room_id
            for hit in page {
                if !self.hits.iter().any(|h| h.room_id == hit.room_id) {
                    self.hits.push(hit);
                }
            }
        }
        if self.hits.len() > MAX_DIRECTORY_HITS {
            self.hits.truncate(MAX_DIRECTORY_HITS);
        }
        self.next_batch = next_batch;
        self.state = DirectorySearchState::Ready;
        self.failure_diagnostic_id = None;
        Ok(())
    }

    pub fn fail(
        &mut self,
        request_id: u64,
        diagnostic_id: &'static str,
    ) -> Result<(), RoomDirectoryError> {
        if request_id != self.request_id {
            return Ok(());
        }
        if self.state != DirectorySearchState::InFlight {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-fail-invalid-state",
            });
        }
        self.state = DirectorySearchState::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(())
    }

    pub fn cancel(&mut self) {
        if matches!(
            self.state,
            DirectorySearchState::InFlight | DirectorySearchState::Ready
        ) {
            self.state = DirectorySearchState::Cancelled;
        }
    }

    pub fn clear(&mut self) {
        self.state = DirectorySearchState::Idle;
        self.query.clear();
        self.server_name = None;
        self.hits.clear();
        self.next_batch = None;
        self.failure_diagnostic_id = None;
    }

    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.request_id = 0;
        self.clear();
    }
}

fn validate_hit(hit: &DirectoryRoomHit) -> Result<(), RoomDirectoryError> {
    if hit.room_id.is_empty() || !hit.room_id.starts_with('!') {
        return Err(RoomDirectoryError::Invalid {
            diagnostic_id: "p6.10-invalid-room-id",
        });
    }
    if let Some(ref n) = hit.name {
        if n.chars().count() > MAX_TEXT_CHARS {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-name-cap",
            });
        }
    }
    if let Some(ref t) = hit.topic {
        if t.chars().count() > MAX_TEXT_CHARS * 4 {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-topic-cap",
            });
        }
    }
    if let Some(ref a) = hit.canonical_alias {
        if a.is_empty() || !a.starts_with('#') || a.chars().count() > MAX_ALIAS_CHARS {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-invalid-alias",
            });
        }
    }
    if let Some(ref url) = hit.avatar_url {
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("data:") || lower.starts_with("javascript:") {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-forbidden-avatar-scheme",
            });
        }
        if lower.contains("access_token") {
            return Err(RoomDirectoryError::Invalid {
                diagnostic_id: "p6.10-forbidden-avatar",
            });
        }
    }
    Ok(())
}
