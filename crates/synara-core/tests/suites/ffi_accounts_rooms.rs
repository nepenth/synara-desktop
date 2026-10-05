//! SharedCore account and room-operation contracts.
//! Keep source files in place so relative fixture and source-contract paths remain stable.

#![recursion_limit = "256"]

#[path = "../p4_s9_account_settings.rs"]
mod p4_s9_account_settings;
#[path = "../p4_s9_backup_restore.rs"]
mod p4_s9_backup_restore;
#[path = "../p4_s9_devices.rs"]
mod p4_s9_devices;
#[path = "../p4_s9_directory_search.rs"]
mod p4_s9_directory_search;
#[path = "../p4_s9_directory_visibility.rs"]
mod p4_s9_directory_visibility;
#[path = "../p4_s9_http_pusher.rs"]
mod p4_s9_http_pusher;
#[path = "../p4_s9_image_packs.rs"]
mod p4_s9_image_packs;
#[path = "../p4_s9_invite_actions.rs"]
mod p4_s9_invite_actions;
#[path = "../p4_s9_join_rules.rs"]
mod p4_s9_join_rules;
#[path = "../p4_s9_later.rs"]
mod p4_s9_later;
#[path = "../p4_s9_mdirect.rs"]
mod p4_s9_mdirect;
#[path = "../p4_s9_members_snapshots.rs"]
mod p4_s9_members_snapshots;
#[path = "../p4_s9_own_profile.rs"]
mod p4_s9_own_profile;
#[path = "../p4_s9_power_levels.rs"]
mod p4_s9_power_levels;
#[path = "../p4_s9_room_create.rs"]
mod p4_s9_room_create;
#[path = "../p4_s9_room_leave_join.rs"]
mod p4_s9_room_leave_join;
#[path = "../p4_s9_room_moderation.rs"]
mod p4_s9_room_moderation;
#[path = "../p4_s9_room_notes.rs"]
mod p4_s9_room_notes;
#[path = "../p4_s9_room_profile.rs"]
mod p4_s9_room_profile;
#[path = "../p4_s9_session_status.rs"]
mod p4_s9_session_status;
#[path = "../p4_s9_spaces.rs"]
mod p4_s9_spaces;
#[path = "../p4_s9_user_directory_search.rs"]
mod p4_s9_user_directory_search;
#[path = "../p4_s9_verification_sas.rs"]
mod p4_s9_verification_sas;
