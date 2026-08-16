//! Search session / result page index (P6.8 harness foundation).
//!
//! Pure projection of Synara [`SearchResult`] DTOs. No SDK search APIs,
//! no dual-backend. Supports cancel + stale-result protection via request ids.

use crate::dto::{SearchResult, SearchResultItem};

use super::error::SearchError;

/// Soft cap on accumulated hits per active search (memory bound).
pub const MAX_RESULTS_PER_SEARCH: usize = 500;

/// Lifecycle of one user-initiated search request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchState {
    Idle,
    InFlight,
    Ready,
    Cancelled,
    Failed,
}

/// Session-generation-stamped search session store.
#[derive(Debug)]
pub struct SearchSession {
    session_generation: u64,
    /// Monotonic request id for the active/latest search.
    request_id: u64,
    state: SearchState,
    query: String,
    room_scope: Option<String>,
    items: Vec<SearchResultItem>,
    next_batch: Option<String>,
    total_count: Option<u32>,
    failure_diagnostic_id: Option<&'static str>,
}

impl SearchSession {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            request_id: 0,
            state: SearchState::Idle,
            query: String::new(),
            room_scope: None,
            items: Vec::new(),
            next_batch: None,
            total_count: None,
            failure_diagnostic_id: None,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn state(&self) -> SearchState {
        self.state
    }

    pub fn request_id(&self) -> u64 {
        self.request_id
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn items(&self) -> &[SearchResultItem] {
        &self.items
    }

    pub fn next_batch(&self) -> Option<&str> {
        self.next_batch.as_deref()
    }

    pub fn total_count(&self) -> Option<u32> {
        self.total_count
    }

    /// Begin a new search. Returns the request id host must stamp on async work.
    pub fn begin(
        &mut self,
        query: impl Into<String>,
        room_id: Option<String>,
    ) -> Result<u64, SearchError> {
        let query = query.into().trim().to_owned();
        if query.is_empty() {
            return Err(SearchError::Invalid {
                diagnostic_id: "p6.8-empty-query",
            });
        }
        if let Some(ref room) = room_id {
            if room.is_empty() || !room.starts_with('!') {
                return Err(SearchError::Invalid {
                    diagnostic_id: "p6.8-invalid-room-id",
                });
            }
        }
        self.request_id = self.request_id.saturating_add(1);
        self.state = SearchState::InFlight;
        self.query = query;
        self.room_scope = room_id;
        self.items.clear();
        self.next_batch = None;
        self.total_count = None;
        self.failure_diagnostic_id = None;
        Ok(self.request_id)
    }

    /// Apply a result page. Stale request ids are ignored (not an error).
    pub fn apply_page(
        &mut self,
        request_id: u64,
        page: SearchResult,
        append: bool,
    ) -> Result<bool, SearchError> {
        if request_id != self.request_id {
            return Ok(false);
        }
        if self.state == SearchState::Cancelled {
            return Err(SearchError::Cancelled {
                diagnostic_id: "p6.8-apply-after-cancel",
            });
        }
        if page.query != self.query {
            return Err(SearchError::Invalid {
                diagnostic_id: "p6.8-query-mismatch",
            });
        }
        for item in &page.results {
            if item.event_id.is_empty() || !item.event_id.starts_with('$') {
                return Err(SearchError::Invalid {
                    diagnostic_id: "p6.8-invalid-event-id",
                });
            }
            if item.room_id.is_empty() || !item.room_id.starts_with('!') {
                return Err(SearchError::Invalid {
                    diagnostic_id: "p6.8-invalid-item-room-id",
                });
            }
        }
        if !append {
            self.items.clear();
        }
        for item in page.results {
            if self.items.len() >= MAX_RESULTS_PER_SEARCH {
                break;
            }
            // Dedup by event_id within accumulated results.
            if self.items.iter().any(|e| e.event_id == item.event_id) {
                continue;
            }
            self.items.push(item);
        }
        self.next_batch = page.next_batch;
        if page.total_count.is_some() {
            self.total_count = page.total_count;
        }
        self.state = SearchState::Ready;
        Ok(true)
    }

    pub fn fail(
        &mut self,
        request_id: u64,
        diagnostic_id: &'static str,
    ) -> Result<bool, SearchError> {
        if request_id != self.request_id {
            return Ok(false);
        }
        if self.state == SearchState::Cancelled {
            return Err(SearchError::Cancelled {
                diagnostic_id: "p6.8-fail-after-cancel",
            });
        }
        self.state = SearchState::Failed;
        self.failure_diagnostic_id = Some(diagnostic_id);
        Ok(true)
    }

    /// Cancel the active search; later pages for this request_id are rejected.
    pub fn cancel(&mut self) {
        if matches!(self.state, SearchState::InFlight | SearchState::Ready) {
            self.state = SearchState::Cancelled;
        }
    }

    pub fn clear(&mut self) {
        self.state = SearchState::Idle;
        self.query.clear();
        self.room_scope = None;
        self.items.clear();
        self.next_batch = None;
        self.total_count = None;
        self.failure_diagnostic_id = None;
    }

    pub fn failure_diagnostic_id(&self) -> Option<&'static str> {
        self.failure_diagnostic_id
    }

    /// Snapshot as a [`SearchResult`] DTO for IPC.
    pub fn to_result(&self) -> SearchResult {
        SearchResult {
            query: self.query.clone(),
            room_id: self.room_scope.clone(),
            results: self.items.clone(),
            next_batch: self.next_batch.clone(),
            total_count: self.total_count,
        }
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        *self = Self::new(new_generation);
    }
}
