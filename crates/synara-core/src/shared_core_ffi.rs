//! UniFFI construction, restore, and dedicated password-login facade.
//!
//! P4-S2 exposed only `SharedCore::new` with a fail-closed vault. P4-S3a adds
//! `new_with_secret_store` so Swift can install a Keychain-backed
//! [`SecretVault`]. UniFFI 0.28 Swift keeps `SharedCore()` for the primary
//! constructor and emits the named constructor as
//! `SharedCore.newWithSecretStore(store:)`, not a second `init(store:)`.
//! P4-S3b adds `restore_persisted_session`. P4-S3c adds
//! `login_with_password`: a dedicated FFI argument, never `Core.command`,
//! never registered as `matrix_login_password`. The password is not stored,
//! not copied into a DTO, never echoed, and is zeroized on drop.
//! P4-S3d adds `attach_session_owners` for the desktop owner set.
//! P4-S4 adds a typed `room_list_snapshot` wrapper that calls the
//! already-registered `matrix_room_list_snapshot` Core command only.
//! P4-S5 adds a typed `invites_snapshot` wrapper that calls the
//! already-registered `matrix_invites_snapshot` Core command only.
//! P4-S6 adds typed `timeline_open` / `timeline_close` / `timeline_paginate`
//! wrappers for those three already-registered Core commands only.
//! P4-S7 adds typed typing/presence wrappers for the five already-registered
//! Core commands in that family only.
//! P4-S8 adds a typed `verification_list` wrapper for the already-registered
//! `matrix_verification_list` Core command only.
//! P4-S9 adds typed verification SAS wrappers for the seven already-registered
//! start/accept/begin_sas/confirm/mismatch/cancel/dismiss Core commands only.
//! P4-S9-2 adds typed device wrappers for the four already-registered
//! snapshot/rename/delete-start/delete-cancel Core commands only.
//! Backup status, room-key transfer status, and cross-signing setup stay off
//! this slice: they sit next to leftover passphrase/path/password envelopes.
//! P4-S9-3 adds a typed `room_join_rule_snapshot` wrapper for the
//! already-registered `matrix_room_join_rule_snapshot` Core command only.
//! There is no join-rule writer on Core.
//! P4-S9-4 adds typed image-pack get/set wrappers for the six
//! already-registered Core commands. Pack metadata/IDs/URLs/JSON may
//! cross. Image/media bytes stay off.
//! P4-S9-5 adds typed later snapshot/upsert/complete/snooze/
//! clear_completed/mark_reminded wrappers for those six already-registered
//! Core commands.
//! P4-S9-6 adds typed m.direct snapshot/add/remove wrappers for those
//! three already-registered Core commands.
//! P4-S9-7 adds typed room-notes snapshot/upsert/delete/complete_todo/
//! move_todo wrappers for those five already-registered Core commands.
//! P4-S9-8 adds typed `set_own_display_name` / `set_own_avatar` wrappers
//! for those two already-registered Core commands. Avatar is an `mxc://`
//! (or empty clear) reference only. Image/media bytes stay off. Failed
//! errors never echo display name or mxc.
//! P4-S9-9 adds typed `set_room_name` / `set_room_topic` / `set_room_avatar`
//! wrappers for those three already-registered Core commands. Room avatar
//! is an `mxc://` (or empty clear) reference only. Image/media bytes stay
//! off. Failed errors never echo room id, name, topic, or mxc.
//! P4-S9-10 adds typed `get_room_directory_visibility` /
//! `set_room_directory_visibility` wrappers for those two already-registered
//! Core commands. Failed errors never echo room id or visibility.
//! P4-S9-11 adds typed `room_directory_protocols` / `room_directory_search` /
//! `room_directory_cancel` wrappers for those three already-registered Core
//! commands. Search results stay metadata (room ids, names, aliases, mxc).
//! Avatar bytes stay off. Failed errors never echo term, server, or room id.
//! P4-S9-12 adds typed `room_leave` / `room_join` wrappers for those two
//! already-registered Core commands. Write ack is status only. Failed errors
//! never echo room id, alias, or via servers.
//! P4-S9-13 adds typed `room_invite` / `room_kick` / `room_ban` / `room_unban`
//! wrappers for those four already-registered Core commands. Write ack is
//! status only. Failed errors never echo room id, user id, or reason.
//! P4-S9-14 adds typed `room_set_power_level` / `room_set_power_levels` /
//! `room_set_power_level_tags` wrappers for those three already-registered
//! Core commands. Write ack is status only. Failed errors never echo room
//! id, user id, power level, or content JSON.
//! P4-S9-15 adds a typed `room_create` wrapper for the already-registered
//! `matrix_room_create` Core command only. Request is name/topic/alias/
//! visibility/preset plus Core scalar extras. Nested create-content,
//! power-level overrides, paths, passphrases, and media bytes stay
//! off. Success returns the created room id. Failed errors never echo name,
//! topic, alias, invite, or parent.
//! P4-S9-16 adds typed `room_members_snapshot` / `room_power_levels_snapshot`
//! / `room_creators_snapshot` / `room_power_level_tags_snapshot` wrappers
//! for those four already-registered Core commands. These are reads. Failed
//! errors never echo member user ids.
//! P4-S9-17 adds typed space parents/hierarchy/children snapshots plus
//! child set/remove and restricted-join reparent wrappers for those six
//! already-registered Core commands. Child set/remove are metadata only
//! (room ids, via, order, suggested). No bytes. Failed errors never echo
//! room ids. Invite accept/decline stay off.
//! P4-S9-18 adds typed `invites_accept` / `invites_decline` /
//! `invites_report_spam` / `invites_block_sender` wrappers for those four
//! already-registered Core commands. They return the existing invite
//! snapshot. Failed errors never echo room id or sender id. Timeline jump
//! and read-state stay off.
//! This still exposes no generic command FFI or APNs surface.

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use matrix_sdk::Client;
use zeroize::Zeroizing;

use crate::app::account_data::{
    NativeGlobalImagePacksSnapshot, NativeImagePack, NativeImagePackOwner, NativeLaterSnapshot,
    NativeMDirectSnapshot, NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot,
    NativeUserImagePackSnapshot, RoomNoteMoveDirection, SynaraLaterItem, SynaraLaterItemKind,
    SynaraRoomNoteItem, SynaraRoomNoteItemKind,
};
use crate::app::auth::{
    login_with_password as core_login_with_password, DevicePlatform, LoginOptions,
};
use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::app::devices::{
    NativeDeviceDeleteAuthentication, NativeDeviceDeleteResult, NativeDeviceOwner,
    NativeDeviceSnapshot, NativeDeviceTrust,
};
use crate::app::lifecycle::{
    persist_session_after_login, restore_session_from_vault, restore_session_onto_client,
    SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription,
};
use crate::app::room_list::{
    NativeInvite, NativeInviteSnapshot, NativeInviteTriage, NativeRoomListSnapshot,
};
use crate::app::room_profile::{MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleOwner};
use crate::app::store::{
    get_or_create_store_key, AccountIdentity, StoreKeyId, StoreKeyMaterial, StoreKeyVault,
    StoreKeyVaultError, STORE_KEY_LEN,
};
use crate::app::sync::{build_sync_service, SyncServiceConfig};
use crate::app::timeline::{
    NativeTimelineDirection, NativeTimelineOpenPosition, NativeTimelineOpenReadback,
    NativeTimelineOwner, NativeTimelineViewportHint, TimelinePageState, TimelineViewPosition,
    TimelineViewSnapshot,
};
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot};
use crate::app::verification::{
    NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationOwner, NativeVerificationPhase, NativeVerificationRequest,
    NativeVerificationSas,
};
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{
    CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory, MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
};

const VAULT_UNAVAILABLE_CODE: &str = "p4-s3b-secret-vault-unavailable";
const VAULT_UNAVAILABLE_DESCRIPTION: &str = "The secret store is unavailable.";
const IDENTITY_INVALID_CODE: &str = "p4-s3b-identity-invalid";
const IDENTITY_INVALID_DESCRIPTION: &str = "The session identity is invalid.";
const STORE_ROOT_INVALID_CODE: &str = "p4-s3b-store-root-invalid";
const STORE_ROOT_INVALID_DESCRIPTION: &str = "The session store root is invalid.";
const MATERIAL_MISSING_CODE: &str = "p4-s3b-session-material-missing";
const MATERIAL_MISSING_DESCRIPTION: &str = "No restorable session is available.";
const RESTORE_FAILED_CODE: &str = "p4-s3b-restore-failed";
const RESTORE_FAILED_DESCRIPTION: &str = "The persisted session could not be restored.";
const LOGIN_VAULT_UNAVAILABLE_CODE: &str = "p4-s3c-secret-vault-unavailable";
const LOGIN_VAULT_UNAVAILABLE_DESCRIPTION: &str = "The secret store is unavailable.";
const LOGIN_IDENTITY_INVALID_CODE: &str = "p4-s3c-identity-invalid";
const LOGIN_IDENTITY_INVALID_DESCRIPTION: &str = "The session identity is invalid.";
const LOGIN_STORE_ROOT_INVALID_CODE: &str = "p4-s3c-store-root-invalid";
const LOGIN_STORE_ROOT_INVALID_DESCRIPTION: &str = "The session store root is invalid.";
const LOGIN_FAILED_CODE: &str = "p4-s3c-login-failed";
const LOGIN_FAILED_DESCRIPTION: &str = "The session could not be authenticated.";
const ATTACH_SESSION_MISSING_CODE: &str = "p4-s3d-session-missing";
const ATTACH_SESSION_MISSING_DESCRIPTION: &str = "No retained session is available.";
const ATTACH_ALREADY_CODE: &str = "p4-s3d-already-attached";
const ATTACH_ALREADY_DESCRIPTION: &str = "Session owners are already attached.";
const ATTACH_FAILED_CODE: &str = "p4-s3d-attach-failed";
const ATTACH_FAILED_DESCRIPTION: &str = "Session owners could not be attached.";
const ATTACHED_OWNER_NAMES: &[&str] = &[
    "typing",
    "presence",
    "verification",
    "devices",
    "join_rules",
    "image_packs",
    "timelines",
    "sync",
];
const ROOM_LIST_COMMAND: &str = "matrix_room_list_snapshot";
const ROOM_LIST_READ_ONLY_GENERATION: u64 = 0;
const ROOM_LIST_NO_SESSION_CODE: &str = "p2-room-list-snapshot-no-session";
const ROOM_LIST_NO_SESSION_DESCRIPTION: &str = "No room list session is available.";
const ROOM_LIST_SYNC_NOT_STARTED_CODE: &str = "p4-s4-sync-not-started";
const ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION: &str = "The room list is not live.";
const ROOM_LIST_FAILED_CODE: &str = "p4-s4-snapshot-failed";
const ROOM_LIST_FAILED_DESCRIPTION: &str = "The room list could not be loaded.";
const INVITES_COMMAND: &str = "matrix_invites_snapshot";
const INVITES_READ_ONLY_GENERATION: u64 = 0;
const INVITES_NO_SESSION_CODE: &str = "p2-invites-snapshot-no-session";
const INVITES_NO_SESSION_DESCRIPTION: &str = "No invite session is available.";
const INVITES_FAILED_CODE: &str = "p4-s5-snapshot-failed";
const INVITES_FAILED_DESCRIPTION: &str = "The invite inbox could not be loaded.";
const TIMELINE_READ_ONLY_GENERATION: u64 = 0;
const TIMELINE_OPEN_COMMAND: &str = "matrix_timeline_open";
const TIMELINE_CLOSE_COMMAND: &str = "matrix_timeline_close";
const TIMELINE_PAGINATE_COMMAND: &str = "matrix_timeline_paginate";
const TIMELINE_OPEN_NO_SESSION_CODE: &str = "p2-timeline-open-no-session";
const TIMELINE_CLOSE_NO_SESSION_CODE: &str = "p2-timeline-close-no-session";
const TIMELINE_PAGINATE_NO_SESSION_CODE: &str = "p2-timeline-paginate-no-session";
const TIMELINE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_OPEN_FAILED_CODE: &str = "p4-s6-open-failed";
const TIMELINE_OPEN_FAILED_DESCRIPTION: &str = "The timeline could not be opened.";
const TIMELINE_CLOSE_FAILED_CODE: &str = "p4-s6-close-failed";
const TIMELINE_CLOSE_FAILED_DESCRIPTION: &str = "The timeline could not be closed.";
const TIMELINE_PAGINATE_FAILED_CODE: &str = "p4-s6-paginate-failed";
const TIMELINE_PAGINATE_FAILED_DESCRIPTION: &str = "The timeline could not be paginated.";
const TIMELINE_ROOM_NOT_FOUND_CODE: &str = "v-timeline-normal-room-not-found";
const TIMELINE_ROOM_NOT_FOUND_DESCRIPTION: &str = "The timeline room is not available.";
const TIMELINE_INVALID_ROOM_CODE: &str = "d0.3-timeline-invalid-room-id";
const TIMELINE_INVALID_ROOM_DESCRIPTION: &str = "The timeline room id is invalid.";
const TIMELINE_VIEW_NOT_OPEN_CODE: &str = "v-timeline-view-not-open";
const TIMELINE_VIEW_NOT_OPEN_DESCRIPTION: &str = "The timeline view is not open.";
const TYPING_PRESENCE_GENERATION: u64 = 0;
const TYPING_SNAPSHOT_COMMAND: &str = "matrix_typing_snapshot";
const TYPING_SET_COMMAND: &str = "matrix_typing_set";
const PRESENCE_SNAPSHOT_COMMAND: &str = "matrix_presence_snapshot";
const PRESENCE_SUBSCRIBE_COMMAND: &str = "matrix_presence_subscribe";
const PRESENCE_UNSUBSCRIBE_COMMAND: &str = "matrix_presence_unsubscribe";
const TYPING_SNAPSHOT_NO_SESSION_CODE: &str = "p2-typing-snapshot-no-session";
const TYPING_SET_NO_SESSION_CODE: &str = "p2-typing-set-no-session";
const PRESENCE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-presence-snapshot-no-session";
const PRESENCE_SUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-subscribe-no-session";
const PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-unsubscribe-no-session";
const TYPING_NO_SESSION_DESCRIPTION: &str = "No typing session is available.";
const PRESENCE_NO_SESSION_DESCRIPTION: &str = "No presence session is available.";
const TYPING_SNAPSHOT_FAILED_CODE: &str = "p4-s7-typing-snapshot-failed";
const TYPING_SNAPSHOT_FAILED_DESCRIPTION: &str = "The typing snapshot could not be loaded.";
const TYPING_SET_FAILED_CODE: &str = "p4-s7-typing-set-failed";
const TYPING_SET_FAILED_DESCRIPTION: &str = "The typing notice could not be updated.";
const TYPING_ROOM_MISSING_CODE: &str = "v-rooms.4-typing-room-missing";
const TYPING_ROOM_MISSING_DESCRIPTION: &str = "The typing room is not available.";
const TYPING_INVALID_ROOM_CODE: &str = "v-rooms.4-typing-invalid-room";
const TYPING_INVALID_ROOM_DESCRIPTION: &str = "The typing room id is invalid.";
const PRESENCE_SNAPSHOT_FAILED_CODE: &str = "p4-s7-presence-snapshot-failed";
const PRESENCE_SNAPSHOT_FAILED_DESCRIPTION: &str = "The presence snapshot could not be loaded.";
const PRESENCE_SUBSCRIBE_FAILED_CODE: &str = "p4-s7-presence-subscribe-failed";
const PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION: &str =
    "The presence subscription could not be created.";
const PRESENCE_UNSUBSCRIBE_FAILED_CODE: &str = "p4-s7-presence-unsubscribe-failed";
const PRESENCE_UNSUBSCRIBE_FAILED_DESCRIPTION: &str =
    "The presence subscription could not be released.";
const PRESENCE_INVALID_USER_CODE: &str = "v-presence-invalid-user-id";
const PRESENCE_INVALID_USER_DESCRIPTION: &str = "The presence user id is invalid.";
const PRESENCE_INVALID_SUBSCRIPTION_CODE: &str = "v-presence-invalid-subscription-id";
const PRESENCE_INVALID_SUBSCRIPTION_DESCRIPTION: &str = "The presence subscription id is invalid.";
const VERIFICATION_LIST_COMMAND: &str = "matrix_verification_list";
const VERIFICATION_LIST_GENERATION: u64 = 0;
const VERIFICATION_LIST_NO_SESSION_CODE: &str = "p2-verification-list-no-session";
const VERIFICATION_LIST_NO_SESSION_DESCRIPTION: &str = "No verification session is available.";
const VERIFICATION_LIST_FAILED_CODE: &str = "p4-s8-list-failed";
const VERIFICATION_LIST_FAILED_DESCRIPTION: &str = "The verification inbox could not be loaded.";
const VERIFICATION_SAS_GENERATION: u64 = 0;
const VERIFICATION_START_COMMAND: &str = "matrix_verification_start";
const VERIFICATION_ACCEPT_COMMAND: &str = "matrix_verification_accept";
const VERIFICATION_BEGIN_SAS_COMMAND: &str = "matrix_verification_begin_sas";
const VERIFICATION_CONFIRM_COMMAND: &str = "matrix_verification_confirm";
const VERIFICATION_MISMATCH_COMMAND: &str = "matrix_verification_mismatch";
const VERIFICATION_CANCEL_COMMAND: &str = "matrix_verification_cancel";
const VERIFICATION_DISMISS_COMMAND: &str = "matrix_verification_dismiss";
const VERIFICATION_START_NO_SESSION_CODE: &str = "p2-verification-start-no-session";
const VERIFICATION_ACCEPT_NO_SESSION_CODE: &str = "p2-verification-accept-no-session";
const VERIFICATION_BEGIN_SAS_NO_SESSION_CODE: &str = "p2-verification-begin-sas-no-session";
const VERIFICATION_CONFIRM_NO_SESSION_CODE: &str = "p2-verification-confirm-no-session";
const VERIFICATION_MISMATCH_NO_SESSION_CODE: &str = "p2-verification-mismatch-no-session";
const VERIFICATION_CANCEL_NO_SESSION_CODE: &str = "p2-verification-cancel-no-session";
const VERIFICATION_DISMISS_NO_SESSION_CODE: &str = "p2-verification-dismiss-no-session";
const VERIFICATION_SAS_NO_SESSION_DESCRIPTION: &str = "No verification session is available.";
const VERIFICATION_SAS_FAILED_CODE: &str = "p4-s9-sas-failed";
const VERIFICATION_SAS_FAILED_DESCRIPTION: &str =
    "The verification request could not be completed.";
