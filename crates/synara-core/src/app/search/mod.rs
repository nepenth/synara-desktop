//! Search session harness plus live message search.
//!
//! Desktop (`search-index`) queries the 0.19 local index and never sends
//! Client-Server `search_events`. iOS SharedCore keeps typed `/search`.
//! Neither path returns leftover-unavailable.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.8-search.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod ipc;
mod listing;
mod live;
mod session;

pub use error::SearchError;
pub use ipc::{MatrixMessageSearchGroup, MatrixMessageSearchItem, MatrixMessageSearchResult};
pub use listing::{
    list_room_attachments, parse_attachment_listing_kind, AttachmentListingKind,
    MAX_ATTACHMENT_LISTING_EVENTS, MAX_ATTACHMENT_LISTING_PAGES,
};
#[cfg(feature = "search-index")]
pub use live::parse_message_search_offset;
pub use live::{
    empty_message_search_result, parse_message_search_next_token, parse_message_search_order,
    parse_message_search_rooms, parse_message_search_senders, parse_message_search_term,
    search_messages, MAX_MESSAGE_SEARCH_BODY_CHARS, MAX_MESSAGE_SEARCH_GROUPS,
    MAX_MESSAGE_SEARCH_HIGHLIGHTS, MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS, MAX_MESSAGE_SEARCH_ITEMS,
    MAX_MESSAGE_SEARCH_NEXT_TOKEN_CHARS, MAX_MESSAGE_SEARCH_ROOMS, MAX_MESSAGE_SEARCH_SENDERS,
    MAX_MESSAGE_SEARCH_TERM_CHARS, MESSAGE_SEARCH_LIMIT,
};
pub use session::{SearchSession, SearchState, MAX_RESULTS_PER_SEARCH};

/// Static marker for link / schema smoke.
pub const MATRIX_SEARCH_MARKER: &str = "matrix-search-p6.8";

/// Touch search paths so they remain linked in non-test builds.
pub fn matrix_search_markers() -> &'static str {
    let s = SearchSession::new(0);
    debug_assert_eq!(s.state(), SearchState::Idle);
    debug_assert!(s.items().is_empty());
    debug_assert_eq!(parse_message_search_term("").ok(), Some(None));
    debug_assert!(parse_attachment_listing_kind("media").is_ok());
    debug_assert_eq!(MESSAGE_SEARCH_LIMIT, 20);
    debug_assert_eq!(MATRIX_SEARCH_MARKER, "matrix-search-p6.8");
    MATRIX_SEARCH_MARKER
}

#[cfg(test)]
mod tests;
