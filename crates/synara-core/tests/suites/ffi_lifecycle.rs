//! SharedCore lifecycle, sessions, subscriptions, and store contracts.
//! Keep source files in place so relative fixture and source-contract paths remain stable.

#![recursion_limit = "256"]

#[path = "../support/ffi_surface.rs"]
mod ffi_surface;
#[path = "../p4_s10_leftovers.rs"]
mod p4_s10_leftovers;
#[path = "../p4_s11_nse_store.rs"]
mod p4_s11_nse_store;
#[path = "../p4_s12_start_sync.rs"]
mod p4_s12_start_sync;
#[path = "../p4_s13_session_bootstrap.rs"]
mod p4_s13_session_bootstrap;
#[path = "../p4_s14_timeline_view_updates.rs"]
mod p4_s14_timeline_view_updates;
#[path = "../p4_s15_leftover_io.rs"]
mod p4_s15_leftover_io;
#[path = "../p4_s16_timeline_rows.rs"]
mod p4_s16_timeline_rows;
#[path = "../p4_s17_owner_updates.rs"]
mod p4_s17_owner_updates;
#[path = "../p4_s19_room_list_updates.rs"]
mod p4_s19_room_list_updates;
#[path = "../p4_s3b_restore.rs"]
mod p4_s3b_restore;
#[path = "../p4_s3c_login.rs"]
mod p4_s3c_login;
#[path = "../p4_s3d_attach.rs"]
mod p4_s3d_attach;
#[path = "../p4_s4_room_list.rs"]
mod p4_s4_room_list;
#[path = "../p4_s5_invites.rs"]
mod p4_s5_invites;
#[path = "../p4_s6_timeline.rs"]
mod p4_s6_timeline;
#[path = "../p4_s7_typing_presence.rs"]
mod p4_s7_typing_presence;
#[path = "../p4_s8_verification_list.rs"]
mod p4_s8_verification_list;

#[path = "../owner_queue_lifecycle.rs"]
mod owner_queue_lifecycle;