const VERIFICATION_SAS_OWNER_DESCRIPTION: &str = "The verification request is not available.";
const DEVICE_COMMAND_GENERATION: u64 = 0;
const DEVICE_SNAPSHOT_COMMAND: &str = "matrix_device_snapshot";
const DEVICE_RENAME_COMMAND: &str = "matrix_device_rename";
const DEVICE_DELETE_START_COMMAND: &str = "matrix_device_delete_start";
const DEVICE_DELETE_CANCEL_COMMAND: &str = "matrix_device_delete_cancel";
const DEVICE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-device-snapshot-no-session";
const DEVICE_RENAME_NO_SESSION_CODE: &str = "p2-device-rename-no-session";
const DEVICE_DELETE_START_NO_SESSION_CODE: &str = "p2-device-delete-start-no-session";
const DEVICE_DELETE_CANCEL_NO_SESSION_CODE: &str = "p2-device-delete-cancel-no-session";
const DEVICE_NO_SESSION_DESCRIPTION: &str = "No device session is available.";
const DEVICE_FAILED_CODE: &str = "p4-s9-2-device-failed";
const DEVICE_FAILED_DESCRIPTION: &str = "The device request could not be completed.";
const DEVICE_OWNER_DESCRIPTION: &str = "The device request is not available.";
const JOIN_RULE_SNAPSHOT_COMMAND: &str = "matrix_room_join_rule_snapshot";
const JOIN_RULE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-join-rule-snapshot-no-session";
const JOIN_RULE_NO_SESSION_DESCRIPTION: &str = "No join-rule session is available.";
const JOIN_RULE_FAILED_CODE: &str = "p4-s9-3-join-rule-failed";
const JOIN_RULE_FAILED_DESCRIPTION: &str = "The join-rule request could not be completed.";
const JOIN_RULE_OWNER_DESCRIPTION: &str = "The join-rule request is not available.";
const IMAGE_PACK_COMMAND_GENERATION: u64 = 0;
const GET_GLOBAL_IMAGE_PACKS_COMMAND: &str = "matrix_get_global_image_packs";
const GET_USER_IMAGE_PACK_COMMAND: &str = "matrix_get_user_image_pack";
const GET_ROOM_IMAGE_PACKS_COMMAND: &str = "matrix_get_room_image_packs";
const SET_USER_IMAGE_PACK_COMMAND: &str = "matrix_set_user_image_pack";
const SET_GLOBAL_IMAGE_PACKS_COMMAND: &str = "matrix_set_global_image_packs";
const SET_ROOM_IMAGE_PACK_COMMAND: &str = "matrix_set_room_image_pack";
const GET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-global-image-packs-no-session";
const GET_USER_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-user-image-pack-no-session";
const GET_ROOM_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-room-image-packs-no-session";
const SET_USER_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-set-user-image-pack-no-session";
const SET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE: &str = "p2-set-global-image-packs-no-session";
const SET_ROOM_IMAGE_PACK_NO_SESSION_CODE: &str = "p2-set-room-image-pack-no-session";
const IMAGE_PACK_NO_SESSION_DESCRIPTION: &str = "No image-pack session is available.";
const IMAGE_PACK_FAILED_CODE: &str = "p4-s9-4-image-pack-failed";
const IMAGE_PACK_FAILED_DESCRIPTION: &str = "The image-pack request could not be completed.";
const IMAGE_PACK_INVALID_JSON_CODE: &str = "p4-s9-4-image-pack-invalid-json";
const IMAGE_PACK_INVALID_JSON_DESCRIPTION: &str = "The image-pack content is invalid.";
const IMAGE_PACK_OWNER_DESCRIPTION: &str = "The image-pack request is not available.";
const LATER_COMMAND_GENERATION: u64 = 0;
const LATER_SNAPSHOT_COMMAND: &str = "matrix_later_snapshot";
const LATER_UPSERT_COMMAND: &str = "matrix_later_upsert";
const LATER_COMPLETE_COMMAND: &str = "matrix_later_complete";
const LATER_SNOOZE_COMMAND: &str = "matrix_later_snooze";
const LATER_CLEAR_COMPLETED_COMMAND: &str = "matrix_later_clear_completed";
const LATER_MARK_REMINDED_COMMAND: &str = "matrix_later_mark_reminded";
const LATER_SNAPSHOT_NO_SESSION_CODE: &str = "p2-later-snapshot-no-session";
const LATER_UPSERT_NO_SESSION_CODE: &str = "p2-later-upsert-no-session";
const LATER_COMPLETE_NO_SESSION_CODE: &str = "p2-later-complete-no-session";
const LATER_SNOOZE_NO_SESSION_CODE: &str = "p2-later-snooze-no-session";
const LATER_CLEAR_COMPLETED_NO_SESSION_CODE: &str = "p2-later-clear-completed-no-session";
const LATER_MARK_REMINDED_NO_SESSION_CODE: &str = "p2-later-mark-reminded-no-session";
const LATER_NO_SESSION_DESCRIPTION: &str = "No later session is available.";
const LATER_FAILED_CODE: &str = "p4-s9-5-later-failed";
const LATER_FAILED_DESCRIPTION: &str = "The later request could not be completed.";
const LATER_INVALID_ITEM_CODE: &str = "p4-s9-5-later-invalid-item";
const LATER_INVALID_ITEM_DESCRIPTION: &str = "The later item is invalid.";
const LATER_OWNER_DESCRIPTION: &str = "The later request is not available.";
const MDIRECT_COMMAND_GENERATION: u64 = 0;
const MDIRECT_SNAPSHOT_COMMAND: &str = "matrix_mdirect_snapshot";
const MDIRECT_ADD_COMMAND: &str = "matrix_mdirect_add";
const MDIRECT_REMOVE_COMMAND: &str = "matrix_mdirect_remove";
const MDIRECT_SNAPSHOT_NO_SESSION_CODE: &str = "p2-mdirect-snapshot-no-session";
const MDIRECT_ADD_NO_SESSION_CODE: &str = "p2-mdirect-add-no-session";
const MDIRECT_REMOVE_NO_SESSION_CODE: &str = "p2-mdirect-remove-no-session";
const MDIRECT_NO_SESSION_DESCRIPTION: &str = "No m.direct session is available.";
const MDIRECT_FAILED_CODE: &str = "p4-s9-6-mdirect-failed";
const MDIRECT_FAILED_DESCRIPTION: &str = "The m.direct request could not be completed.";
const MDIRECT_OWNER_DESCRIPTION: &str = "The m.direct request is not available.";
const ROOM_NOTES_COMMAND_GENERATION: u64 = 0;
const ROOM_NOTES_SNAPSHOT_COMMAND: &str = "matrix_room_notes_snapshot";
const ROOM_NOTES_UPSERT_COMMAND: &str = "matrix_room_notes_upsert";
const ROOM_NOTES_DELETE_COMMAND: &str = "matrix_room_notes_delete";
const ROOM_NOTES_COMPLETE_TODO_COMMAND: &str = "matrix_room_notes_complete_todo";
const ROOM_NOTES_MOVE_TODO_COMMAND: &str = "matrix_room_notes_move_todo";
const ROOM_NOTES_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-notes-snapshot-no-session";
const ROOM_NOTES_UPSERT_NO_SESSION_CODE: &str = "p2-room-notes-upsert-no-session";
const ROOM_NOTES_DELETE_NO_SESSION_CODE: &str = "p2-room-notes-delete-no-session";
const ROOM_NOTES_COMPLETE_TODO_NO_SESSION_CODE: &str = "p2-room-notes-complete-todo-no-session";
const ROOM_NOTES_MOVE_TODO_NO_SESSION_CODE: &str = "p2-room-notes-move-todo-no-session";
const ROOM_NOTES_NO_SESSION_DESCRIPTION: &str = "No room-notes session is available.";
const ROOM_NOTES_FAILED_CODE: &str = "p4-s9-7-room-notes-failed";
const ROOM_NOTES_FAILED_DESCRIPTION: &str = "The room-notes request could not be completed.";
const ROOM_NOTES_INVALID_ITEM_CODE: &str = "p4-s9-7-room-notes-invalid-item";
const ROOM_NOTES_INVALID_ITEM_DESCRIPTION: &str = "The room-notes item is invalid.";
const ROOM_NOTES_OWNER_DESCRIPTION: &str = "The room-notes request is not available.";
const OWN_PROFILE_COMMAND_GENERATION: u64 = 0;
const SET_OWN_DISPLAY_NAME_COMMAND: &str = "matrix_set_own_display_name";
const SET_OWN_AVATAR_COMMAND: &str = "matrix_set_own_avatar";
const SET_OWN_DISPLAY_NAME_NO_SESSION_CODE: &str = "p2-set-own-display-name-no-session";
const SET_OWN_AVATAR_NO_SESSION_CODE: &str = "p2-set-own-avatar-no-session";
const OWN_PROFILE_NO_SESSION_DESCRIPTION: &str = "No own-profile session is available.";
const OWN_PROFILE_FAILED_CODE: &str = "p4-s9-8-own-profile-failed";
const OWN_PROFILE_FAILED_DESCRIPTION: &str = "The own-profile request could not be completed.";
const OWN_PROFILE_OWNER_DESCRIPTION: &str = "The own-profile request is not available.";
const ROOM_PROFILE_COMMAND_GENERATION: u64 = 0;
const SET_ROOM_NAME_COMMAND: &str = "matrix_set_room_name";
const SET_ROOM_TOPIC_COMMAND: &str = "matrix_set_room_topic";
const SET_ROOM_AVATAR_COMMAND: &str = "matrix_set_room_avatar";
const SET_ROOM_NAME_NO_SESSION_CODE: &str = "p2-set-room-name-no-session";
const SET_ROOM_TOPIC_NO_SESSION_CODE: &str = "p2-set-room-topic-no-session";
const SET_ROOM_AVATAR_NO_SESSION_CODE: &str = "p2-set-room-avatar-no-session";
const ROOM_PROFILE_NO_SESSION_DESCRIPTION: &str = "No room-profile session is available.";
const ROOM_PROFILE_FAILED_CODE: &str = "p4-s9-9-room-profile-failed";
const ROOM_PROFILE_FAILED_DESCRIPTION: &str = "The room-profile request could not be completed.";
const ROOM_PROFILE_OWNER_DESCRIPTION: &str = "The room-profile request is not available.";
const GET_ROOM_DIRECTORY_VISIBILITY_COMMAND: &str = "matrix_get_room_directory_visibility";
const SET_ROOM_DIRECTORY_VISIBILITY_COMMAND: &str = "matrix_set_room_directory_visibility";
const GET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE: &str =
    "p2-get-room-directory-visibility-no-session";
const SET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE: &str =
    "p2-set-room-directory-visibility-no-session";
const DIRECTORY_VISIBILITY_NO_SESSION_DESCRIPTION: &str =
    "No room-directory-visibility session is available.";
const DIRECTORY_VISIBILITY_FAILED_CODE: &str = "p4-s9-10-directory-visibility-failed";
const DIRECTORY_VISIBILITY_FAILED_DESCRIPTION: &str =
    "The room-directory-visibility request could not be completed.";
const DIRECTORY_VISIBILITY_OWNER_DESCRIPTION: &str =
    "The room-directory-visibility request is not available.";
const DIRECTORY_SEARCH_ENVELOPE_GENERATION: u64 = 0;
const ROOM_DIRECTORY_PROTOCOLS_COMMAND: &str = "matrix_room_directory_protocols";
const ROOM_DIRECTORY_SEARCH_COMMAND: &str = "matrix_room_directory_search";
const ROOM_DIRECTORY_CANCEL_COMMAND: &str = "matrix_room_directory_cancel";
const ROOM_DIRECTORY_PROTOCOLS_NO_SESSION_CODE: &str = "p2-room-directory-protocols-no-session";
const ROOM_DIRECTORY_SEARCH_NO_SESSION_CODE: &str = "p2-room-directory-search-no-session";
const ROOM_DIRECTORY_CANCEL_NO_SESSION_CODE: &str = "p2-room-directory-cancel-no-session";
const DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION: &str =
    "No room-directory-search session is available.";
const DIRECTORY_SEARCH_FAILED_CODE: &str = "p4-s9-11-directory-search-failed";
const DIRECTORY_SEARCH_FAILED_DESCRIPTION: &str =
    "The room-directory-search request could not be completed.";
const DIRECTORY_SEARCH_OWNER_DESCRIPTION: &str =
    "The room-directory-search request is not available.";
const ROOM_MEMBERSHIP_COMMAND_GENERATION: u64 = 0;
const ROOM_LEAVE_COMMAND: &str = "matrix_room_leave";
const ROOM_JOIN_COMMAND: &str = "matrix_room_join";
const ROOM_LEAVE_NO_SESSION_CODE: &str = "p2-room-leave-no-session";
const ROOM_JOIN_NO_SESSION_CODE: &str = "p2-room-join-no-session";
const ROOM_MEMBERSHIP_NO_SESSION_DESCRIPTION: &str = "No room-membership session is available.";
const ROOM_MEMBERSHIP_FAILED_CODE: &str = "p4-s9-12-room-membership-failed";
const ROOM_MEMBERSHIP_FAILED_DESCRIPTION: &str =
    "The room-membership request could not be completed.";
const ROOM_MEMBERSHIP_OWNER_DESCRIPTION: &str = "The room-membership request is not available.";
const ROOM_MODERATION_COMMAND_GENERATION: u64 = 0;
const ROOM_INVITE_COMMAND: &str = "matrix_room_invite";
const ROOM_KICK_COMMAND: &str = "matrix_room_kick";
const ROOM_BAN_COMMAND: &str = "matrix_room_ban";
const ROOM_UNBAN_COMMAND: &str = "matrix_room_unban";
const ROOM_INVITE_NO_SESSION_CODE: &str = "p2-room-invite-no-session";
const ROOM_KICK_NO_SESSION_CODE: &str = "p2-room-kick-no-session";
const ROOM_BAN_NO_SESSION_CODE: &str = "p2-room-ban-no-session";
const ROOM_UNBAN_NO_SESSION_CODE: &str = "p2-room-unban-no-session";
const ROOM_MODERATION_NO_SESSION_DESCRIPTION: &str = "No room-moderation session is available.";
const ROOM_MODERATION_FAILED_CODE: &str = "p4-s9-13-room-moderation-failed";
const ROOM_MODERATION_FAILED_DESCRIPTION: &str =
    "The room-moderation request could not be completed.";
const ROOM_MODERATION_OWNER_DESCRIPTION: &str = "The room-moderation request is not available.";
const ROOM_POWER_LEVEL_COMMAND_GENERATION: u64 = 0;
const ROOM_SET_POWER_LEVEL_COMMAND: &str = "matrix_room_set_power_level";
const ROOM_SET_POWER_LEVELS_COMMAND: &str = "matrix_room_set_power_levels";
const ROOM_SET_POWER_LEVEL_TAGS_COMMAND: &str = "matrix_room_set_power_level_tags";
const ROOM_SET_POWER_LEVEL_NO_SESSION_CODE: &str = "p2-room-set-power-level-no-session";
const ROOM_SET_POWER_LEVELS_NO_SESSION_CODE: &str = "p2-room-set-power-levels-no-session";
const ROOM_SET_POWER_LEVEL_TAGS_NO_SESSION_CODE: &str = "p2-room-set-power-level-tags-no-session";
const ROOM_POWER_LEVEL_NO_SESSION_DESCRIPTION: &str = "No room-power-level session is available.";
const ROOM_POWER_LEVEL_FAILED_CODE: &str = "p4-s9-14-room-power-levels-failed";
const ROOM_POWER_LEVEL_FAILED_DESCRIPTION: &str =
    "The room-power-level request could not be completed.";
const ROOM_POWER_LEVEL_OWNER_DESCRIPTION: &str = "The room-power-level request is not available.";
const ROOM_CREATE_COMMAND_GENERATION: u64 = 0;
const ROOM_CREATE_COMMAND: &str = "matrix_room_create";
const ROOM_CREATE_NO_SESSION_CODE: &str = "p2-room-create-no-session";
const ROOM_CREATE_NO_SESSION_DESCRIPTION: &str = "No room-create session is available.";
const ROOM_CREATE_FAILED_CODE: &str = "p4-s9-15-room-create-failed";
const ROOM_CREATE_FAILED_DESCRIPTION: &str = "The room-create request could not be completed.";
const ROOM_CREATE_OWNER_DESCRIPTION: &str = "The room-create request is not available.";
const ROOM_MEMBERS_SNAPSHOT_COMMAND_GENERATION: u64 = 0;
const ROOM_MEMBERS_SNAPSHOT_COMMAND: &str = "matrix_room_members_snapshot";
const ROOM_POWER_LEVELS_SNAPSHOT_COMMAND: &str = "matrix_room_power_levels_snapshot";
const ROOM_CREATORS_SNAPSHOT_COMMAND: &str = "matrix_room_creators_snapshot";
const ROOM_POWER_LEVEL_TAGS_SNAPSHOT_COMMAND: &str = "matrix_room_power_level_tags_snapshot";
const ROOM_MEMBERS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-members-snapshot-no-session";
const ROOM_POWER_LEVELS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-power-levels-snapshot-no-session";
const ROOM_CREATORS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-creators-snapshot-no-session";
const ROOM_POWER_LEVEL_TAGS_SNAPSHOT_NO_SESSION_CODE: &str =
    "p2-room-power-level-tags-snapshot-no-session";
const ROOM_MEMBERS_SNAPSHOT_NO_SESSION_DESCRIPTION: &str =
    "No room-members-snapshot session is available.";
const ROOM_MEMBERS_SNAPSHOT_FAILED_CODE: &str = "p4-s9-16-room-members-snapshots-failed";
const ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION: &str =
    "The room-members snapshot could not be loaded.";
const ROOM_MEMBERS_SNAPSHOT_OWNER_DESCRIPTION: &str = "The room-members snapshot is not available.";
const SPACE_COMMAND_GENERATION: u64 = 0;
const SPACE_PARENTS_SNAPSHOT_COMMAND: &str = "matrix_space_parents_snapshot";
const SPACE_HIERARCHY_SNAPSHOT_COMMAND: &str = "matrix_space_hierarchy_snapshot";
const SPACE_CHILDREN_SNAPSHOT_COMMAND: &str = "matrix_space_children_snapshot";
const SPACE_CHILD_SET_COMMAND: &str = "matrix_space_child_set";
const SPACE_CHILD_REMOVE_COMMAND: &str = "matrix_space_child_remove";
const RESTRICTED_JOIN_REPARENT_COMMAND: &str = "matrix_restricted_join_reparent";
const SPACE_PARENTS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-space-parents-snapshot-no-session";
const SPACE_HIERARCHY_SNAPSHOT_NO_SESSION_CODE: &str = "p2-space-hierarchy-snapshot-no-session";
const SPACE_CHILDREN_SNAPSHOT_NO_SESSION_CODE: &str = "p2-space-children-snapshot-no-session";
const SPACE_CHILD_SET_NO_SESSION_CODE: &str = "p2-space-child-set-no-session";
const SPACE_CHILD_REMOVE_NO_SESSION_CODE: &str = "p2-space-child-remove-no-session";
const RESTRICTED_JOIN_REPARENT_NO_SESSION_CODE: &str = "p2-restricted-join-reparent-no-session";
const SPACE_NO_SESSION_DESCRIPTION: &str = "No space session is available.";
const SPACE_FAILED_CODE: &str = "p4-s9-17-spaces-failed";
const SPACE_FAILED_DESCRIPTION: &str = "The space request could not be completed.";
const SPACE_OWNER_DESCRIPTION: &str = "The space request is not available.";
const INVITE_ACTION_GENERATION: u64 = 0;
const INVITES_ACCEPT_COMMAND: &str = "matrix_invites_accept";
const INVITES_DECLINE_COMMAND: &str = "matrix_invites_decline";
const INVITES_REPORT_SPAM_COMMAND: &str = "matrix_invites_report_spam";
const INVITES_BLOCK_SENDER_COMMAND: &str = "matrix_invites_block_sender";
const INVITES_ACCEPT_NO_SESSION_CODE: &str = "p2-invites-accept-no-session";
const INVITES_DECLINE_NO_SESSION_CODE: &str = "p2-invites-decline-no-session";
const INVITES_REPORT_SPAM_NO_SESSION_CODE: &str = "p2-invites-report-spam-no-session";
const INVITES_BLOCK_SENDER_NO_SESSION_CODE: &str = "p2-invites-block-sender-no-session";
const INVITE_ACTION_NO_SESSION_DESCRIPTION: &str = "No invite-action session is available.";
const INVITE_ACTION_FAILED_CODE: &str = "p4-s9-18-invite-actions-failed";
const INVITE_ACTION_FAILED_DESCRIPTION: &str = "The invite action could not be completed.";
const INVITE_ACTION_OWNER_DESCRIPTION: &str = "The invite action is not available.";

