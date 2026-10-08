//! Product adapters against isolated SDK mock servers and shared format fixtures.
//! Keep source files in place so relative fixture and source-contract paths remain stable.

#![recursion_limit = "256"]

#[path = "../agent_approval_inbox.rs"]
mod agent_approval_inbox;
#[path = "../encrypted_state_events.rs"]
mod encrypted_state_events;
#[path = "../support/ffi_surface.rs"]
mod ffi_surface;
#[path = "../inbox_notifications.rs"]
mod inbox_notifications;
#[path = "../message_format_corpus.rs"]
mod message_format_corpus;
#[path = "../messaging_core_pins.rs"]
mod messaging_core_pins;
#[path = "../messaging_core_reactions.rs"]
mod messaging_core_reactions;
#[path = "../messaging_core_send_queue.rs"]
mod messaging_core_send_queue;
#[path = "../native_extras.rs"]
mod native_extras;
#[path = "../offline_timeline_cold_restart.rs"]
mod offline_timeline_cold_restart;
#[path = "../p4_s30_room_list_encryption.rs"]
mod p4_s30_room_list_encryption;
#[path = "../p4_s33_timeline_media.rs"]
mod p4_s33_timeline_media;
#[path = "../p4_s35_last_message_preview.rs"]
mod p4_s35_last_message_preview;
#[path = "../p4_s36_desktop_media_cutover.rs"]
mod p4_s36_desktop_media_cutover;
#[path = "../p4_s37_timeline_sequencing.rs"]
mod p4_s37_timeline_sequencing;
#[path = "../p4_s38_timeline_follow_live.rs"]
mod p4_s38_timeline_follow_live;
#[path = "../p4_s39_notification_push_rules.rs"]
mod p4_s39_notification_push_rules;
#[path = "../room_attachment_listing.rs"]
mod room_attachment_listing;
