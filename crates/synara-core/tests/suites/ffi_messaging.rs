//! SharedCore message, media, composer, and timeline-operation contracts.
//! Keep source files in place so relative fixture and source-contract paths remain stable.

#![recursion_limit = "256"]

#[path = "../support/ffi_surface.rs"]
mod ffi_surface;
#[path = "../p4_s9_composer_draft.rs"]
mod p4_s9_composer_draft;
#[path = "../p4_s9_edit_message.rs"]
mod p4_s9_edit_message;
#[path = "../p4_s9_media_send.rs"]
mod p4_s9_media_send;
#[path = "../p4_s9_message_search.rs"]
mod p4_s9_message_search;
#[path = "../p4_s9_plain_media.rs"]
mod p4_s9_plain_media;
#[path = "../p4_s9_poll_respond.rs"]
mod p4_s9_poll_respond;
#[path = "../p4_s9_send_poll.rs"]
mod p4_s9_send_poll;
#[path = "../p4_s9_send_text.rs"]
mod p4_s9_send_text;
#[path = "../p4_s9_timeline_forward.rs"]
mod p4_s9_timeline_forward;
#[path = "../p4_s9_timeline_mutate.rs"]
mod p4_s9_timeline_mutate;
#[path = "../p4_s9_timeline_pin.rs"]
mod p4_s9_timeline_pin;
#[path = "../p4_s9_timeline_reactions.rs"]
mod p4_s9_timeline_reactions;
#[path = "../p4_s9_timeline_read_state.rs"]
mod p4_s9_timeline_read_state;
#[path = "../p4_s9_timeline_vote_decline.rs"]
mod p4_s9_timeline_vote_decline;