/// Static fail-closed vault error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IosSecretVaultError {
    Unavailable { code: String, description: String },
}

/// Swift-owned key/value secret store described by the existing UDL callback.
///
/// UniFFI UDL mode generates glue only; the trait itself must live in Rust.
pub trait IosSecretVault: Send + Sync {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError>;
    fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError>;
    fn delete(&self, key: String) -> Result<(), IosSecretVaultError>;
}

impl std::fmt::Display for IosSecretVaultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IosSecretVaultError {}

/// Privacy-safe restore outcome. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestoreDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionRestoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionRestoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionRestoreError {}

fn restore_failed(code: &'static str, description: &'static str) -> SessionRestoreError {
    SessionRestoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe login outcome. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLoginDto {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
}

/// Static fail-closed login error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLoginError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionLoginError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionLoginError {}

fn login_failed(code: &'static str, description: &'static str) -> SessionLoginError {
    SessionLoginError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe attach outcome. Owner names only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAttachDto {
    pub owners: Vec<String>,
}

/// Static fail-closed attach error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAttachError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionAttachError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionAttachError {}

fn attach_failed(code: &'static str, description: &'static str) -> SessionAttachError {
    SessionAttachError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe room-list snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListSnapshotDto {
    pub session_generation: u64,
    pub ordered_room_ids: Vec<String>,
    pub rooms: Vec<RoomListRoomDto>,
}

/// One privacy-safe room-list row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListRoomDto {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub avatar_url: Option<String>,
    pub membership: String,
    pub is_direct: bool,
    pub is_space: bool,
    pub is_favorite: bool,
    pub unread_count: u32,
    pub highlight_count: u32,
    pub marked_unread: bool,
    pub last_activity_ts: Option<u64>,
}

/// Static fail-closed room-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomListSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomListSnapshotError {}

fn room_list_failed(code: &'static str, description: &'static str) -> RoomListSnapshotError {
    RoomListSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_list_core_error(error: MatrixIpcError) -> RoomListSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some("p2-room-list-snapshot-no-session") => {
            room_list_failed(ROOM_LIST_NO_SESSION_CODE, ROOM_LIST_NO_SESSION_DESCRIPTION)
        }
        Some(
            "d0.2-room-list-snapshot-timeout"
            | "d0.2-room-list-stream-ended"
            | "d0.2-room-list-reset-missing"
            | "d0.2-room-list-open-failed"
            | "d0.2-room-list-filter-failed",
        ) => room_list_failed(
            ROOM_LIST_SYNC_NOT_STARTED_CODE,
            ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION,
        ),
        _ => room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe invite snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteSnapshotDto {
    pub session_generation: u64,
    pub invites: Vec<InviteDto>,
}

/// One privacy-safe invite row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InviteDto {
    pub room_id: String,
    pub room_name: String,
    pub avatar_handle_id: Option<String>,
    pub room_topic: Option<String>,
    pub room_alias: Option<String>,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_ignored: bool,
    pub invite_ts: Option<u64>,
    pub reason: Option<String>,
    pub is_space: bool,
    pub is_direct: bool,
    pub is_encrypted: bool,
    pub triage: String,
}

/// Static fail-closed invite-snapshot error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InviteSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for InviteSnapshotError {}

fn invites_failed(code: &'static str, description: &'static str) -> InviteSnapshotError {
    InviteSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_invites_core_error(error: MatrixIpcError) -> InviteSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some(
            "p2-invites-snapshot-no-session"
            | "v-rooms.1-invites-requires-session"
            | "v-send.r-room-profile-join-rule-requires-session",
        ) => invites_failed(INVITES_NO_SESSION_CODE, INVITES_NO_SESSION_DESCRIPTION),
        _ => invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION),
    }
}

fn invite_dto(invite: NativeInvite) -> InviteDto {
    InviteDto {
        room_id: invite.room_id,
        room_name: invite.room_name,
        avatar_handle_id: invite.avatar_handle_id,
        room_topic: invite.room_topic,
        room_alias: invite.room_alias,
        sender_id: invite.sender_id,
        sender_name: invite.sender_name,
        sender_ignored: invite.sender_ignored,
        invite_ts: invite.invite_ts,
        reason: invite.reason,
        is_space: invite.is_space,
        is_direct: invite.is_direct,
        is_encrypted: invite.is_encrypted,
        triage: match invite.triage {
            NativeInviteTriage::Known => "known".to_owned(),
            NativeInviteTriage::Public => "public".to_owned(),
            NativeInviteTriage::Spam => "spam".to_owned(),
        },
    }
}

/// Requested open placement. Kind is a closed string; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOpenPositionDto {
    pub kind: String,
    pub at_bottom: bool,
    pub restored_anchor_event_id: Option<String>,
    pub live_tail_event_id: Option<String>,
    pub updated_at_ms: Option<u64>,
    pub event_id: Option<String>,
}

/// Privacy-safe resolved view placement. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewPositionDto {
    pub kind: String,
    pub event_id: Option<String>,
}

/// Privacy-safe timeline snapshot. Identity/stream fields only; no token echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSnapshotDto {
    pub schema_version: u32,
    pub session_generation: u64,
    pub room_id: String,
    pub revision: u64,
    pub position: TimelineViewPositionDto,
    pub pagination_backward: String,
    pub pagination_forward: String,
    pub own_read_event_id: Option<String>,
    pub unread_anchor_event_id: Option<String>,
    pub is_marked_unread: bool,
    pub pinned_event_ids: Vec<String>,
    pub row_count: u32,
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
}

/// Privacy-safe timeline open readback. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOpenDto {
    pub schema_version: u32,
    pub stream_id: String,
    pub position: TimelineViewPositionDto,
    pub snapshot: TimelineSnapshotDto,
}

/// Static fail-closed timeline error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineError {}

fn timeline_failed(code: &'static str, description: &'static str) -> TimelineError {
    TimelineError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_open_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-open-no-session") => timeline_failed(
            TIMELINE_OPEN_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-normal-room-not-found") => timeline_failed(
            TIMELINE_ROOM_NOT_FOUND_CODE,
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-room-not-found") => timeline_failed(
            "d0.3-timeline-room-not-found",
            TIMELINE_ROOM_NOT_FOUND_DESCRIPTION,
        ),
        Some("d0.3-timeline-invalid-room-id") => timeline_failed(
            TIMELINE_INVALID_ROOM_CODE,
            TIMELINE_INVALID_ROOM_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION),
    }
}

fn map_timeline_close_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-close-no-session") => timeline_failed(
            TIMELINE_CLOSE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_CLOSE_FAILED_CODE,
            TIMELINE_CLOSE_FAILED_DESCRIPTION,
        ),
    }
}

fn map_timeline_paginate_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-paginate-no-session") => timeline_failed(
            TIMELINE_PAGINATE_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        ),
    }
}

fn open_position_from_dto(
    position: TimelineOpenPositionDto,
) -> Result<NativeTimelineOpenPosition, TimelineError> {
    match position.kind.as_str() {
        "live_bottom" => Ok(NativeTimelineOpenPosition::LiveBottom),
        "unread" => Ok(NativeTimelineOpenPosition::Unread),
        "focused" => {
            let event_id = position
                .event_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
                })?;
            Ok(NativeTimelineOpenPosition::Focused { event_id })
        }
        "normal" => Ok(NativeTimelineOpenPosition::Normal {
            viewport: NativeTimelineViewportHint {
                at_bottom: position.at_bottom,
                restored_anchor_event_id: position.restored_anchor_event_id,
                live_tail_event_id: position.live_tail_event_id,
                updated_at_ms: position.updated_at_ms,
            },
        }),
        _ => Err(timeline_failed(
            TIMELINE_OPEN_FAILED_CODE,
            TIMELINE_OPEN_FAILED_DESCRIPTION,
        )),
    }
}

fn paginate_direction(direction: &str) -> Result<NativeTimelineDirection, TimelineError> {
    match direction {
        "backwards" => Ok(NativeTimelineDirection::Backwards),
        "forwards" => Ok(NativeTimelineDirection::Forwards),
        _ => Err(timeline_failed(
            TIMELINE_PAGINATE_FAILED_CODE,
            TIMELINE_PAGINATE_FAILED_DESCRIPTION,
        )),
    }
}

fn page_state_as_str(state: TimelinePageState) -> String {
    match state {
        TimelinePageState::Available => "available",
        TimelinePageState::Exhausted => "exhausted",
        TimelinePageState::Loading => "loading",
        TimelinePageState::Unavailable => "unavailable",
    }
    .to_owned()
}

fn view_position_dto(position: TimelineViewPosition) -> TimelineViewPositionDto {
    match position {
        TimelineViewPosition::LiveBottom => TimelineViewPositionDto {
            kind: "live_bottom".to_owned(),
            event_id: None,
        },
        TimelineViewPosition::Unread { anchor_event_id } => TimelineViewPositionDto {
            kind: "unread".to_owned(),
            event_id: Some(anchor_event_id),
        },
        TimelineViewPosition::Focused { target_event_id } => TimelineViewPositionDto {
            kind: "focused".to_owned(),
            event_id: Some(target_event_id),
        },
        TimelineViewPosition::Restored { anchor_event_id } => TimelineViewPositionDto {
            kind: "restored".to_owned(),
            event_id: anchor_event_id,
        },
    }
}

fn timeline_snapshot_dto(snapshot: TimelineViewSnapshot) -> TimelineSnapshotDto {
    TimelineSnapshotDto {
        schema_version: snapshot.schema_version,
        session_generation: snapshot.session_generation,
        room_id: snapshot.room_id,
        revision: snapshot.revision,
        position: view_position_dto(snapshot.position),
        pagination_backward: page_state_as_str(snapshot.pagination.backward),
        pagination_forward: page_state_as_str(snapshot.pagination.forward),
        own_read_event_id: snapshot.read_state.own_read_event_id,
        unread_anchor_event_id: snapshot.read_state.unread_anchor_event_id,
        is_marked_unread: snapshot.read_state.is_marked_unread,
        pinned_event_ids: snapshot.pinned_event_ids,
        row_count: u32::try_from(snapshot.rows.len()).unwrap_or(u32::MAX),
        mark_read: snapshot.capabilities.mark_read,
        mark_unread: snapshot.capabilities.mark_unread,
        paginate_backward: snapshot.capabilities.paginate_backward,
        paginate_forward: snapshot.capabilities.paginate_forward,
    }
}

/// Privacy-safe typing room row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingRoomDto {
    pub room_id: String,
    pub user_ids: Vec<String>,
}

/// Privacy-safe typing snapshot. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingSnapshotDto {
    pub session_generation: u64,
    pub rooms: Vec<TypingRoomDto>,
}

/// Static fail-closed typing error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypingCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TypingCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TypingCommandError {}

fn typing_failed(code: &'static str, description: &'static str) -> TypingCommandError {
    TypingCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_typing_snapshot_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-snapshot-no-session") => typing_failed(
            TYPING_SNAPSHOT_NO_SESSION_CODE,
            TYPING_NO_SESSION_DESCRIPTION,
        ),
        _ => typing_failed(
            TYPING_SNAPSHOT_FAILED_CODE,
            TYPING_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

fn map_typing_set_core_error(error: MatrixIpcError) -> TypingCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-typing-set-no-session") => {
            typing_failed(TYPING_SET_NO_SESSION_CODE, TYPING_NO_SESSION_DESCRIPTION)
        }
        Some("v-rooms.4-typing-invalid-room") => {
            typing_failed(TYPING_INVALID_ROOM_CODE, TYPING_INVALID_ROOM_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-missing") => {
            typing_failed(TYPING_ROOM_MISSING_CODE, TYPING_ROOM_MISSING_DESCRIPTION)
        }
        Some("v-rooms.4-typing-room-not-joined") => typing_failed(
            "v-rooms.4-typing-room-not-joined",
            TYPING_ROOM_MISSING_DESCRIPTION,
        ),
        _ => typing_failed(TYPING_SET_FAILED_CODE, TYPING_SET_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe presence snapshot. Identity fields only; no tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub user_id: String,
    pub state: Option<String>,
    pub currently_active: bool,
    pub last_active_ts: Option<u64>,
    pub status_msg: Option<String>,
}

/// Privacy-safe presence subscription. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSubscriptionDto {
    pub subscription_id: String,
    pub user_id: String,
    pub session_generation: u64,
}

/// Static fail-closed presence error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PresenceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PresenceCommandError {}

fn presence_failed(code: &'static str, description: &'static str) -> PresenceCommandError {
    PresenceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_presence_snapshot_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-snapshot-no-session") => presence_failed(
            PRESENCE_SNAPSHOT_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SNAPSHOT_FAILED_CODE,
            PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

fn map_presence_subscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-subscribe-no-session") => presence_failed(
            PRESENCE_SUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-user-id") => presence_failed(
            PRESENCE_INVALID_USER_CODE,
            PRESENCE_INVALID_USER_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_SUBSCRIBE_FAILED_CODE,
            PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

fn map_presence_unsubscribe_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-unsubscribe-no-session") => presence_failed(
            PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-invalid-subscription-id") => presence_failed(
            PRESENCE_INVALID_SUBSCRIPTION_CODE,
            PRESENCE_INVALID_SUBSCRIPTION_DESCRIPTION,
        ),
        _ => presence_failed(
            PRESENCE_UNSUBSCRIBE_FAILED_CODE,
            PRESENCE_UNSUBSCRIBE_FAILED_DESCRIPTION,
        ),
    }
}

fn presence_state_as_str(state: NativePresenceState) -> String {
    match state {
        NativePresenceState::Unknown => "unknown",
        NativePresenceState::Offline => "offline",
        NativePresenceState::Online => "online",
        NativePresenceState::Unavailable => "unavailable",
    }
    .to_owned()
}

fn presence_snapshot_dto(result: NativePresenceSnapshotResult) -> PresenceSnapshotDto {
    match result {
        NativePresenceSnapshotResult::Ready {
            session_generation,
            user_id,
            snapshot,
        } => PresenceSnapshotDto {
            status: "ready".to_owned(),
            session_generation,
            user_id,
            state: Some(presence_state_as_str(snapshot.state)),
            currently_active: snapshot.currently_active,
            last_active_ts: snapshot.last_active_ts,
            status_msg: snapshot.status_msg,
        },
        NativePresenceSnapshotResult::Unknown {
            session_generation,
            user_id,
        } => PresenceSnapshotDto {
            status: "unknown".to_owned(),
            session_generation,
            user_id,
            state: None,
            currently_active: false,
            last_active_ts: None,
            status_msg: None,
        },
    }
}

/// Privacy-safe SAS emoji. User-visible comparison only; no key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationEmojiDto {
    pub symbol: String,
    pub description: String,
}

/// Privacy-safe SAS comparison. Emoji/decimals only; no tokens or MACs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationSasDto {
    pub emoji: Option<Vec<VerificationEmojiDto>>,
    pub decimals: Option<Vec<u16>>,
}

/// Privacy-safe verification request row. Identity/flow fields only; no tokens.
/// S8 list omits SAS. S9 mutation returns may include optional SAS comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationRequestDto {
    pub flow_id: String,
    pub other_user_id: String,
    pub other_device_id: Option<String>,
    pub direction: String,
    pub phase: String,
    pub started_ts: Option<u64>,
    pub sas: Option<VerificationSasDto>,
}

/// Privacy-safe verification inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationInboxDto {
    pub session_generation: u64,
    pub requests: Vec<VerificationRequestDto>,
}

/// Static fail-closed verification-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationListError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationListError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationListError {}

fn verification_list_failed(
    code: &'static str,
    description: &'static str,
) -> VerificationListError {
    VerificationListError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_verification_list_core_error(error: MatrixIpcError) -> VerificationListError {
    match error.diagnostic_id.as_deref() {
        Some("p2-verification-list-no-session") => verification_list_failed(
            VERIFICATION_LIST_NO_SESSION_CODE,
            VERIFICATION_LIST_NO_SESSION_DESCRIPTION,
        ),
        _ => verification_list_failed(
            VERIFICATION_LIST_FAILED_CODE,
            VERIFICATION_LIST_FAILED_DESCRIPTION,
        ),
    }
}

fn verification_direction_as_str(direction: NativeVerificationDirection) -> String {
    match direction {
        NativeVerificationDirection::Incoming => "incoming",
        NativeVerificationDirection::Outgoing => "outgoing",
    }
    .to_owned()
}

fn verification_phase_as_str(phase: NativeVerificationPhase) -> String {
    match phase {
        NativeVerificationPhase::Requested => "requested",
        NativeVerificationPhase::Ready => "ready",
        NativeVerificationPhase::Started => "started",
        NativeVerificationPhase::SasReady => "sas_ready",
        NativeVerificationPhase::Confirmed => "confirmed",
        NativeVerificationPhase::Done => "done",
        NativeVerificationPhase::Mismatched => "mismatched",
        NativeVerificationPhase::Cancelled => "cancelled",
    }
    .to_owned()
}

fn verification_request_dto(request: NativeVerificationRequest) -> VerificationRequestDto {
    VerificationRequestDto {
        flow_id: request.flow_id,
        other_user_id: request.other_user_id,
        other_device_id: request.other_device_id,
        direction: verification_direction_as_str(request.direction),
        phase: verification_phase_as_str(request.phase),
        started_ts: request.started_ts,
        sas: None,
    }
}

fn verification_emoji_dto(emoji: NativeVerificationEmoji) -> VerificationEmojiDto {
    VerificationEmojiDto {
        symbol: emoji.symbol,
        description: emoji.description,
    }
}

fn verification_sas_dto(sas: NativeVerificationSas) -> VerificationSasDto {
    VerificationSasDto {
        emoji: sas
            .emoji
            .map(|emoji| emoji.into_iter().map(verification_emoji_dto).collect()),
        decimals: sas.decimals.map(|decimals| decimals.to_vec()),
    }
}

fn verification_request_dto_with_sas(request: NativeVerificationRequest) -> VerificationRequestDto {
    VerificationRequestDto {
        flow_id: request.flow_id,
        other_user_id: request.other_user_id,
        other_device_id: request.other_device_id,
        direction: verification_direction_as_str(request.direction),
        phase: verification_phase_as_str(request.phase),
        started_ts: request.started_ts,
        sas: request.sas.map(verification_sas_dto),
    }
}

/// Static fail-closed verification-SAS error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationSasError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for VerificationSasError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for VerificationSasError {}

fn verification_sas_failed(code: &str, description: &'static str) -> VerificationSasError {
    VerificationSasError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_verification_sas_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> VerificationSasError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            verification_sas_failed(code, VERIFICATION_SAS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-crypto.1-") => {
            verification_sas_failed(code, VERIFICATION_SAS_OWNER_DESCRIPTION)
        }
        _ => verification_sas_failed(
            VERIFICATION_SAS_FAILED_CODE,
            VERIFICATION_SAS_FAILED_DESCRIPTION,
        ),
    }
}

enum RestoredClientSlot {
    Empty,
    InFlight,
    /// Retained for S3d attach after restore or login.
    Ready(Client),
}

enum OwnerAttachSlot {
    Empty,
    InFlight,
    Ready,
}

/// Retained shared Core for the iOS UniFFI boundary.
pub struct SharedCore {
    core: Core,
    secret_store: Arc<dyn SecretVault + Send + Sync>,
    restored_client: Mutex<RestoredClientSlot>,
    owner_attach: Mutex<OwnerAttachSlot>,
}

impl SharedCore {
    /// Construct a real Core with the fail-closed iOS Platform.
    pub fn new() -> Self {
        let platform = IosFailClosedPlatform::new();
        let secret_store = Platform::secret_store(&platform);
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
        }
    }

    /// Construct a real Core whose `Platform::secret_store` is the Swift vault.
    pub fn new_with_secret_store(store: Box<dyn IosSecretVault>) -> Self {
        let vault: Arc<dyn SecretVault + Send + Sync> =
            Arc::new(CallbackSecretVault { inner: store });
        let platform = IosFailClosedPlatform::with_secret_store(Arc::clone(&vault));
        Self {
            core: Core::new(Arc::new(platform)),
            secret_store: vault,
            restored_client: Mutex::new(RestoredClientSlot::Empty),
            owner_attach: Mutex::new(OwnerAttachSlot::Empty),
        }
    }

    /// Restore an already-persisted session from the S3a vault. No password.
    ///
    /// `store_root` is the shell-owned SDK store directory. It is never echoed.
    /// This is not `matrix_restore_session` and does not attach owners or
    /// expose `Core.command`.
    pub async fn restore_persisted_session(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<SessionRestoreDto, SessionRestoreError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| restore_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        let root = validate_store_root(&store_root)?;
        let claim = RestoreClaim::acquire(&self.restored_client)?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        if vault
            .get(&SessionMaterialId::from_identity(&identity))
            .map_err(|_| restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION))?
            .is_none()
        {
            return Err(restore_failed(
                MATERIAL_MISSING_CODE,
                MATERIAL_MISSING_DESCRIPTION,
            ));
        }

        let store_key = store_key_for(&self.secret_store, &identity)?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        let outcome = restore_session_from_vault(&client, &identity, &vault)
            .await
            .map_err(|error| match error {
                crate::app::lifecycle::LifecycleError::Vault {
                    diagnostic_id: "p3.6-session-material-missing",
                    ..
                } => restore_failed(MATERIAL_MISSING_CODE, MATERIAL_MISSING_DESCRIPTION),
                _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
            })?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: outcome.meta.user_id.clone(),
            device_id: outcome.meta.device_id.clone(),
            homeserver_url: outcome.meta.homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;

        if claim.commit(client).is_err() {
            let _ = self.core.close().await;
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }

        Ok(SessionRestoreDto {
            user_id: outcome.meta.user_id,
            device_id: outcome.meta.device_id,
            homeserver_url: outcome.meta.homeserver_url,
        })
    }

    /// Password login through Core, persisted into the S3a vault for S3b restore.
    ///
    /// `password` is a dedicated FFI argument. It is never stored, never copied
    /// into the DTO, never echoed, and is zeroized when this frame returns.
    /// This is not `matrix_login_password` and does not attach owners.
    pub async fn login_with_password(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        password: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let password = Zeroizing::new(password);
        self.login_with_password_inner(&user_id, &homeserver_url, &store_root, password.as_str())
            .await
    }

    async fn login_with_password_inner(
        &self,
        user_id: &str,
        homeserver_url: &str,
        store_root: &str,
        password: &str,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(user_id, homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        if password.is_empty() {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let outcome = core_login_with_password(
            &client,
            identity.user_id(),
            password,
            &LoginOptions {
                request_refresh_token: true,
                device_display_name: Some(DevicePlatform::Ios.device_display_name().to_owned()),
            },
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let live_identity = AccountIdentity::new(&outcome.user_id, &outcome.homeserver_url)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        if live_identity != identity {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        self.persist_open_and_retain(
            client,
            &live_identity,
            &vault,
            claim,
            outcome.user_id,
            outcome.device_id,
            outcome.homeserver_url,
        )
        .await
    }

    /// Test-only persist+open+retain through the production login path.
    ///
    /// Plants a Matrix session on an unauthenticated Client (no homeserver),
    /// then calls the same `store_key_for` + `persist_session_after_login` +
    /// `Core::open` + retain sequence `login_with_password` uses. Not on UDL.
    #[doc(hidden)]
    pub async fn persist_planted_session_for_test(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        device_id: String,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
            login_failed(
                LOGIN_IDENTITY_INVALID_CODE,
                LOGIN_IDENTITY_INVALID_DESCRIPTION,
            )
        })?;
        let root = parse_store_root(&store_root).map_err(|_| {
            login_failed(
                LOGIN_STORE_ROOT_INVALID_CODE,
                LOGIN_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        let claim = RestoreClaim::acquire(&self.restored_client)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let vault = SecretStoreSessionVault {
            store: Arc::clone(&self.secret_store),
        };
        let store_key =
            store_key_for(&self.secret_store, &identity).map_err(|error| match error {
                SessionRestoreError::Failed { code, .. } if code == VAULT_UNAVAILABLE_CODE => {
                    login_failed(
                        LOGIN_VAULT_UNAVAILABLE_CODE,
                        LOGIN_VAULT_UNAVAILABLE_DESCRIPTION,
                    )
                }
                _ => login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION),
            })?;
        let config = ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let material = SessionMaterial::from_matrix_tokens(
            &identity,
            &device_id,
            &access_token,
            refresh_token.as_deref(),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        restore_session_onto_client(&client, &identity, &material)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        self.persist_open_and_retain(
            client,
            &identity,
            &vault,
            claim,
            identity.user_id().to_owned(),
            device_id,
            identity.homeserver_url().to_owned(),
        )
        .await
    }

    async fn persist_open_and_retain(
        &self,
        client: Client,
        identity: &AccountIdentity,
        vault: &SecretStoreSessionVault,
        claim: RestoreClaim<'_>,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        persist_session_after_login(&client, identity, vault)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: user_id.clone(),
            device_id: device_id.clone(),
            homeserver_url: homeserver_url.clone(),
            display_name: None,
            avatar_url: None,
            lifecycle: SessionLifecycle::Ready,
            crypto_ready: false,
        };
        self.core
            .open(snapshot)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        if claim.commit(client).is_err() {
            let _ = self.core.close().await;
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }

        Ok(SessionLoginDto {
            user_id,
            device_id,
            homeserver_url,
        })
    }

    /// Attach the desktop owner set on the retained Client. No Core.command.
    ///
    /// Builds owners with no-op emit sinks (Platform::emit stays a later
    /// slice). SyncService is attached but not started so iOS does not run a
    /// second live sync while MatrixRustSDK still owns product room list.
    /// Fail-closed if no Client is retained or owners are already attached.
    pub async fn attach_session_owners(&self) -> Result<SessionAttachDto, SessionAttachError> {
        let claim = AttachClaim::acquire(&self.owner_attach)?;
        let client = {
            let guard = self
                .restored_client
                .lock()
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
            match &*guard {
                RestoredClientSlot::Ready(client) => client.clone(),
                RestoredClientSlot::Empty | RestoredClientSlot::InFlight => {
                    return Err(attach_failed(
                        ATTACH_SESSION_MISSING_CODE,
                        ATTACH_SESSION_MISSING_DESCRIPTION,
                    ));
                }
            }
        };
        let generation = self
            .core
            .session_snapshot()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?
            .ok_or_else(|| {
                attach_failed(
                    ATTACH_SESSION_MISSING_CODE,
                    ATTACH_SESSION_MISSING_DESCRIPTION,
                )
            })?
            .session_generation;
        if generation == 0 {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }

        let typing = Arc::new(
            NativeTypingOwner::start(&client, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let presence = Arc::new(
            NativePresenceOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let verification = Arc::new(NativeVerificationOwner::new(&client, generation));
        let devices = Arc::new(
            NativeDeviceOwner::start(&client, Arc::new(|_| {}), generation)
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let join_rules = Arc::new(
            NativeRoomJoinRuleOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let image_packs = Arc::new(
            NativeImagePackOwner::start(&client, Arc::new(|_| {}), generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let timelines = Arc::new(NativeTimelineOwner::new(
            &client,
            Arc::new(|_| {}),
            generation,
        ));
        let sync = Arc::new(
            build_sync_service(&client, generation, SyncServiceConfig::default())
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );

        self.core
            .attach_typing(typing)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_presence(presence)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_verification(verification)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_devices(devices)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_join_rules(join_rules)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_image_packs(image_packs)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_timelines(timelines)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        self.core
            .attach_sync(sync)
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;

        claim
            .commit()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        Ok(SessionAttachDto {
            owners: ATTACHED_OWNER_NAMES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
        })
    }

    /// Typed consume of the already-registered `matrix_room_list_snapshot`.
    ///
    /// Uses `Core::command` with the same null camelCase payload desktop
    /// sends. Does not start SyncService (no dual live sync); an unstarted
    /// owner yields the handler's empty snapshot. Does not expose a generic
    /// command FFI.
    pub async fn room_list_snapshot(&self) -> Result<RoomListSnapshotDto, RoomListSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_LIST_COMMAND.to_owned(),
                session_generation: ROOM_LIST_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_room_list_core_error)?;
        let snapshot: NativeRoomListSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION))?;
        Ok(RoomListSnapshotDto {
            session_generation: snapshot.session_generation,
            ordered_room_ids: snapshot.ordered_room_ids,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| RoomListRoomDto {
                    room_id: room.room_id,
                    name: room.name,
                    canonical_alias: room.canonical_alias,
                    avatar_url: room.avatar_url,
                    membership: room.membership.as_str().to_owned(),
                    is_direct: room.is_direct,
                    is_space: room.is_space,
                    is_favorite: room.is_favorite,
                    unread_count: room.unread_count,
                    highlight_count: room.highlight_count,
                    marked_unread: room.marked_unread,
                    last_activity_ts: room.last_activity_ts,
                })
                .collect(),
        })
    }

    pub async fn invites_snapshot(&self) -> Result<InviteSnapshotDto, InviteSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: INVITES_COMMAND.to_owned(),
                session_generation: INVITES_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_invites_core_error)?;
        let snapshot: NativeInviteSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION))?;
        Ok(InviteSnapshotDto {
            session_generation: snapshot.session_generation,
            invites: snapshot.invites.into_iter().map(invite_dto).collect(),
        })
    }

    pub async fn timeline_open(
        &self,
        room_id: String,
        position: TimelineOpenPositionDto,
    ) -> Result<TimelineOpenDto, TimelineError> {
        let position = open_position_from_dto(position)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_OPEN_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "position": position,
                }),
            })
            .await
            .map_err(map_timeline_open_core_error)?;
        let readback: NativeTimelineOpenReadback = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
            })?;
        Ok(TimelineOpenDto {
            schema_version: readback.schema_version,
            stream_id: readback.stream_id,
            position: view_position_dto(readback.position),
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn timeline_close(&self, stream_id: String) -> Result<bool, TimelineError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_CLOSE_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "streamId": stream_id }),
            })
            .await
            .map_err(map_timeline_close_core_error)?;
        serde_json::from_value(response.payload).map_err(|_| {
            timeline_failed(
                TIMELINE_CLOSE_FAILED_CODE,
                TIMELINE_CLOSE_FAILED_DESCRIPTION,
            )
        })
    }

    pub async fn timeline_paginate(
        &self,
        stream_id: String,
        direction: String,
    ) -> Result<TimelineSnapshotDto, TimelineError> {
        let direction = paginate_direction(&direction)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_PAGINATE_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "streamId": stream_id,
                    "direction": direction,
                }),
            })
            .await
            .map_err(map_timeline_paginate_core_error)?;
        let snapshot: TimelineViewSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_failed(
                    TIMELINE_PAGINATE_FAILED_CODE,
                    TIMELINE_PAGINATE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(timeline_snapshot_dto(snapshot))
    }

    pub async fn typing_snapshot(&self) -> Result<TypingSnapshotDto, TypingCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TYPING_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_typing_snapshot_core_error)?;
        let snapshot: NativeTypingSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                typing_failed(
                    TYPING_SNAPSHOT_FAILED_CODE,
                    TYPING_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TypingSnapshotDto {
            session_generation: snapshot.session_generation,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| TypingRoomDto {
                    room_id: room.room_id,
                    user_ids: room.user_ids,
                })
                .collect(),
        })
    }

    pub async fn typing_set(
        &self,
        room_id: String,
        typing: bool,
    ) -> Result<(), TypingCommandError> {
        self.core
            .command(CommandEnvelope {
                command: TYPING_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id, "typing": typing }),
            })
            .await
            .map_err(map_typing_set_core_error)?;
        Ok(())
    }

    pub async fn presence_snapshot(
        &self,
        user_id: String,
    ) -> Result<PresenceSnapshotDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_snapshot_core_error)?;
        let result: NativePresenceSnapshotResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SNAPSHOT_FAILED_CODE,
                    PRESENCE_SNAPSHOT_FAILED_DESCRIPTION,
                )
            })?;
        Ok(presence_snapshot_dto(result))
    }

    pub async fn presence_subscribe(
        &self,
        user_id: String,
    ) -> Result<PresenceSubscriptionDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "userId": user_id }),
            })
            .await
            .map_err(map_presence_subscribe_core_error)?;
        let subscription: NativePresenceSubscription = serde_json::from_value(response.payload)
            .map_err(|_| {
                presence_failed(
                    PRESENCE_SUBSCRIBE_FAILED_CODE,
                    PRESENCE_SUBSCRIBE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(PresenceSubscriptionDto {
            subscription_id: subscription.subscription_id,
            user_id: subscription.user_id,
            session_generation: subscription.session_generation,
        })
    }

    pub async fn presence_unsubscribe(
        &self,
        subscription_id: String,
    ) -> Result<(), PresenceCommandError> {
        self.core
            .command(CommandEnvelope {
                command: PRESENCE_UNSUBSCRIBE_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "subscriptionId": subscription_id }),
            })
            .await
            .map_err(map_presence_unsubscribe_core_error)?;
        Ok(())
    }

    pub async fn verification_list(&self) -> Result<VerificationInboxDto, VerificationListError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: VERIFICATION_LIST_COMMAND.to_owned(),
                session_generation: VERIFICATION_LIST_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_verification_list_core_error)?;
        let inbox: NativeVerificationInbox =
            serde_json::from_value(response.payload).map_err(|_| {
                verification_list_failed(
                    VERIFICATION_LIST_FAILED_CODE,
                    VERIFICATION_LIST_FAILED_DESCRIPTION,
                )
            })?;
        Ok(VerificationInboxDto {
            session_generation: inbox.session_generation,
            requests: inbox
                .requests
                .into_iter()
                .map(verification_request_dto)
                .collect(),
        })
    }

    pub async fn verification_start(
        &self,
        device_id: Option<String>,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: VERIFICATION_START_COMMAND.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "deviceId": device_id }),
            })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_START_NO_SESSION_CODE, error)
            })?;
        parse_verification_sas_request(response.payload)
    }

    pub async fn verification_accept(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_ACCEPT_COMMAND,
            VERIFICATION_ACCEPT_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_begin_sas(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_BEGIN_SAS_COMMAND,
            VERIFICATION_BEGIN_SAS_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_confirm(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CONFIRM_COMMAND,
            VERIFICATION_CONFIRM_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_mismatch(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_MISMATCH_COMMAND,
            VERIFICATION_MISMATCH_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_cancel(
        &self,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        self.verification_flow_command(
            VERIFICATION_CANCEL_COMMAND,
            VERIFICATION_CANCEL_NO_SESSION_CODE,
            flow_id,
        )
        .await
    }

    pub async fn verification_dismiss(&self, flow_id: String) -> Result<(), VerificationSasError> {
        self.core
            .command(CommandEnvelope {
                command: VERIFICATION_DISMISS_COMMAND.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "flowId": flow_id }),
            })
            .await
            .map_err(|error| {
                map_verification_sas_core_error(VERIFICATION_DISMISS_NO_SESSION_CODE, error)
            })?;
        Ok(())
    }

    async fn verification_flow_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        flow_id: String,
    ) -> Result<VerificationRequestDto, VerificationSasError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: VERIFICATION_SAS_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "flowId": flow_id }),
            })
            .await
            .map_err(|error| map_verification_sas_core_error(no_session, error))?;
        parse_verification_sas_request(response.payload)
    }

    pub async fn device_snapshot(&self) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .device_null_command(DEVICE_SNAPSHOT_COMMAND, DEVICE_SNAPSHOT_NO_SESSION_CODE)
            .await?;
        let snapshot: NativeDeviceSnapshot = serde_json::from_value(response)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_rename(
        &self,
        device_id: String,
        display_name: String,
    ) -> Result<DeviceSnapshotDto, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: DEVICE_RENAME_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "deviceId": device_id,
                    "displayName": display_name,
                }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_RENAME_NO_SESSION_CODE, error))?;
        let snapshot: NativeDeviceSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_snapshot_dto(snapshot))
    }

    pub async fn device_delete_start(
        &self,
        device_ids: Vec<String>,
    ) -> Result<DeviceDeleteDto, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: DEVICE_DELETE_START_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "deviceIds": device_ids }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_START_NO_SESSION_CODE, error))?;
        let result: NativeDeviceDeleteResult = serde_json::from_value(response.payload)
            .map_err(|_| device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION))?;
        Ok(device_delete_dto(result))
    }

    pub async fn device_delete_cancel(
        &self,
        operation_id: u64,
        session_generation: u64,
    ) -> Result<(), DeviceCommandError> {
        self.core
            .command(CommandEnvelope {
                command: DEVICE_DELETE_CANCEL_COMMAND.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "operationId": operation_id,
                    "sessionGeneration": session_generation,
                }),
            })
            .await
            .map_err(|error| map_device_core_error(DEVICE_DELETE_CANCEL_NO_SESSION_CODE, error))?;
        Ok(())
    }

    pub async fn room_join_rule_snapshot(
        &self,
        room_id: String,
        session_generation: u64,
    ) -> Result<RoomJoinRuleSnapshotDto, JoinRuleCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: JOIN_RULE_SNAPSHOT_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "sessionGeneration": session_generation,
                }),
            })
            .await
            .map_err(|error| map_join_rule_core_error(error))?;
        let snapshot: MatrixRoomJoinRuleSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleSnapshotDto {
            status: snapshot.status,
            room_id: snapshot.room_id,
            session_generation: snapshot.session_generation,
            join_rule: snapshot.join_rule,
        })
    }

    pub async fn get_global_image_packs(
        &self,
    ) -> Result<GlobalImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_GLOBAL_IMAGE_PACKS_COMMAND,
                GET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeGlobalImagePacksSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(GlobalImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn get_user_image_pack(
        &self,
    ) -> Result<UserImagePackSnapshotDto, ImagePackCommandError> {
        let payload = self
            .image_pack_null_command(
                GET_USER_IMAGE_PACK_COMMAND,
                GET_USER_IMAGE_PACK_NO_SESSION_CODE,
            )
            .await?;
        let snapshot: NativeUserImagePackSnapshot =
            serde_json::from_value(payload).map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(UserImagePackSnapshotDto {
            session_generation: snapshot.session_generation,
            pack: snapshot.pack.map(image_pack_dto).transpose()?,
        })
    }

    pub async fn get_room_image_packs(
        &self,
        room_id: String,
    ) -> Result<RoomImagePacksSnapshotDto, ImagePackCommandError> {
        let payload = self
            .core
            .command(CommandEnvelope {
                command: GET_ROOM_IMAGE_PACKS_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "roomId": room_id }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(GET_ROOM_IMAGE_PACKS_NO_SESSION_CODE, error)
            })?;
        let snapshot: NativeRoomImagePacksSnapshot = serde_json::from_value(payload.payload)
            .map_err(|_| {
                image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION)
            })?;
        Ok(RoomImagePacksSnapshotDto {
            session_generation: snapshot.session_generation,
            room_id: snapshot.room_id,
            packs: snapshot
                .packs
                .into_iter()
                .map(image_pack_dto)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub async fn set_user_image_pack(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_USER_IMAGE_PACK_COMMAND,
            SET_USER_IMAGE_PACK_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_global_image_packs(
        &self,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        self.image_pack_set_content(
            SET_GLOBAL_IMAGE_PACKS_COMMAND,
            SET_GLOBAL_IMAGE_PACKS_NO_SESSION_CODE,
            content_json,
        )
        .await
    }

    pub async fn set_room_image_pack(
        &self,
        room_id: String,
        state_key: String,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SET_ROOM_IMAGE_PACK_COMMAND.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "stateKey": state_key,
                    "content": content,
                }),
            })
            .await
            .map_err(|error| {
                map_image_pack_core_error(SET_ROOM_IMAGE_PACK_NO_SESSION_CODE, error)
            })?;
        image_pack_write_dto(response.payload)
    }

    pub async fn later_snapshot(&self) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_null_command(LATER_SNAPSHOT_COMMAND, LATER_SNAPSHOT_NO_SESSION_CODE)
            .await
    }

    pub async fn later_upsert(
        &self,
        item: LaterItemDto,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let item = later_item_from_dto(item)?;
        let payload = later_envelope_payload(serde_json::json!({ "item": item }))?;
        self.later_command(LATER_UPSERT_COMMAND, LATER_UPSERT_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn later_complete(
        &self,
        item_id: String,
        completed_at: Option<f64>,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "completedAt": completed_at,
        }))?;
        self.later_command(
            LATER_COMPLETE_COMMAND,
            LATER_COMPLETE_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn later_snooze(
        &self,
        item_id: String,
        due_ts: f64,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "dueTs": due_ts,
        }))?;
        self.later_command(LATER_SNOOZE_COMMAND, LATER_SNOOZE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn later_clear_completed(&self) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_null_command(
            LATER_CLEAR_COMPLETED_COMMAND,
            LATER_CLEAR_COMPLETED_NO_SESSION_CODE,
        )
        .await
    }

    pub async fn later_mark_reminded(
        &self,
        item_id: String,
        reminded_at: Option<f64>,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let payload = later_envelope_payload(serde_json::json!({
            "itemId": item_id,
            "remindedAt": reminded_at,
        }))?;
        self.later_command(
            LATER_MARK_REMINDED_COMMAND,
            LATER_MARK_REMINDED_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn mdirect_snapshot(&self) -> Result<MDirectSnapshotDto, MDirectCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_SNAPSHOT_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_SNAPSHOT_NO_SESSION_CODE, error))?;
        mdirect_snapshot_dto(response.payload)
    }

    pub async fn mdirect_add(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<MDirectMutationDto, MDirectCommandError> {
        let payload = mdirect_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_ADD_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_ADD_NO_SESSION_CODE, error))?;
        mdirect_mutation_dto(response.payload)
    }

    pub async fn mdirect_remove(
        &self,
        room_id: String,
    ) -> Result<MDirectMutationDto, MDirectCommandError> {
        let payload = mdirect_envelope_payload(serde_json::json!({ "roomId": room_id }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MDIRECT_REMOVE_COMMAND.to_owned(),
                session_generation: MDIRECT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_mdirect_core_error(MDIRECT_REMOVE_NO_SESSION_CODE, error))?;
        mdirect_mutation_dto(response.payload)
    }

    pub async fn room_notes_snapshot(&self) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        self.room_notes_null_command(
            ROOM_NOTES_SNAPSHOT_COMMAND,
            ROOM_NOTES_SNAPSHOT_NO_SESSION_CODE,
        )
        .await
    }

    pub async fn room_notes_upsert(
        &self,
        item: RoomNoteItemDto,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let item = room_note_item_from_dto(item)?;
        let payload = room_notes_envelope_payload(serde_json::json!({ "item": item }))?;
        self.room_notes_command(
            ROOM_NOTES_UPSERT_COMMAND,
            ROOM_NOTES_UPSERT_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_delete(
        &self,
        room_id: String,
        item_id: String,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_DELETE_COMMAND,
            ROOM_NOTES_DELETE_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_complete_todo(
        &self,
        room_id: String,
        item_id: String,
        completed: bool,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "completed": completed,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_COMPLETE_TODO_COMMAND,
            ROOM_NOTES_COMPLETE_TODO_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_notes_move_todo(
        &self,
        room_id: String,
        item_id: String,
        direction: String,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let direction = room_note_move_direction_from_dto(&direction)?;
        let payload = room_notes_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "direction": direction,
        }))?;
        self.room_notes_command(
            ROOM_NOTES_MOVE_TODO_COMMAND,
            ROOM_NOTES_MOVE_TODO_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_own_display_name(
        &self,
        display_name: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        let payload = own_profile_envelope_payload(serde_json::json!({
            "displayName": display_name,
        }))?;
        self.own_profile_command(
            SET_OWN_DISPLAY_NAME_COMMAND,
            SET_OWN_DISPLAY_NAME_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_own_avatar(
        &self,
        mxc: String,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        let payload = own_profile_envelope_payload(serde_json::json!({ "mxc": mxc }))?;
        self.own_profile_command(
            SET_OWN_AVATAR_COMMAND,
            SET_OWN_AVATAR_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_room_name(
        &self,
        room_id: String,
        name: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "name": name,
        }))?;
        self.room_profile_command(
            SET_ROOM_NAME_COMMAND,
            SET_ROOM_NAME_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_room_topic(
        &self,
        room_id: String,
        topic: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "topic": topic,
        }))?;
        self.room_profile_command(
            SET_ROOM_TOPIC_COMMAND,
            SET_ROOM_TOPIC_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_room_avatar(
        &self,
        room_id: String,
        mxc: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "mxc": mxc,
        }))?;
        self.room_profile_command(
            SET_ROOM_AVATAR_COMMAND,
            SET_ROOM_AVATAR_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn get_room_directory_visibility(
        &self,
        room_id: String,
        session_generation: u64,
    ) -> Result<RoomDirectoryVisibilityDto, DirectoryVisibilityCommandError> {
        let payload = directory_visibility_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "sessionGeneration": session_generation,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: GET_ROOM_DIRECTORY_VISIBILITY_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_visibility_core_error(
                    GET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE,
                    error,
                )
            })?;
        room_directory_visibility_dto(response.payload)
    }

    pub async fn set_room_directory_visibility(
        &self,
        room_id: String,
        session_generation: u64,
        visibility: String,
    ) -> Result<RoomDirectoryVisibilityWriteDto, DirectoryVisibilityCommandError> {
        let payload = directory_visibility_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "sessionGeneration": session_generation,
            "visibility": visibility,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SET_ROOM_DIRECTORY_VISIBILITY_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_visibility_core_error(
                    SET_ROOM_DIRECTORY_VISIBILITY_NO_SESSION_CODE,
                    error,
                )
            })?;
        room_directory_visibility_write_dto(response.payload)
    }

    pub async fn room_directory_protocols(
        &self,
    ) -> Result<RoomDirectoryProtocolsDto, DirectorySearchCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_PROTOCOLS_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_PROTOCOLS_NO_SESSION_CODE, error)
            })?;
        room_directory_protocols_dto(response.payload)
    }

    pub async fn room_directory_search(
        &self,
        session_generation: u64,
        request_id: u64,
        server_name: Option<String>,
        term: Option<String>,
        room_type: Option<String>,
        third_party_instance_id: Option<String>,
        limit: u64,
        since: Option<String>,
    ) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
        let payload = directory_search_envelope_payload(serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
            "serverName": server_name,
            "term": term,
            "roomType": room_type,
            "thirdPartyInstanceId": third_party_instance_id,
            "limit": limit,
            "since": since,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_SEARCH_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_SEARCH_NO_SESSION_CODE, error)
            })?;
        room_directory_search_dto(response.payload)
    }

    pub async fn room_directory_cancel(
        &self,
        session_generation: u64,
        request_id: u64,
    ) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
        let payload = directory_search_envelope_payload(serde_json::json!({
            "sessionGeneration": session_generation,
            "requestId": request_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_DIRECTORY_CANCEL_COMMAND.to_owned(),
                session_generation: DIRECTORY_SEARCH_ENVELOPE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_directory_search_core_error(ROOM_DIRECTORY_CANCEL_NO_SESSION_CODE, error)
            })?;
        room_directory_search_dto(response.payload)
    }

    pub async fn room_leave(
        &self,
        room_id: String,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        self.room_membership_command(ROOM_LEAVE_COMMAND, ROOM_LEAVE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_join(
        &self,
        room_id_or_alias: String,
        via_servers: Option<Vec<String>>,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomIdOrAlias": room_id_or_alias,
            "viaServers": via_servers,
        }))?;
        self.room_membership_command(ROOM_JOIN_COMMAND, ROOM_JOIN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_invite(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_INVITE_COMMAND, ROOM_INVITE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_kick(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_KICK_COMMAND, ROOM_KICK_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_ban(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_BAN_COMMAND, ROOM_BAN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_unban(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
        }))?;
        self.room_moderation_command(ROOM_UNBAN_COMMAND, ROOM_UNBAN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_set_power_level(
        &self,
        room_id: String,
        user_id: String,
        power_level: i64,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "powerLevel": power_level,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVEL_COMMAND,
            ROOM_SET_POWER_LEVEL_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_set_power_levels(
        &self,
        room_id: String,
        content_json: String,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let content = parse_power_level_content_json(&content_json)?;
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "content": content,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVELS_COMMAND,
            ROOM_SET_POWER_LEVELS_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_set_power_level_tags(
        &self,
        room_id: String,
        content_json: String,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let content = parse_power_level_content_json(&content_json)?;
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "content": content,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVEL_TAGS_COMMAND,
            ROOM_SET_POWER_LEVEL_TAGS_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_create(
        &self,
        request: RoomCreateRequestDto,
    ) -> Result<RoomCreateDto, RoomCreateCommandError> {
        let payload = room_create_envelope_payload(room_create_request_payload(request)?)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_CREATE_COMMAND.to_owned(),
                session_generation: ROOM_CREATE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_create_core_error(ROOM_CREATE_NO_SESSION_CODE, error))?;
        room_create_dto(response.payload)
    }

    pub async fn room_members_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomMembersSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_MEMBERS_SNAPSHOT_COMMAND,
                ROOM_MEMBERS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_members_snapshot_dto(payload)
    }

    pub async fn room_power_levels_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomPowerLevelsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_POWER_LEVELS_SNAPSHOT_COMMAND,
                ROOM_POWER_LEVELS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_power_levels_snapshot_dto(payload)
    }

    pub async fn room_creators_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomCreatorsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_CREATORS_SNAPSHOT_COMMAND,
                ROOM_CREATORS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_creators_snapshot_dto(payload)
    }

    pub async fn room_power_level_tags_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomPowerLevelTagsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_POWER_LEVEL_TAGS_SNAPSHOT_COMMAND,
                ROOM_POWER_LEVEL_TAGS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_power_level_tags_snapshot_dto(payload)
    }

    pub async fn space_parents_snapshot(
        &self,
    ) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_null_command(
                SPACE_PARENTS_SNAPSHOT_COMMAND,
                SPACE_PARENTS_SNAPSHOT_NO_SESSION_CODE,
            )
            .await?;
        space_parents_snapshot_dto(payload)
    }

    pub async fn space_hierarchy_snapshot(
        &self,
        room_id: String,
    ) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .space_command(
                SPACE_HIERARCHY_SNAPSHOT_COMMAND,
                SPACE_HIERARCHY_SNAPSHOT_NO_SESSION_CODE,
                payload,
            )
            .await?;
        space_hierarchy_snapshot_dto(response)
    }

    pub async fn space_children_snapshot(
        &self,
    ) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
        let payload = self
            .space_null_command(
                SPACE_CHILDREN_SNAPSHOT_COMMAND,
                SPACE_CHILDREN_SNAPSHOT_NO_SESSION_CODE,
            )
            .await?;
        space_children_snapshot_dto(payload)
    }

    pub async fn space_child_set(
        &self,
        parent_id: String,
        child_id: String,
        via: Vec<String>,
        order: Option<String>,
        suggested: Option<bool>,
    ) -> Result<SpaceChildMutationDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
            "via": via,
            "order": order,
            "suggested": suggested,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_SET_COMMAND,
                SPACE_CHILD_SET_NO_SESSION_CODE,
                payload,
            )
            .await?;
        space_child_mutation_dto(response)
    }

    pub async fn space_child_remove(
        &self,
        parent_id: String,
        child_id: String,
    ) -> Result<SpaceChildMutationDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "parentId": parent_id,
            "childId": child_id,
        }))?;
        let response = self
            .space_command(
                SPACE_CHILD_REMOVE_COMMAND,
                SPACE_CHILD_REMOVE_NO_SESSION_CODE,
                payload,
            )
            .await?;
        space_child_mutation_dto(response)
    }

    pub async fn restricted_join_reparent(
        &self,
        room_id: String,
        remove_parent_id: Option<String>,
        add_parent_id: String,
    ) -> Result<RestrictedJoinReparentDto, SpaceCommandError> {
        let payload = space_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "removeParentId": remove_parent_id,
            "addParentId": add_parent_id,
        }))?;
        let response = self
            .space_command(
                RESTRICTED_JOIN_REPARENT_COMMAND,
                RESTRICTED_JOIN_REPARENT_NO_SESSION_CODE,
                payload,
            )
            .await?;
        restricted_join_reparent_dto(response)
    }

    pub async fn invites_accept(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_ACCEPT_COMMAND,
            INVITES_ACCEPT_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_decline(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_DECLINE_COMMAND,
            INVITES_DECLINE_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_report_spam(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_REPORT_SPAM_COMMAND,
            INVITES_REPORT_SPAM_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_block_sender(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_BLOCK_SENDER_COMMAND,
            INVITES_BLOCK_SENDER_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    async fn space_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, SpaceCommandError> {
        self.space_command(command, no_session, serde_json::Value::Null)
            .await
    }

    async fn space_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, SpaceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: SPACE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_space_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn invite_action_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        let payload = invite_action_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: INVITE_ACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_invite_action_core_error(no_session, error))?;
        invite_action_snapshot_dto(response.payload)
    }

    async fn room_members_snapshot_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        room_id: String,
    ) -> Result<serde_json::Value, RoomMembersSnapshotError> {
        let payload = room_members_snapshot_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MEMBERS_SNAPSHOT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_members_snapshot_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn later_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        self.later_command(command, no_session, serde_json::Value::Null)
            .await
    }

    async fn later_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<LaterSnapshotDto, LaterCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: LATER_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_later_core_error(no_session, error))?;
        later_snapshot_dto(response.payload)
    }

    async fn room_notes_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        self.room_notes_command(command, no_session, serde_json::Value::Null)
            .await
    }

    async fn room_notes_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_NOTES_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_notes_core_error(no_session, error))?;
        room_notes_snapshot_dto(response.payload)
    }

    async fn own_profile_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: OWN_PROFILE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_own_profile_core_error(no_session, error))?;
        own_profile_write_dto(response.payload)
    }

    async fn room_profile_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_PROFILE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_profile_core_error(no_session, error))?;
        room_profile_write_dto(response.payload)
    }

    async fn room_membership_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MEMBERSHIP_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_membership_core_error(no_session, error))?;
        room_membership_write_dto(response.payload)
    }

    async fn room_moderation_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MODERATION_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_moderation_core_error(no_session, error))?;
        room_moderation_write_dto(response.payload)
    }

    async fn room_power_level_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_POWER_LEVEL_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_power_level_core_error(no_session, error))?;
        room_power_level_write_dto(response.payload)
    }

    async fn image_pack_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, ImagePackCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn image_pack_set_content(
        &self,
        command: &'static str,
        no_session: &'static str,
        content_json: String,
    ) -> Result<ImagePackWriteDto, ImagePackCommandError> {
        let content = parse_image_pack_content_json(&content_json)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IMAGE_PACK_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "content": content }),
            })
            .await
            .map_err(|error| map_image_pack_core_error(no_session, error))?;
        image_pack_write_dto(response.payload)
    }

    async fn device_null_command(
        &self,
        command: &'static str,
        no_session: &'static str,
    ) -> Result<serde_json::Value, DeviceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: DEVICE_COMMAND_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(|error| map_device_core_error(no_session, error))?;
        Ok(response.payload)
    }
}

fn parse_verification_sas_request(
    payload: serde_json::Value,
) -> Result<VerificationRequestDto, VerificationSasError> {
    let request: NativeVerificationRequest = serde_json::from_value(payload).map_err(|_| {
        verification_sas_failed(
            VERIFICATION_SAS_FAILED_CODE,
            VERIFICATION_SAS_FAILED_DESCRIPTION,
        )
    })?;
    Ok(verification_request_dto_with_sas(request))
}

/// Privacy-safe device row. Identity/presentation fields only; no keys or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummaryDto {
    pub device_id: String,
    pub display_name: Option<String>,
    pub last_seen_ip: Option<String>,
    pub last_seen_ts: Option<u64>,
    pub trust: String,
    pub is_current: bool,
}

/// Privacy-safe device inbox. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSnapshotDto {
    pub session_generation: u64,
    pub devices: Vec<DeviceSummaryDto>,
}

/// Privacy-safe delete challenge. Authentication type only; no password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDeleteChallengeDto {
    pub operation_id: u64,
    pub session_generation: u64,
    pub authentication: String,
    pub authentication_failed: bool,
}

/// Privacy-safe delete start result. Complete snapshot or challenge; no password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDeleteDto {
    pub outcome: String,
    pub snapshot: Option<DeviceSnapshotDto>,
    pub challenge: Option<DeviceDeleteChallengeDto>,
}

/// Static fail-closed device-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DeviceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for DeviceCommandError {}

fn device_failed(code: &str, description: &'static str) -> DeviceCommandError {
    DeviceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_device_core_error(no_session: &'static str, error: MatrixIpcError) -> DeviceCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => device_failed(code, DEVICE_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-crypto.7-") => {
            device_failed(code, DEVICE_OWNER_DESCRIPTION)
        }
        _ => device_failed(DEVICE_FAILED_CODE, DEVICE_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe join-rule snapshot. Closed vocabulary only; no allow-list or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomJoinRuleSnapshotDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub join_rule: String,
}

/// Static fail-closed join-rule error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinRuleCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for JoinRuleCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for JoinRuleCommandError {}

fn join_rule_failed(code: &str, description: &'static str) -> JoinRuleCommandError {
    JoinRuleCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_join_rule_core_error(error: MatrixIpcError) -> JoinRuleCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == JOIN_RULE_SNAPSHOT_NO_SESSION_CODE => {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-room-profile-join-rule-") => {
            join_rule_failed(code, JOIN_RULE_OWNER_DESCRIPTION)
        }
        _ => join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe image-pack row. Metadata/IDs/mxc URLs/JSON only; never image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePackDto {
    pub id: String,
    pub room_id: Option<String>,
    pub state_key: Option<String>,
    pub content_json: String,
}

/// Privacy-safe user pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserImagePackSnapshotDto {
    pub session_generation: u64,
    pub pack: Option<ImagePackDto>,
}

/// Privacy-safe room pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomImagePacksSnapshotDto {
    pub session_generation: u64,
    pub room_id: String,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe global pack snapshot. No tokens or image bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalImagePacksSnapshotDto {
    pub session_generation: u64,
    pub packs: Vec<ImagePackDto>,
}

/// Privacy-safe pack write ack. Status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePackWriteDto {
    pub status: String,
}

/// Static fail-closed image-pack-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImagePackCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ImagePackCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ImagePackCommandError {}

fn image_pack_failed(code: &str, description: &'static str) -> ImagePackCommandError {
    ImagePackCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_image_pack_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ImagePackCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            image_pack_failed(code, IMAGE_PACK_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-pack-") => {
            image_pack_failed(code, IMAGE_PACK_OWNER_DESCRIPTION)
        }
        _ => image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION),
    }
}

fn parse_image_pack_content_json(
    content_json: &str,
) -> Result<serde_json::Value, ImagePackCommandError> {
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(image_pack_failed(
            IMAGE_PACK_FAILED_CODE,
            IMAGE_PACK_FAILED_DESCRIPTION,
        ));
    }
    serde_json::from_str(content_json).map_err(|_| {
        image_pack_failed(
            IMAGE_PACK_INVALID_JSON_CODE,
            IMAGE_PACK_INVALID_JSON_DESCRIPTION,
        )
    })
}

fn image_pack_dto(pack: NativeImagePack) -> Result<ImagePackDto, ImagePackCommandError> {
    let content_json = serde_json::to_string(&pack.content)
        .map_err(|_| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackDto {
        id: pack.id,
        room_id: pack.room_id,
        state_key: pack.state_key,
        content_json,
    })
}

fn image_pack_write_dto(
    payload: serde_json::Value,
) -> Result<ImagePackWriteDto, ImagePackCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| image_pack_failed(IMAGE_PACK_FAILED_CODE, IMAGE_PACK_FAILED_DESCRIPTION))?;
    Ok(ImagePackWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe later item. Room/event ids and timestamps only; no tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct LaterItemDto {
    pub id: String,
    pub kind: String,
    pub room_id: String,
    pub event_id: String,
    pub created_at: f64,
    pub due_ts: Option<f64>,
    pub reminded_at: Option<f64>,
    pub completed_at: Option<f64>,
}

/// Privacy-safe later snapshot. No tokens or secret material.
#[derive(Debug, Clone, PartialEq)]
pub struct LaterSnapshotDto {
    pub session_generation: u64,
    pub version: u32,
    pub items: Vec<LaterItemDto>,
}

/// Static fail-closed later-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaterCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for LaterCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for LaterCommandError {}

fn later_failed(code: &str, description: &'static str) -> LaterCommandError {
    LaterCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_later_core_error(no_session: &'static str, error: MatrixIpcError) -> LaterCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => later_failed(code, LATER_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-timeline-later-") => {
            later_failed(code, LATER_OWNER_DESCRIPTION)
        }
        _ => later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION),
    }
}

fn later_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, LaterCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION));
    }
    Ok(payload)
}

fn later_item_from_dto(item: LaterItemDto) -> Result<SynaraLaterItem, LaterCommandError> {
    let kind = match item.kind.as_str() {
        "saved" => SynaraLaterItemKind::Saved,
        "reminder" => SynaraLaterItemKind::Reminder,
        _ => {
            return Err(later_failed(
                LATER_INVALID_ITEM_CODE,
                LATER_INVALID_ITEM_DESCRIPTION,
            ))
        }
    };
    if item.id.is_empty()
        || item.room_id.is_empty()
        || item.event_id.is_empty()
        || !item.created_at.is_finite()
    {
        return Err(later_failed(
            LATER_INVALID_ITEM_CODE,
            LATER_INVALID_ITEM_DESCRIPTION,
        ));
    }
    Ok(SynaraLaterItem {
        id: item.id,
        kind,
        room_id: item.room_id,
        event_id: item.event_id,
        created_at: item.created_at,
        due_ts: item.due_ts.filter(|value| value.is_finite()),
        reminded_at: item.reminded_at.filter(|value| value.is_finite()),
        completed_at: item.completed_at.filter(|value| value.is_finite()),
    })
}

fn later_item_dto(item: SynaraLaterItem) -> LaterItemDto {
    LaterItemDto {
        id: item.id,
        kind: match item.kind {
            SynaraLaterItemKind::Saved => "saved".to_owned(),
            SynaraLaterItemKind::Reminder => "reminder".to_owned(),
        },
        room_id: item.room_id,
        event_id: item.event_id,
        created_at: item.created_at,
        due_ts: item.due_ts,
        reminded_at: item.reminded_at,
        completed_at: item.completed_at,
    }
}

fn later_snapshot_dto(payload: serde_json::Value) -> Result<LaterSnapshotDto, LaterCommandError> {
    let snapshot: NativeLaterSnapshot = serde_json::from_value(payload)
        .map_err(|_| later_failed(LATER_FAILED_CODE, LATER_FAILED_DESCRIPTION))?;
    Ok(LaterSnapshotDto {
        session_generation: snapshot.session_generation,
        version: snapshot.content.version,
        items: snapshot
            .content
            .items
            .into_values()
            .map(later_item_dto)
            .collect(),
    })
}

/// Privacy-safe m.direct snapshot. User/room ids are the product map; no tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MDirectSnapshotDto {
    pub session_generation: u64,
    pub room_ids: Vec<String>,
    pub user_ids: Vec<String>,
}

/// Privacy-safe m.direct write ack. Status and the mutated room id only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MDirectMutationDto {
    pub room_id: String,
    pub status: String,
}

/// Static fail-closed m.direct-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MDirectCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MDirectCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MDirectCommandError {}

fn mdirect_failed(code: &str, description: &'static str) -> MDirectCommandError {
    MDirectCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_mdirect_core_error(no_session: &'static str, error: MatrixIpcError) -> MDirectCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => mdirect_failed(code, MDIRECT_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-rooms.5-mdirect-") => {
            mdirect_failed(code, MDIRECT_OWNER_DESCRIPTION)
        }
        _ => mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION),
    }
}

fn mdirect_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, MDirectCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(mdirect_failed(
            MDIRECT_FAILED_CODE,
            MDIRECT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn mdirect_snapshot_dto(
    payload: serde_json::Value,
) -> Result<MDirectSnapshotDto, MDirectCommandError> {
    let snapshot: NativeMDirectSnapshot = serde_json::from_value(payload)
        .map_err(|_| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    Ok(MDirectSnapshotDto {
        session_generation: snapshot.session_generation,
        room_ids: snapshot.room_ids,
        user_ids: snapshot.user_ids,
    })
}

fn mdirect_mutation_dto(
    payload: serde_json::Value,
) -> Result<MDirectMutationDto, MDirectCommandError> {
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| mdirect_failed(MDIRECT_FAILED_CODE, MDIRECT_FAILED_DESCRIPTION))?;
    if status != "updated" {
        return Err(mdirect_failed(
            MDIRECT_FAILED_CODE,
            MDIRECT_FAILED_DESCRIPTION,
        ));
    }
    Ok(MDirectMutationDto {
        room_id: room_id.to_owned(),
        status: status.to_owned(),
    })
}

/// Privacy-safe room-note item. Body/ids/timestamps may cross; no tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomNoteItemDto {
    pub id: String,
    pub kind: String,
    pub room_id: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub body: Option<String>,
    pub completed_at: Option<f64>,
    pub order: Option<f64>,
    pub event_id: Option<String>,
    pub event_ts: Option<f64>,
    pub sender: Option<String>,
}

/// Privacy-safe room-notes snapshot. Flattened items; no tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct RoomNotesSnapshotDto {
    pub session_generation: u64,
    pub version: u32,
    pub items: Vec<RoomNoteItemDto>,
}

/// Static fail-closed room-notes-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomNotesCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomNotesCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomNotesCommandError {}

fn room_notes_failed(code: &str, description: &'static str) -> RoomNotesCommandError {
    RoomNotesCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_notes_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomNotesCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_notes_failed(code, ROOM_NOTES_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-timeline-room-notes-") => {
            room_notes_failed(code, ROOM_NOTES_OWNER_DESCRIPTION)
        }
        _ => room_notes_failed(ROOM_NOTES_FAILED_CODE, ROOM_NOTES_FAILED_DESCRIPTION),
    }
}

fn room_notes_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomNotesCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_notes_failed(
            ROOM_NOTES_FAILED_CODE,
            ROOM_NOTES_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn room_note_item_from_dto(
    item: RoomNoteItemDto,
) -> Result<SynaraRoomNoteItem, RoomNotesCommandError> {
    let kind = match item.kind.as_str() {
        "note" => SynaraRoomNoteItemKind::Note,
        "todo" => SynaraRoomNoteItemKind::Todo,
        "message" => SynaraRoomNoteItemKind::Message,
        _ => {
            return Err(room_notes_failed(
                ROOM_NOTES_INVALID_ITEM_CODE,
                ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
            ))
        }
    };
    if item.id.is_empty()
        || item.room_id.is_empty()
        || !item.created_at.is_finite()
        || !item.updated_at.is_finite()
    {
        return Err(room_notes_failed(
            ROOM_NOTES_INVALID_ITEM_CODE,
            ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
        ));
    }
    Ok(SynaraRoomNoteItem {
        id: item.id,
        kind,
        room_id: item.room_id,
        created_at: item.created_at,
        updated_at: item.updated_at,
        body: item.body.filter(|value| !value.is_empty()),
        completed_at: item.completed_at.filter(|value| value.is_finite()),
        order: item.order.filter(|value| value.is_finite()),
        event_id: item.event_id.filter(|value| !value.is_empty()),
        event_ts: item.event_ts.filter(|value| value.is_finite()),
        sender: item.sender.filter(|value| !value.is_empty()),
    })
}

fn room_note_move_direction_from_dto(
    direction: &str,
) -> Result<RoomNoteMoveDirection, RoomNotesCommandError> {
    match direction {
        "up" => Ok(RoomNoteMoveDirection::Up),
        "down" => Ok(RoomNoteMoveDirection::Down),
        _ => Err(room_notes_failed(
            ROOM_NOTES_INVALID_ITEM_CODE,
            ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
        )),
    }
}

fn room_note_item_dto(item: SynaraRoomNoteItem) -> RoomNoteItemDto {
    RoomNoteItemDto {
        id: item.id,
        kind: match item.kind {
            SynaraRoomNoteItemKind::Note => "note".to_owned(),
            SynaraRoomNoteItemKind::Todo => "todo".to_owned(),
            SynaraRoomNoteItemKind::Message => "message".to_owned(),
        },
        room_id: item.room_id,
        created_at: item.created_at,
        updated_at: item.updated_at,
        body: item.body,
        completed_at: item.completed_at,
        order: item.order,
        event_id: item.event_id,
        event_ts: item.event_ts,
        sender: item.sender,
    }
}

fn room_notes_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomNotesSnapshotDto, RoomNotesCommandError> {
    let snapshot: NativeRoomNotesSnapshot = serde_json::from_value(payload)
        .map_err(|_| room_notes_failed(ROOM_NOTES_FAILED_CODE, ROOM_NOTES_FAILED_DESCRIPTION))?;
    Ok(RoomNotesSnapshotDto {
        session_generation: snapshot.session_generation,
        version: snapshot.content.version,
        items: snapshot
            .content
            .rooms
            .into_values()
            .flat_map(|room| room.items.into_values())
            .map(room_note_item_dto)
            .collect(),
    })
}

/// Privacy-safe own-profile write ack. Status only; no display name or mxc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnProfileWriteDto {
    pub status: String,
}

/// Static fail-closed own-profile-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnProfileCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for OwnProfileCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for OwnProfileCommandError {}

fn own_profile_failed(code: &str, description: &'static str) -> OwnProfileCommandError {
    OwnProfileCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_own_profile_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> OwnProfileCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            own_profile_failed(code, OWN_PROFILE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-avatar-") => {
            own_profile_failed(code, OWN_PROFILE_OWNER_DESCRIPTION)
        }
        _ => own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION),
    }
}

fn own_profile_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, OwnProfileCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(own_profile_failed(
            OWN_PROFILE_FAILED_CODE,
            OWN_PROFILE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn own_profile_write_dto(
    payload: serde_json::Value,
) -> Result<OwnProfileWriteDto, OwnProfileCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION)
        })?;
    Ok(OwnProfileWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room-profile write ack. Status only; no room id, name, topic, or mxc.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomProfileWriteDto {
    pub status: String,
}

/// Static fail-closed room-profile-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomProfileCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomProfileCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomProfileCommandError {}

fn room_profile_failed(code: &str, description: &'static str) -> RoomProfileCommandError {
    RoomProfileCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_profile_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomProfileCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_profile_failed(code, ROOM_PROFILE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-send.r-room-profile-")
                || code == "v-send.r-avatar-invalid-mxc"
                || code == "d0.4-send-invalid-room-id" =>
        {
            room_profile_failed(code, ROOM_PROFILE_OWNER_DESCRIPTION)
        }
        _ => room_profile_failed(ROOM_PROFILE_FAILED_CODE, ROOM_PROFILE_FAILED_DESCRIPTION),
    }
}

fn room_profile_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomProfileCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_profile_failed(
            ROOM_PROFILE_FAILED_CODE,
            ROOM_PROFILE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn room_profile_write_dto(
    payload: serde_json::Value,
) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_profile_failed(ROOM_PROFILE_FAILED_CODE, ROOM_PROFILE_FAILED_DESCRIPTION)
        })?;
    Ok(RoomProfileWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe directory-visibility read. Visibility is public/private only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryVisibilityDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub visibility: String,
}

/// Privacy-safe directory-visibility write ack. Visibility is public/private only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryVisibilityWriteDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub requested_visibility: String,
}

/// Static fail-closed directory-visibility-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectoryVisibilityCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DirectoryVisibilityCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for DirectoryVisibilityCommandError {}

fn directory_visibility_failed(
    code: &str,
    description: &'static str,
) -> DirectoryVisibilityCommandError {
    DirectoryVisibilityCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_directory_visibility_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> DirectoryVisibilityCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            directory_visibility_failed(code, DIRECTORY_VISIBILITY_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-room-profile-directory-visibility-") => {
            directory_visibility_failed(code, DIRECTORY_VISIBILITY_OWNER_DESCRIPTION)
        }
        _ => directory_visibility_failed(
            DIRECTORY_VISIBILITY_FAILED_CODE,
            DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
        ),
    }
}

fn directory_visibility_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, DirectoryVisibilityCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(directory_visibility_failed(
            DIRECTORY_VISIBILITY_FAILED_CODE,
            DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn closed_directory_visibility(value: &str) -> Option<&'static str> {
    match value {
        "public" => Some("public"),
        "private" => Some("private"),
        _ => None,
    }
}

fn room_directory_visibility_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryVisibilityDto, DirectoryVisibilityCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let visibility = payload
        .get("visibility")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_visibility)
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryVisibilityDto {
        status: status.to_owned(),
        room_id: room_id.to_owned(),
        session_generation,
        visibility: visibility.to_owned(),
    })
}

fn room_directory_visibility_write_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryVisibilityWriteDto, DirectoryVisibilityCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    let requested_visibility = payload
        .get("requestedVisibility")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_visibility)
        .ok_or_else(|| {
            directory_visibility_failed(
                DIRECTORY_VISIBILITY_FAILED_CODE,
                DIRECTORY_VISIBILITY_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryVisibilityWriteDto {
        status: status.to_owned(),
        room_id: room_id.to_owned(),
        session_generation,
        requested_visibility: requested_visibility.to_owned(),
    })
}

/// Privacy-safe third-party directory protocol instance. Ids and description only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryProtocolInstanceDto {
    pub protocol_id: String,
    pub instance_id: String,
    pub description: String,
}

/// Privacy-safe protocol list. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryProtocolsDto {
    pub session_generation: u64,
    pub instances: Vec<RoomDirectoryProtocolInstanceDto>,
}

/// Privacy-safe public-directory room hit. Metadata only; avatar_url is mxc, never bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryHitDto {
    pub room_id: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub canonical_alias: Option<String>,
    pub avatar_url: Option<String>,
    pub member_count: u32,
    pub world_readable: bool,
    pub guest_can_join: bool,
    pub room_type: String,
}

/// Privacy-safe search page. Room metadata only; no avatar bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectoryPageDto {
    pub session_generation: u64,
    pub request_id: u64,
    pub chunk: Vec<RoomDirectoryHitDto>,
    pub prev_batch: Option<String>,
    pub next_batch: Option<String>,
}

/// Privacy-safe search/cancel result. Status is ready/stale/cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomDirectorySearchDto {
    pub session_generation: u64,
    pub request_id: u64,
    pub status: String,
    pub page: Option<RoomDirectoryPageDto>,
}

/// Static fail-closed directory-search-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectorySearchCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for DirectorySearchCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for DirectorySearchCommandError {}

fn directory_search_failed(code: &str, description: &'static str) -> DirectorySearchCommandError {
    DirectorySearchCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_directory_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> DirectorySearchCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            directory_search_failed(code, DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms.directory-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            directory_search_failed(code, DIRECTORY_SEARCH_OWNER_DESCRIPTION)
        }
        _ => directory_search_failed(
            DIRECTORY_SEARCH_FAILED_CODE,
            DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ),
    }
}

fn directory_search_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, DirectorySearchCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(directory_search_failed(
            DIRECTORY_SEARCH_FAILED_CODE,
            DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn closed_directory_search_status(value: &str) -> Option<&'static str> {
    match value {
        "ready" => Some("ready"),
        "stale" => Some("stale"),
        "cancelled" => Some("cancelled"),
        _ => None,
    }
}

fn closed_directory_room_type(value: &str) -> Option<&'static str> {
    match value {
        "room" => Some("room"),
        "space" => Some("space"),
        _ => None,
    }
}

fn json_optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|value| value.as_str()).map(str::to_owned)
}

fn room_directory_protocols_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectoryProtocolsDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let instances = payload
        .get("instances")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut mapped = Vec::with_capacity(instances.len());
    for instance in instances {
        let protocol_id = instance
            .get("protocolId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        let instance_id = instance
            .get("instanceId")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        let description = instance
            .get("description")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                directory_search_failed(
                    DIRECTORY_SEARCH_FAILED_CODE,
                    DIRECTORY_SEARCH_FAILED_DESCRIPTION,
                )
            })?;
        mapped.push(RoomDirectoryProtocolInstanceDto {
            protocol_id: protocol_id.to_owned(),
            instance_id: instance_id.to_owned(),
            description: description.to_owned(),
        });
    }
    Ok(RoomDirectoryProtocolsDto {
        session_generation,
        instances: mapped,
    })
}

fn room_directory_hit_dto(
    payload: &serde_json::Value,
) -> Result<RoomDirectoryHitDto, DirectorySearchCommandError> {
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let member_count = payload
        .get("memberCount")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let world_readable = payload
        .get("worldReadable")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let guest_can_join = payload
        .get("guestCanJoin")
        .and_then(|value| value.as_bool())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let room_type = payload
        .get("roomType")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_room_type)
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomDirectoryHitDto {
        room_id: room_id.to_owned(),
        name: json_optional_string(payload.get("name")),
        topic: json_optional_string(payload.get("topic")),
        canonical_alias: json_optional_string(payload.get("canonicalAlias")),
        avatar_url: json_optional_string(payload.get("avatarUrl")),
        member_count,
        world_readable,
        guest_can_join,
        room_type: room_type.to_owned(),
    })
}

fn room_directory_page_dto(
    payload: &serde_json::Value,
) -> Result<RoomDirectoryPageDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let request_id = payload
        .get("requestId")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let chunk = payload
        .get("chunk")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut mapped = Vec::with_capacity(chunk.len());
    for hit in chunk {
        mapped.push(room_directory_hit_dto(hit)?);
    }
    Ok(RoomDirectoryPageDto {
        session_generation,
        request_id,
        chunk: mapped,
        prev_batch: json_optional_string(payload.get("prevBatch")),
        next_batch: json_optional_string(payload.get("nextBatch")),
    })
}

fn room_directory_search_dto(
    payload: serde_json::Value,
) -> Result<RoomDirectorySearchDto, DirectorySearchCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let request_id = payload
        .get("requestId")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_directory_search_status)
        .ok_or_else(|| {
            directory_search_failed(
                DIRECTORY_SEARCH_FAILED_CODE,
                DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let page = match payload.get("page") {
        None | Some(serde_json::Value::Null) => None,
        Some(page) => Some(room_directory_page_dto(page)?),
    };
    Ok(RoomDirectorySearchDto {
        session_generation,
        request_id,
        status: status.to_owned(),
        page,
    })
}

/// Privacy-safe room leave/join write ack. Status only; no room id, alias, or via servers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMembershipWriteDto {
    pub status: String,
}

/// Static fail-closed room-membership-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomMembershipCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomMembershipCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomMembershipCommandError {}

fn room_membership_failed(code: &str, description: &'static str) -> RoomMembershipCommandError {
    RoomMembershipCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_membership_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomMembershipCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_membership_failed(code, ROOM_MEMBERSHIP_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-room-leave-")
                || code.starts_with("v-rooms-room-join-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_membership_failed(code, ROOM_MEMBERSHIP_OWNER_DESCRIPTION)
        }
        _ => room_membership_failed(
            ROOM_MEMBERSHIP_FAILED_CODE,
            ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
        ),
    }
}

fn room_membership_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomMembershipCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_membership_failed(
            ROOM_MEMBERSHIP_FAILED_CODE,
            ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn closed_room_membership_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

fn room_membership_write_dto(
    payload: serde_json::Value,
) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
    if payload.is_null() {
        return Ok(RoomMembershipWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_membership_status)
        .ok_or_else(|| {
            room_membership_failed(
                ROOM_MEMBERSHIP_FAILED_CODE,
                ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomMembershipWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room invite/kick/ban/unban write ack. Status only; no room id, user id, or reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomModerationWriteDto {
    pub status: String,
}

/// Static fail-closed room-moderation-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomModerationCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomModerationCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomModerationCommandError {}

fn room_moderation_failed(code: &str, description: &'static str) -> RoomModerationCommandError {
    RoomModerationCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_moderation_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomModerationCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_moderation_failed(code, ROOM_MODERATION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-moderation-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_moderation_failed(code, ROOM_MODERATION_OWNER_DESCRIPTION)
        }
        _ => room_moderation_failed(
            ROOM_MODERATION_FAILED_CODE,
            ROOM_MODERATION_FAILED_DESCRIPTION,
        ),
    }
}

fn room_moderation_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomModerationCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_moderation_failed(
            ROOM_MODERATION_FAILED_CODE,
            ROOM_MODERATION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn closed_room_moderation_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

fn room_moderation_write_dto(
    payload: serde_json::Value,
) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
    if payload.is_null() {
        return Ok(RoomModerationWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_moderation_status)
        .ok_or_else(|| {
            room_moderation_failed(
                ROOM_MODERATION_FAILED_CODE,
                ROOM_MODERATION_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomModerationWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room power-level write ack. Status only; no room id, user id, power level, or content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomPowerLevelWriteDto {
    pub status: String,
}

/// Static fail-closed room-power-level-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomPowerLevelCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomPowerLevelCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomPowerLevelCommandError {}

fn room_power_level_failed(code: &str, description: &'static str) -> RoomPowerLevelCommandError {
    RoomPowerLevelCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_power_level_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomPowerLevelCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_power_level_failed(code, ROOM_POWER_LEVEL_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-moderation-")
                || code.starts_with("v-rooms-power-levels-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_power_level_failed(code, ROOM_POWER_LEVEL_OWNER_DESCRIPTION)
        }
        _ => room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ),
    }
}

fn parse_power_level_content_json(
    content_json: &str,
) -> Result<serde_json::Value, RoomPowerLevelCommandError> {
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ));
    }
    serde_json::from_str(content_json).map_err(|_| {
        room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        )
    })
}

fn room_power_level_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomPowerLevelCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn closed_room_power_level_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

fn room_power_level_write_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
    if payload.is_null() {
        return Ok(RoomPowerLevelWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_power_level_status)
        .ok_or_else(|| {
            room_power_level_failed(
                ROOM_POWER_LEVEL_FAILED_CODE,
                ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomPowerLevelWriteDto {
        status: status.to_owned(),
    })
}

/// Typed room-create request. Core scalar fields only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCreateRequestDto {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub room_alias_name: Option<String>,
    pub visibility: Option<String>,
    pub preset: Option<String>,
    pub is_direct: bool,
    pub encryption: bool,
    pub invite: Vec<String>,
    pub room_version: Option<String>,
    pub join_rule: Option<String>,
    pub knock: bool,
    pub parent_room_id: Option<String>,
}

/// Privacy-safe room-create result. Created room id only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCreateDto {
    pub room_id: String,
}

/// Static fail-closed room-create error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomCreateCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomCreateCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomCreateCommandError {}

fn room_create_failed(code: &str, description: &'static str) -> RoomCreateCommandError {
    RoomCreateCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_create_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomCreateCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_create_failed(code, ROOM_CREATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-room-create-")
                || code == "p2-room-create-invalid-payload"
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_create_failed(code, ROOM_CREATE_OWNER_DESCRIPTION)
        }
        _ => room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION),
    }
}

fn closed_room_create_visibility(value: &str) -> Option<&'static str> {
    match value {
        "private" => Some("private"),
        "public" => Some("public"),
        _ => None,
    }
}

fn closed_room_create_preset(value: &str) -> Option<&'static str> {
    match value {
        "private_chat" => Some("private_chat"),
        "public_chat" => Some("public_chat"),
        "trusted_private_chat" => Some("trusted_private_chat"),
        _ => None,
    }
}

fn closed_created_room_id(value: &str) -> Option<String> {
    if value.starts_with('!')
        && !value.is_empty()
        && value.len() <= 512
        && value.trim() == value
        && !value.chars().any(char::is_whitespace)
    {
        Some(value.to_owned())
    } else {
        None
    }
}

fn room_create_request_payload(
    request: RoomCreateRequestDto,
) -> Result<serde_json::Value, RoomCreateCommandError> {
    let visibility = request
        .visibility
        .as_deref()
        .map(|value| {
            closed_room_create_visibility(value).ok_or_else(|| {
                room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION)
            })
        })
        .transpose()?;
    let preset = request
        .preset
        .as_deref()
        .map(|value| {
            closed_room_create_preset(value).ok_or_else(|| {
                room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION)
            })
        })
        .transpose()?;
    Ok(serde_json::json!({
        "name": request.name,
        "topic": request.topic,
        "roomAliasName": request.room_alias_name,
        "visibility": visibility,
        "preset": preset,
        "isDirect": request.is_direct,
        "encryption": request.encryption,
        "invite": request.invite,
        "roomVersion": request.room_version,
        "joinRule": request.join_rule,
        "knock": request.knock,
        "parentRoomId": request.parent_room_id,
    }))
}

fn room_create_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomCreateCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_create_failed(
            ROOM_CREATE_FAILED_CODE,
            ROOM_CREATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn room_create_dto(payload: serde_json::Value) -> Result<RoomCreateDto, RoomCreateCommandError> {
    payload
        .as_str()
        .and_then(closed_created_room_id)
        .map(|room_id| RoomCreateDto { room_id })
        .ok_or_else(|| room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION))
}

/// Privacy-safe room member row. Ids, display name, mxc, membership, and power only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMemberDto {
    pub room_id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub membership: String,
    pub power_level: i32,
    pub is_direct_target: Option<bool>,
}

/// Privacy-safe members snapshot. Member rows only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomMembersSnapshotDto {
    pub session_generation: u64,
    pub room_id: String,
    pub members: Vec<RoomMemberDto>,
}

/// Privacy-safe power-levels snapshot. Content is JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomPowerLevelsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content_json: String,
}

/// Privacy-safe creators snapshot. Creator user ids only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCreatorsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub creators: Vec<String>,
}

/// Privacy-safe power-level-tags snapshot. Content is JSON text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomPowerLevelTagsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content_json: String,
}

/// Static fail-closed members-snapshot error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomMembersSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomMembersSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomMembersSnapshotError {}

fn room_members_snapshot_failed(code: &str, description: &'static str) -> RoomMembersSnapshotError {
    RoomMembersSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_members_snapshot_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomMembersSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_members_snapshot_failed(code, ROOM_MEMBERS_SNAPSHOT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-read-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_members_snapshot_failed(code, ROOM_MEMBERS_SNAPSHOT_OWNER_DESCRIPTION)
        }
        _ => room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

fn closed_room_member_membership(value: &str) -> Option<&'static str> {
    match value {
        "invite" => Some("invite"),
        "join" => Some("join"),
        "knock" => Some("knock"),
        "leave" => Some("leave"),
        "ban" => Some("ban"),
        _ => None,
    }
}

fn closed_members_snapshot_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

fn closed_power_levels_event_type(value: &str) -> Option<&'static str> {
    match value {
        "m.room.power_levels" => Some("m.room.power_levels"),
        _ => None,
    }
}

fn closed_creators_event_type(value: &str) -> Option<&'static str> {
    match value {
        "m.room.create" => Some("m.room.create"),
        _ => None,
    }
}

fn closed_power_level_tags_event_type(value: &str) -> Option<&'static str> {
    match value {
        "in.synara.room.power_level_tags" => Some("in.synara.room.power_level_tags"),
        _ => None,
    }
}

fn room_members_snapshot_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomMembersSnapshotError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn snapshot_content_json(content: &serde_json::Value) -> Result<String, RoomMembersSnapshotError> {
    let content_json = serde_json::to_string(content).map_err(|_| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ));
    }
    Ok(content_json)
}

fn room_member_dto(value: &serde_json::Value) -> Result<RoomMemberDto, RoomMembersSnapshotError> {
    let membership = value
        .get("membership")
        .and_then(|item| item.as_str())
        .and_then(closed_room_member_membership)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let power_level = value
        .get("powerLevel")
        .and_then(|item| item.as_i64())
        .and_then(|item| i32::try_from(item).ok())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = value
        .get("roomId")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let user_id = value
        .get("userId")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomMemberDto {
        room_id: room_id.to_owned(),
        user_id: user_id.to_owned(),
        display_name: value
            .get("displayName")
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned),
        avatar_url: value
            .get("avatarUrl")
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned),
        membership: membership.to_owned(),
        power_level,
        is_direct_target: value.get("isDirectTarget").and_then(|item| item.as_bool()),
    })
}

fn room_members_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomMembersSnapshotDto, RoomMembersSnapshotError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let members = payload
        .get("members")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?
        .iter()
        .map(room_member_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RoomMembersSnapshotDto {
        session_generation,
        room_id: room_id.to_owned(),
        members,
    })
}

fn room_power_levels_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_power_levels_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let content = payload.get("content").ok_or_else(|| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    Ok(RoomPowerLevelsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        content_json: snapshot_content_json(content)?,
    })
}

fn room_creators_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomCreatorsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_creators_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let creators = payload
        .get("creators")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    room_members_snapshot_failed(
                        ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                        ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RoomCreatorsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        creators,
    })
}

fn room_power_level_tags_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelTagsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_power_level_tags_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let content = payload.get("content").ok_or_else(|| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    Ok(RoomPowerLevelTagsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        content_json: snapshot_content_json(content)?,
    })
}

/// Privacy-safe space parent row. Child room id plus parent room ids only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceParentEntryDto {
    pub room_id: String,
    pub parent_ids: Vec<String>,
}

/// Privacy-safe space parents snapshot. Entries only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceParentsSnapshotDto {
    pub session_generation: u64,
    pub entries: Vec<SpaceParentEntryDto>,
}

/// Privacy-safe hierarchy room. Metadata only; avatar is an mxc reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceHierarchyRoomDto {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub topic: Option<String>,
    pub avatar_url: Option<String>,
    pub room_type: Option<String>,
    pub num_joined_members: u64,
    pub join_rule: String,
    pub world_readable: bool,
    pub guest_can_join: bool,
}

/// Privacy-safe space hierarchy snapshot. Room metadata only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceHierarchySnapshotDto {
    pub session_generation: u64,
    pub rooms: Vec<SpaceHierarchyRoomDto>,
}

/// Privacy-safe local space-child edge. Room ids, order, suggested, via only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceChildEdgeDto {
    pub parent_id: String,
    pub child_id: String,
    pub order: Option<String>,
    pub suggested: bool,
    pub via: Vec<String>,
    pub origin_server_ts: u64,
}

/// Privacy-safe space children snapshot. Edges only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceChildrenSnapshotDto {
    pub session_generation: u64,
    pub edges: Vec<SpaceChildEdgeDto>,
}

/// Privacy-safe space-child write ack. Room ids and status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceChildMutationDto {
    pub parent_id: String,
    pub child_id: String,
    pub status: String,
}

/// Privacy-safe restricted-join reparent ack. Room id and status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestrictedJoinReparentDto {
    pub room_id: String,
    pub status: String,
}

/// Static fail-closed space error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpaceCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SpaceCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SpaceCommandError {}

fn space_failed(code: &str, description: &'static str) -> SpaceCommandError {
    SpaceCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_space_core_error(no_session: &'static str, error: MatrixIpcError) -> SpaceCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => space_failed(code, SPACE_NO_SESSION_DESCRIPTION),
        Some(code)
            if code.starts_with("v-rooms.2a-")
                || code.starts_with("v-rooms.2b-")
                || code.starts_with("v-rooms.2c-")
                || code == "v-send.r-room-profile-join-rule-requires-session"
                || code.starts_with("p2-space-")
                || code.starts_with("p2-restricted-join-reparent-") =>
        {
            space_failed(code, SPACE_OWNER_DESCRIPTION)
        }
        _ => space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION),
    }
}

/// Static fail-closed invite-action error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InviteActionError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InviteActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for InviteActionError {}

fn invite_action_failed(code: &str, description: &'static str) -> InviteActionError {
    InviteActionError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_invite_action_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> InviteActionError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            invite_action_failed(code, INVITE_ACTION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms.1-invite")
                || code.starts_with("p2-invites-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            invite_action_failed(code, INVITE_ACTION_OWNER_DESCRIPTION)
        }
        _ => invite_action_failed(INVITE_ACTION_FAILED_CODE, INVITE_ACTION_FAILED_DESCRIPTION),
    }
}

fn invite_action_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, InviteActionError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(invite_action_failed(
            INVITE_ACTION_FAILED_CODE,
            INVITE_ACTION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn invite_action_snapshot_dto(
    payload: serde_json::Value,
) -> Result<InviteSnapshotDto, InviteActionError> {
    let snapshot: NativeInviteSnapshot = serde_json::from_value(payload).map_err(|_| {
        invite_action_failed(INVITE_ACTION_FAILED_CODE, INVITE_ACTION_FAILED_DESCRIPTION)
    })?;
    Ok(InviteSnapshotDto {
        session_generation: snapshot.session_generation,
        invites: snapshot.invites.into_iter().map(invite_dto).collect(),
    })
}

fn space_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SpaceCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION));
    }
    Ok(payload)
}

fn closed_space_join_rule(value: &str) -> Option<&'static str> {
    match value {
        "public" => Some("public"),
        "invite" => Some("invite"),
        "knock" => Some("knock"),
        "private" => Some("private"),
        "restricted" => Some("restricted"),
        "knock_restricted" => Some("knock_restricted"),
        _ => None,
    }
}

fn closed_space_child_status(value: &str) -> Option<&'static str> {
    match value {
        "updated" => Some("updated"),
        "removed" => Some("removed"),
        "skipped" => Some("skipped"),
        _ => None,
    }
}

fn closed_restricted_join_reparent_status(value: &str) -> Option<&'static str> {
    match value {
        "updated" => Some("updated"),
        "skipped" => Some("skipped"),
        _ => None,
    }
}

fn required_space_id(value: Option<&serde_json::Value>) -> Result<String, SpaceCommandError> {
    value
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))
}

fn optional_space_string(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|item| item.as_str()).map(ToOwned::to_owned)
}

fn space_string_list(value: Option<&serde_json::Value>) -> Result<Vec<String>, SpaceCommandError> {
    value
        .and_then(|item| item.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(|item| required_space_id(Some(item)))
        .collect()
}

fn space_parent_entry_dto(
    value: &serde_json::Value,
) -> Result<SpaceParentEntryDto, SpaceCommandError> {
    Ok(SpaceParentEntryDto {
        room_id: required_space_id(value.get("roomId"))?,
        parent_ids: space_string_list(value.get("parentIds"))?,
    })
}

fn space_parents_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceParentsSnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let entries = payload
        .get("entries")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_parent_entry_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceParentsSnapshotDto {
        session_generation,
        entries,
    })
}

fn space_hierarchy_room_dto(
    value: &serde_json::Value,
) -> Result<SpaceHierarchyRoomDto, SpaceCommandError> {
    let join_rule = value
        .get("joinRule")
        .and_then(|item| item.as_str())
        .and_then(closed_space_join_rule)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let num_joined_members = value
        .get("numJoinedMembers")
        .and_then(|item| item.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let world_readable = value
        .get("worldReadable")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let guest_can_join = value
        .get("guestCanJoin")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceHierarchyRoomDto {
        room_id: required_space_id(value.get("roomId"))?,
        name: optional_space_string(value.get("name")),
        canonical_alias: optional_space_string(value.get("canonicalAlias")),
        topic: optional_space_string(value.get("topic")),
        avatar_url: optional_space_string(value.get("avatarUrl")),
        room_type: optional_space_string(value.get("roomType")),
        num_joined_members,
        join_rule: join_rule.to_owned(),
        world_readable,
        guest_can_join,
    })
}

fn space_hierarchy_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceHierarchySnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let rooms = payload
        .get("rooms")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_hierarchy_room_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceHierarchySnapshotDto {
        session_generation,
        rooms,
    })
}

fn space_child_edge_dto(value: &serde_json::Value) -> Result<SpaceChildEdgeDto, SpaceCommandError> {
    let suggested = value
        .get("suggested")
        .and_then(|item| item.as_bool())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let origin_server_ts = value
        .get("originServerTs")
        .and_then(|item| item.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceChildEdgeDto {
        parent_id: required_space_id(value.get("parentId"))?,
        child_id: required_space_id(value.get("childId"))?,
        order: optional_space_string(value.get("order")),
        suggested,
        via: space_string_list(value.get("via"))?,
        origin_server_ts,
    })
}

fn space_children_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SpaceChildrenSnapshotDto, SpaceCommandError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    let edges = payload
        .get("edges")
        .and_then(|value| value.as_array())
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?
        .iter()
        .map(space_child_edge_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SpaceChildrenSnapshotDto {
        session_generation,
        edges,
    })
}

fn space_child_mutation_dto(
    payload: serde_json::Value,
) -> Result<SpaceChildMutationDto, SpaceCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_space_child_status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(SpaceChildMutationDto {
        parent_id: required_space_id(payload.get("parentId"))?,
        child_id: required_space_id(payload.get("childId"))?,
        status: status.to_owned(),
    })
}

fn restricted_join_reparent_dto(
    payload: serde_json::Value,
) -> Result<RestrictedJoinReparentDto, SpaceCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_restricted_join_reparent_status)
        .ok_or_else(|| space_failed(SPACE_FAILED_CODE, SPACE_FAILED_DESCRIPTION))?;
    Ok(RestrictedJoinReparentDto {
        room_id: required_space_id(payload.get("roomId"))?,
        status: status.to_owned(),
    })
}

fn device_trust_as_str(trust: NativeDeviceTrust) -> String {
    match trust {
        NativeDeviceTrust::Verified => "verified",
        NativeDeviceTrust::Unverified => "unverified",
        NativeDeviceTrust::Unsupported => "unsupported",
    }
    .to_owned()
}

fn device_snapshot_dto(snapshot: NativeDeviceSnapshot) -> DeviceSnapshotDto {
    DeviceSnapshotDto {
        session_generation: snapshot.session_generation,
        devices: snapshot
            .devices
            .into_iter()
            .map(|device| DeviceSummaryDto {
                device_id: device.device_id,
                display_name: device.display_name,
                last_seen_ip: device.last_seen_ip,
                last_seen_ts: device.last_seen_ts,
                trust: device_trust_as_str(device.trust),
                is_current: device.is_current,
            })
            .collect(),
    }
}

fn device_delete_dto(result: NativeDeviceDeleteResult) -> DeviceDeleteDto {
    match result {
        NativeDeviceDeleteResult::Complete { snapshot } => DeviceDeleteDto {
            outcome: "complete".to_owned(),
            snapshot: Some(device_snapshot_dto(snapshot)),
            challenge: None,
        },
        NativeDeviceDeleteResult::AuthenticationRequired { challenge } => DeviceDeleteDto {
            outcome: "authentication_required".to_owned(),
            snapshot: None,
            challenge: Some(DeviceDeleteChallengeDto {
                operation_id: challenge.operation_id,
                session_generation: challenge.session_generation,
                authentication: match challenge.authentication {
                    NativeDeviceDeleteAuthentication::Password => "password".to_owned(),
                },
                authentication_failed: challenge.authentication_failed,
            }),
        },
    }
}

/// Claims the restore slot for one in-flight attempt. Drop releases it unless
/// [`RestoreClaim::commit`] stores the Client after a successful Core open.
struct RestoreClaim<'a> {
    slot: &'a Mutex<RestoredClientSlot>,
    committed: bool,
}

impl<'a> RestoreClaim<'a> {
    fn acquire(slot: &'a Mutex<RestoredClientSlot>) -> Result<Self, SessionRestoreError> {
        let mut guard = slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        match *guard {
            RestoredClientSlot::Empty => {
                *guard = RestoredClientSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            RestoredClientSlot::InFlight | RestoredClientSlot::Ready(_) => Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            )),
        }
    }

    fn commit(mut self, client: Client) -> Result<(), SessionRestoreError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if !matches!(*guard, RestoredClientSlot::InFlight) {
            return Err(restore_failed(
                RESTORE_FAILED_CODE,
                RESTORE_FAILED_DESCRIPTION,
            ));
        }
        *guard = RestoredClientSlot::Ready(client);
        self.committed = true;
        Ok(())
    }
}

impl Drop for RestoreClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, RestoredClientSlot::InFlight) {
                *guard = RestoredClientSlot::Empty;
            }
        }
    }
}

/// Claims the owner-attach slot for one in-flight attempt.
struct AttachClaim<'a> {
    slot: &'a Mutex<OwnerAttachSlot>,
    committed: bool,
}

impl<'a> AttachClaim<'a> {
    fn acquire(slot: &'a Mutex<OwnerAttachSlot>) -> Result<Self, SessionAttachError> {
        let mut guard = slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        match *guard {
            OwnerAttachSlot::Empty => {
                *guard = OwnerAttachSlot::InFlight;
                Ok(Self {
                    slot,
                    committed: false,
                })
            }
            OwnerAttachSlot::Ready => Err(attach_failed(
                ATTACH_ALREADY_CODE,
                ATTACH_ALREADY_DESCRIPTION,
            )),
            OwnerAttachSlot::InFlight => {
                Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))
            }
        }
    }

    fn commit(mut self) -> Result<(), SessionAttachError> {
        let mut guard = self
            .slot
            .lock()
            .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?;
        if !matches!(*guard, OwnerAttachSlot::InFlight) {
            return Err(attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION));
        }
        *guard = OwnerAttachSlot::Ready;
        self.committed = true;
        Ok(())
    }
}

impl Drop for AttachClaim<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut guard) = self.slot.lock() {
            if matches!(*guard, OwnerAttachSlot::InFlight) {
                *guard = OwnerAttachSlot::Empty;
            }
        }
    }
}

fn parse_store_root(store_root: &str) -> Result<&Path, ()> {
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(());
    }
    Ok(path)
}

fn validate_store_root(store_root: &str) -> Result<&Path, SessionRestoreError> {
    parse_store_root(store_root)
        .map_err(|_| restore_failed(STORE_ROOT_INVALID_CODE, STORE_ROOT_INVALID_DESCRIPTION))
}

fn store_key_for(
    store: &Arc<dyn SecretVault + Send + Sync>,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    get_or_create_store_key(&vault, &StoreKeyId::from_identity(identity)).map_err(|error| {
        match error {
            StoreKeyVaultError::BackendUnavailable { .. } => {
                restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
            }
            StoreKeyVaultError::CorruptPayload => {
                restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION)
            }
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        }
    })
}

struct CallbackSecretVault {
    inner: Box<dyn IosSecretVault>,
}

impl SecretVault for CallbackSecretVault {
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MatrixIpcError> {
        self.inner
            .get(key.to_owned())
            .map_err(|_| vault_unavailable())
    }

    fn put(&self, key: &str, value: &[u8]) -> Result<(), MatrixIpcError> {
        self.inner
            .put(key.to_owned(), value.to_vec())
            .map_err(|_| vault_unavailable())
    }

    fn delete(&self, key: &str) -> Result<(), MatrixIpcError> {
        self.inner
            .delete(key.to_owned())
            .map_err(|_| vault_unavailable())
    }
}

struct SecretStoreSessionVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

impl SessionMaterialVault for SecretStoreSessionVault {
    fn get(
        &self,
        id: &SessionMaterialId,
    ) -> Result<Option<SessionMaterial>, crate::app::lifecycle::LifecycleError> {
        match self.store.get(id.account()) {
            Ok(Some(bytes)) => Ok(Some(SessionMaterial::from_sealed_blob(bytes))),
            Ok(None) => Ok(None),
            Err(_) => Err(crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }),
        }
    }

    fn set(
        &self,
        id: &SessionMaterialId,
        material: &SessionMaterial,
    ) -> Result<(), crate::app::lifecycle::LifecycleError> {
        self.store
            .put(id.account(), material.as_bytes())
            .map_err(|_| crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            })
    }

    fn clear(&self, id: &SessionMaterialId) -> Result<bool, crate::app::lifecycle::LifecycleError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store.delete(id.account()).map_err(|_| {
            crate::app::lifecycle::LifecycleError::Vault {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
                category: MatrixIpcErrorCategory::StoreUnavailable,
            }
        })?;
        Ok(existed)
    }
}

struct SecretStoreKeyVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

impl StoreKeyVault for SecretStoreKeyVault {
    fn get(&self, id: &StoreKeyId) -> Result<Option<StoreKeyMaterial>, StoreKeyVaultError> {
        match self.store.get(id.account()) {
            Ok(None) => Ok(None),
            Ok(Some(bytes)) if bytes.len() == STORE_KEY_LEN => {
                let mut key_bytes = [0u8; STORE_KEY_LEN];
                key_bytes.copy_from_slice(&bytes);
                Ok(Some(StoreKeyMaterial::from_bytes(key_bytes)))
            }
            Ok(Some(_)) => Err(StoreKeyVaultError::CorruptPayload),
            Err(_) => Err(StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            }),
        }
    }

    fn set(&self, id: &StoreKeyId, key: &StoreKeyMaterial) -> Result<(), StoreKeyVaultError> {
        self.store
            .put(id.account(), key.as_bytes().as_slice())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })
    }

    fn delete(&self, id: &StoreKeyId) -> Result<bool, StoreKeyVaultError> {
        let existed = self.store.get(id.account()).ok().flatten().is_some();
        self.store
            .delete(id.account())
            .map_err(|_| StoreKeyVaultError::BackendUnavailable {
                diagnostic_id: "p4-s3b-secret-vault-unavailable",
            })?;
        Ok(existed)
    }
}

fn vault_unavailable() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::StoreUnavailable)
        .with_diagnostic("p4-s3-secret-vault-unavailable")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::lifecycle::persist_session_material;
    use crate::app::store::StoreKeyId;
    use crate::transport::MatrixIpcErrorCategory;
    use std::collections::HashMap;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct MemoryCallbackVault(std::sync::Arc<Mutex<HashMap<String, Vec<u8>>>>);

    impl IosSecretVault for MemoryCallbackVault {
        fn get(&self, key: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
            Ok(self.0.lock().expect("vault").get(&key).cloned())
        }

        fn put(&self, key: String, value: Vec<u8>) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").insert(key, value);
            Ok(())
        }

        fn delete(&self, key: String) -> Result<(), IosSecretVaultError> {
            self.0.lock().expect("vault").remove(&key);
            Ok(())
        }
    }

    fn alice() -> AccountIdentity {
        AccountIdentity::new("@alice:example.org", "https://matrix.example.org").unwrap()
    }

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-p4-s3b-{tag}-{nanos}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn test_runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn shared_core_constructs_and_retains_the_built_in_core() {
        let shared_core = SharedCore::new();
        assert!(
            !shared_core.core.registered_commands().is_empty(),
            "P4-S2 must retain a real Core with its built-in registry"
        );
    }

    #[test]
    fn shared_core_with_secret_store_round_trips_through_the_callback() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        assert!(
            !shared.core.registered_commands().is_empty(),
            "P4-S3a must still retain a real Core"
        );
    }

    #[test]
    fn callback_vault_maps_foreign_failure_to_static_store_unavailable() {
        struct FailingVault;
        impl IosSecretVault for FailingVault {
            fn get(&self, _: String) -> Result<Option<Vec<u8>>, IosSecretVaultError> {
                Err(IosSecretVaultError::Unavailable {
                    code: "p4-s3-secret-vault-unavailable".to_owned(),
                    description: "The secret store is unavailable.".to_owned(),
                })
            }
            fn put(&self, _: String, _: Vec<u8>) -> Result<(), IosSecretVaultError> {
                unreachable!("put")
            }
            fn delete(&self, _: String) -> Result<(), IosSecretVaultError> {
                unreachable!("delete")
            }
        }

        let vault = CallbackSecretVault {
            inner: Box::new(FailingVault),
        };
        let error = vault.get("session").expect_err("must fail closed");
        assert_eq!(error.category, MatrixIpcErrorCategory::StoreUnavailable);
        assert!(!format!("{error:?}").contains("session"));
    }

    #[test]
    fn restore_without_vault_fails_closed_without_echoing_identity() {
        let shared = SharedCore::new();
        let root = temp_root("no-vault");
        let rt = test_runtime();
        let error = rt
            .block_on(shared.restore_persisted_session(
                "@alice:example.org".to_owned(),
                "https://matrix.example.org".to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("fail-closed vault cannot restore");
        let text = format!("{error:?}");
        assert!(text.contains(VAULT_UNAVAILABLE_CODE));
        assert!(!text.contains(MATERIAL_MISSING_CODE));
        assert!(!text.contains("@alice"));
        assert!(!text.contains("matrix.example.org"));
        assert!(!text.contains(root.to_string_lossy().as_ref()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_rejects_hostile_identity_without_echo() {
        let store = Box::new(MemoryCallbackVault(std::sync::Arc::new(Mutex::new(
            HashMap::new(),
        ))));
        let shared = SharedCore::new_with_secret_store(store);
        let root = temp_root("hostile");
        let rt = test_runtime();
        let hostile = "https://user:secret@evil.example/?password=hunter2";
        let error = rt
            .block_on(shared.restore_persisted_session(
                "not-a-user".to_owned(),
                hostile.to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("invalid identity");
        let text = format!("{error:?}{error}");
        assert!(text.contains(IDENTITY_INVALID_CODE));
        assert!(!text.contains("secret"));
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("evil.example"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_from_vault_installs_session_without_password_or_token_leak() {
        let access = "syt_s3b_access_token_value";
        let refresh = "syr_s3b_refresh_token_value";
        let identity = alice();
        let material =
            SessionMaterial::from_matrix_tokens(&identity, "DEVICEABC", access, Some(refresh))
                .unwrap();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let persist_vault = SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
            }),
        };
        persist_session_material(&persist_vault, &identity, &material).unwrap();
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&map),
        )));
        let root = temp_root("restore");
        let rt = test_runtime();
        let _enter = rt.enter();
        let dto = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect("restore");
        assert_eq!(dto.user_id, "@alice:example.org");
        assert_eq!(dto.device_id, "DEVICEABC");
        assert_eq!(dto.homeserver_url, "https://matrix.example.org");
        let dbg = format!("{dto:?}");
        assert!(!dbg.contains(access));
        assert!(!dbg.contains(refresh));
        assert!(!dbg.contains("password"));
        let snapshot = shared.core.session_snapshot().expect("projection");
        assert!(snapshot.is_some());
        assert!(matches!(
            *shared.restored_client.lock().expect("client"),
            RestoredClientSlot::Ready(_)
        ));
        let keys: Vec<String> = map.lock().expect("vault").keys().cloned().collect();
        assert!(keys.iter().any(|key| key.starts_with("store-key:")));
        assert!(keys.iter().any(|key| key.starts_with("matrix-session:")));
        assert!(!keys.iter().any(|key| key.contains("p4-s3b-store-key")));
        let second = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("second restore");
        assert!(format!("{second:?}").contains(RESTORE_FAILED_CODE));
        assert!(matches!(
            *shared.restored_client.lock().expect("client"),
            RestoredClientSlot::Ready(_)
        ));
        drop(shared);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn restore_rejects_wrong_length_store_key_without_replacing_it() {
        let identity = alice();
        let material = SessionMaterial::from_matrix_tokens(
            &identity,
            "DEVICEABC",
            "syt_s3b_corrupt_key_access",
            None,
        )
        .unwrap();
        let map = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let persist_vault = SecretStoreSessionVault {
            store: Arc::new(CallbackSecretVault {
                inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&map))),
            }),
        };
        persist_session_material(&persist_vault, &identity, &material).unwrap();
        let store_key_account = StoreKeyId::from_identity(&identity).account().to_owned();
        map.lock()
            .expect("vault")
            .insert(store_key_account.clone(), vec![0u8; 8]);
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&map),
        )));
        let root = temp_root("corrupt-key");
        let rt = test_runtime();
        let error = rt
            .block_on(shared.restore_persisted_session(
                identity.user_id().to_owned(),
                identity.homeserver_url().to_owned(),
                root.to_string_lossy().into_owned(),
            ))
            .expect_err("corrupt store key");
        assert!(format!("{error:?}").contains(RESTORE_FAILED_CODE));
        let stored = map
            .lock()
            .expect("vault")
            .get(&store_key_account)
            .cloned()
            .expect("key remains");
        assert_eq!(stored.len(), 8);
        assert!(!map
            .lock()
            .expect("vault")
            .keys()
            .any(|key| key.contains("p4-s3b-store-key")));
        let _ = fs::remove_dir_all(&root);
    }
}
