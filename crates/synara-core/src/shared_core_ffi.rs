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
//! P4-S12 adds `start_sync` for that already-attached SyncService. NSE
//! still cannot start sync. This is not iOS-on-engine.
//! P4-S4 adds a typed `room_list_snapshot` wrapper that calls the
//! already-registered `matrix_room_list_snapshot` Core command only.
//! P4-S5 adds a typed `invites_snapshot` wrapper that calls the
//! already-registered `matrix_invites_snapshot` Core command only.
//! P4-S6 adds typed `timeline_open` / `timeline_close` / `timeline_paginate`
//! wrappers for those three already-registered Core commands only.
//! P4-S7 adds typed typing/presence wrappers for the already-registered
//! Core commands in that family, including presence SET.
//! P4-S8 adds a typed `verification_list` wrapper for the already-registered
//! `matrix_verification_list` Core command only.
//! P4-S9 adds typed verification SAS wrappers for the seven already-registered
//! start/accept/begin_sas/confirm/mismatch/cancel/dismiss Core commands only.
//! P4-S9-2 adds typed device wrappers for the four already-registered
//! snapshot/rename/delete-start/delete-cancel Core commands only.
//! Backup status, room-key transfer status, and cross-signing setup stay off
//! this slice: they sit next to leftover passphrase/path/password envelopes.
//! P4-S9-3 adds a typed `room_join_rule_snapshot` wrapper for the
//! already-registered `matrix_room_join_rule_snapshot` Core command.
//! Join-rule write (`room_set_join_rule` / `matrix_room_set_join_rule`)
//! hangs on the same NativeRoomJoinRuleOwner. Write ack is status only.
//! Failed errors never echo room id, join rule, or allow-list ids.
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
//! P4-S9-8a adds typed `get_own_profile` wrapping the already-registered
//! `matrix_get_own_profile` Core command. Empty payload. Avatar is an
//! `mxc://` URI only. Failed errors never echo display name, mxc, or tokens.
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
//! P4-S9-19 adds typed `timeline_event_readback` / `timeline_set_read_state`
//! / `timeline_jump_latest` wrappers for those three already-registered
//! Core commands. Jump returns the existing open readback. Failed errors
//! never echo event id, room id, or stream id. Timeline reactions stay off.
//! P4-S9-20 adds typed `reaction_ensure` / `reaction_redact` /
//! `timeline_reaction_toggle` wrappers for those three already-registered
//! Core commands. Failed errors never echo room id, event id, reaction
//! event id, or key. Composer reply draft stays off.
//! P4-S9-21 adds typed `composer_set_reply_draft` / `composer_get_reply_draft` /
//! `composer_clear_reply_draft` wrappers for those three already-registered
//! Core commands. Failed errors never echo room id or event id. Send text
//! stays off.
//! P4-S9-22 adds a typed `send_text` wrapper for the already-registered
//! `matrix_send_text` Core command only. No media bytes. Failed errors never
//! echo body or room id. Sticker, poll, edit, and respond stay off.
//! Live `upload_content` / `send_room_attachment` take bytes as method
//! arguments. Leftover `media_upload` remains.
//! P4-S9-24 adds a typed `send_poll` wrapper for the already-registered
//! `matrix_send_poll` Core command only. No media bytes. Failed errors never
//! echo question, options, or room id. Edit and respond stay off.
//! P4-S9-25 adds a typed `edit_message` wrapper for the already-registered
//! `matrix_edit_message` Core command only. No media bytes. Failed errors never
//! echo body, event id, or room id. Poll respond stays off.
//! P4-S9-26 adds a typed `poll_respond` wrapper for the already-registered
//! `matrix_poll_respond` Core command only. No media bytes. Failed errors never
//! echo answers, event id, or room id. Timeline edit/redact/report stay off.
//! P4-S9-27 adds typed `timeline_edit_text` / `timeline_redact` /
//! `timeline_report` wrappers for the already-registered Core commands.
//! Failed errors never echo body, event id, room id, or reason. Pin/unpin
//! stay off.
//! P4-S9-28 adds typed `timeline_pin` / `timeline_unpin` wrappers for the
//! already-registered Core commands. Failed errors never echo event id or
//! room id. Poll vote / call decline stay off.
//! P4-S9-29 adds typed `timeline_poll_vote` / `timeline_call_decline`
//! wrappers for the already-registered Core commands. Failed errors never
//! echo event id, room id, or answer. Timeline forward stays off.
//! P4-S9-30 adds typed `timeline_forward_text` / `timeline_forward_media`
//! wrappers for the already-registered Core commands. Failed errors never
//! echo event id, source room id, or target room id. Session/status
//! reads stay off. No media bytes cross the envelope.
//! P4-S9-31 adds typed `session_snapshot` / `sync_status` / `media_config` /
//! `secret_storage_status` wrappers for the already-registered Core
//! commands. Failed errors never echo user id, homeserver, or device id.
//! Backup/crypto/cross-signing/room-key status stay off.
//! P4-S11 adds a read-only NSE store API (`nse_open_read_only_store` /
//! `nse_store_status` / `nse_event_preview`). It never starts SyncService,
//! never attaches owners, and never boots leftover Client sync. Failed
//! errors never echo room id, event id, user id, or tokens.
//! P4-S10 leftover retirement adds typed leftover status wrappers plus
//! dedicated leftover FFI for wipe/logout/recover/raw-send/notification/
//! media/avatar/pusher. Secrets and bytes are dedicated arguments only.
//! Failed errors stay static and never echo password, recovery key, event,
//! body, bytes, URL, or token. Oversize fail-closes at 1 MiB with no
//! truncate. Planted leftover I/O does not hit a live homeserver.
//! Live HTTP pusher set/delete are dedicated Core methods (push keys stay
//! off `Core::command` JSON). Leftover `pusher_set` / `pusher_delete` remain.
//! Live `restore_backup` takes a recovery secret as a method argument and
//! calls SDK `recover()`. Leftover `recover` remains fail-closed.
//! Live `download_plain_media` / `thumbnail_plain_media` take an `mxc://`
//! argument and return bytes. Leftover `media_download` / `media_thumbnail`
//! remain fail-closed. Timeline-media handles stay on `timeline_media_bytes`.
//! This still exposes no generic command FFI or APNs surface.

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use matrix_sdk::ruma::events::{
    room::message::MessageType, AnySyncMessageLikeEvent, AnySyncTimelineEvent,
};
use matrix_sdk::store::RoomLoadSettings;
use matrix_sdk::Client;
use matrix_sdk_ui::notification_client::{
    NotificationClient, NotificationEvent, NotificationProcessSetup, NotificationStatus,
};
use zeroize::Zeroizing;

use crate::app::account_data::{
    NativeGlobalImagePacksSnapshot, NativeImagePack, NativeImagePackOwner,
    NativeImagePackUpdateSignal, NativeLaterSnapshot, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraLaterItemKind, SynaraRoomNoteItem,
    SynaraRoomNoteItemKind,
};
use crate::app::auth::{
    existing_sqlite_crypto_device_id, login_with_password as core_login_with_password,
    DevicePlatform, LoginOptions,
};
use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig, TimeoutPolicy};
use crate::app::devices::{
    NativeDeviceDeleteAuthentication, NativeDeviceDeleteResult, NativeDeviceOwner,
    NativeDeviceSnapshot, NativeDeviceTrust, NativeDeviceUpdateSignal,
};
use crate::app::lifecycle::{
    load_session_material, matrix_session_from_host_secrets, persist_session_after_login,
    restore_session_from_vault, restore_session_from_vault_with_room_load_settings,
    restore_session_onto_client, SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::notifications::NativeHttpPusherOwner;
use crate::app::presence::{
    NativePresenceOwner, NativePresenceSnapshotResult, NativePresenceState,
    NativePresenceSubscription, NativePresenceUpdate, NativePresenceWriteResult,
};
use crate::app::room_list::{
    NativeInvite, NativeInviteSnapshot, NativeInviteTriage, NativeRoomListOwner,
    NativeRoomListSnapshot, NativeRoomListUpdateSignal,
};
use crate::app::room_profile::{
    MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleOwner, NativeRoomJoinRuleUpdate,
};
use crate::app::store::{
    get_or_create_store_key, AccountIdentity, StoreKeyId, StoreKeyMaterial, StoreKeyVault,
    StoreKeyVaultError, STORE_KEY_LEN,
};
use crate::app::sync::{
    build_sync_service, SyncReadiness, SyncReadinessSnapshot, SyncServiceConfig, SyncServiceOwner,
};
use crate::app::timeline::{
    NativeAgentApprovalDecisionResult, NativeComposerReplyDraft, NativeDecryptionState,
    NativeReactionMutation, NativeReactionMutationResult, NativeTimelineDirection,
    NativeTimelineEventReadback, NativeTimelineItem, NativeTimelineOpenPosition,
    NativeTimelineOpenReadback, NativeTimelineOwner, NativeTimelineReaction,
    NativeTimelineReactionSender, NativeTimelineReadAction, NativeTimelineReadIntent,
    NativeTimelineReadStateReadback, NativeTimelineViewportHint, TimelineMediaHandle,
    TimelinePageState, TimelinePollAnswer, TimelinePollRow, TimelineReaction, TimelineReplyPreview,
    TimelineRowCapabilities, TimelineThreadSummary, TimelineViewDeltaBatch, TimelineViewPosition,
    TimelineViewRow, TimelineViewSnapshot, TimelineViewUpdateEmit, TIMELINE_VIEW_SCHEMA_VERSION,
};
use crate::app::typing::{NativeTypingOwner, NativeTypingSnapshot, NativeTypingUpdateSignal};
use crate::app::verification::{
    NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationOwner, NativeVerificationPhase, NativeVerificationRequest,
    NativeVerificationSas, NativeVerificationUpdateSignal,
};
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{
    CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory, MAX_ENVELOPE_PAYLOAD_JSON_BYTES,
};
use serde::Deserialize;

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
const ALREADY_RESTORED_CODE: &str = "p4-s3b-session-already-restored";
const ALREADY_RESTORED_DESCRIPTION: &str = "A live session is already restored.";
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
const NSE_STORE_NOT_OPEN_CODE: &str = "p4-s11-nse-store-not-open";
const NSE_STORE_NOT_OPEN_DESCRIPTION: &str = "The NSE read-only store is not open.";
const NSE_EVENT_NOT_IN_STORE_CODE: &str = "p4-s11-nse-event-not-in-store";
const NSE_EVENT_NOT_IN_STORE_DESCRIPTION: &str =
    "The notification event is not in the local store.";
const NSE_PAYLOAD_OVERSIZE_CODE: &str = "p4-s11-nse-payload-oversize";
const NSE_PAYLOAD_OVERSIZE_DESCRIPTION: &str = "The NSE store request exceeds the payload limit.";
const NSE_FORBIDS_ATTACH_CODE: &str = "p4-s11-nse-read-only-forbids-attach";
const NSE_FORBIDS_ATTACH_DESCRIPTION: &str =
    "The NSE read-only store cannot attach session owners.";
const NSE_FORBIDS_START_CODE: &str = "p4-s12-nse-forbids-start";
const NSE_FORBIDS_START_DESCRIPTION: &str = "The NSE read-only store cannot start SyncService.";
const NSE_FORBIDS_STOP_CODE: &str = "p4-s12-nse-forbids-stop";
const NSE_FORBIDS_STOP_DESCRIPTION: &str = "The NSE read-only store cannot stop SyncService.";
const NSE_FORBIDS_MEDIA_CODE: &str = "p4-s33-nse-forbids-media";
const NSE_FORBIDS_MEDIA_DESCRIPTION: &str =
    "The NSE read-only store cannot download timeline media.";
const TIMELINE_MEDIA_NO_SESSION_CODE: &str = "p4-s33-media-no-session";
const TIMELINE_MEDIA_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_MEDIA_UNKNOWN_HANDLE_CODE: &str = "p4-s33-media-unknown-handle";
const TIMELINE_MEDIA_UNKNOWN_HANDLE_DESCRIPTION: &str = "The timeline media handle is unknown.";
const TIMELINE_MEDIA_TOO_LARGE_CODE: &str = "p4-s33-media-too-large";
const TIMELINE_MEDIA_TOO_LARGE_DESCRIPTION: &str = "The timeline media exceeds the size limit.";
const TIMELINE_MEDIA_FAILED_CODE: &str = "p4-s33-media-failed";
const TIMELINE_MEDIA_FAILED_DESCRIPTION: &str = "Timeline media could not be downloaded.";
const SYNC_NOT_ATTACHED_CODE: &str = "p4-s12-sync-not-attached";
const SYNC_NOT_ATTACHED_DESCRIPTION: &str = "Session owners are not attached.";
const SYNC_START_FAILED_CODE: &str = "p4-s12-sync-start-failed";
const SYNC_START_FAILED_DESCRIPTION: &str = "SyncService could not be started.";
const CLIENT_RESUME_FAILED_CODE: &str = "p4-s12-client-resume-failed";
const CLIENT_RESUME_FAILED_DESCRIPTION: &str = "The Matrix client stores could not be resumed.";
const SYNC_STOP_FAILED_CODE: &str = "p4-s12-sync-stop-failed";
const SYNC_STOP_FAILED_DESCRIPTION: &str = "SyncService could not be stopped.";
const CLIENT_PAUSE_FAILED_CODE: &str = "p4-s12-client-pause-failed";
const CLIENT_PAUSE_FAILED_DESCRIPTION: &str = "The Matrix client stores could not be paused.";
const NSE_FORBIDS_POLL_CODE: &str = "p4-s14-nse-forbids-poll";
const NSE_FORBIDS_POLL_DESCRIPTION: &str =
    "The NSE read-only store cannot poll timeline view updates.";
const TIMELINE_VIEW_POLL_FAILED_CODE: &str = "p4-s14-timeline-view-poll-failed";
const TIMELINE_VIEW_POLL_FAILED_DESCRIPTION: &str = "Timeline view updates could not be polled.";
const TIMELINE_VIEW_UPDATE_QUEUE_CAP: usize = 32;
const NSE_FORBIDS_OWNER_POLL_CODE: &str = "p4-s17-nse-forbids-poll";
const NSE_FORBIDS_OWNER_POLL_DESCRIPTION: &str =
    "The NSE read-only store cannot poll owner updates.";
const OWNER_UPDATE_POLL_FAILED_CODE: &str = "p4-s17-owner-update-poll-failed";
const OWNER_UPDATE_POLL_FAILED_DESCRIPTION: &str = "Owner updates could not be polled.";
const OWNER_UPDATE_QUEUE_CAP: usize = 32;
const NSE_FORBIDS_ROOM_LIST_POLL_CODE: &str = "p4-s19-nse-forbids-poll";
const NSE_FORBIDS_ROOM_LIST_POLL_DESCRIPTION: &str =
    "The NSE read-only store cannot poll room list updates.";
const ROOM_LIST_UPDATE_POLL_FAILED_CODE: &str = "p4-s19-room-list-update-poll-failed";
const ROOM_LIST_UPDATE_POLL_FAILED_DESCRIPTION: &str = "Room list updates could not be polled.";
const ROOM_LIST_UPDATE_QUEUE_CAP: usize = 32;
const NSE_OWNERS_ATTACHED_CODE: &str = "p4-s11-nse-owners-already-attached";
const NSE_OWNERS_ATTACHED_DESCRIPTION: &str =
    "The NSE read-only store cannot open after owners attach.";
const NSE_FAILED_CODE: &str = "p4-s11-nse-store-failed";
const NSE_FAILED_DESCRIPTION: &str = "The NSE read-only store request could not be completed.";
const NSE_RESTORE_FAILED_CODE: &str = "p4-s11-nse-restore-failed";
const NSE_RESTORE_FAILED_DESCRIPTION: &str = "The NSE session could not be restored.";
const NSE_CLIENT_INIT_FAILED_CODE: &str = "p4-s11-nse-client-init-failed";
const NSE_CLIENT_INIT_FAILED_DESCRIPTION: &str =
    "The NSE notification client could not be initialized.";
const NSE_EVENT_FETCH_FAILED_CODE: &str = "p4-s11-nse-event-fetch-failed";
const NSE_EVENT_FETCH_FAILED_DESCRIPTION: &str = "The NSE notification event could not be fetched.";
const NSE_RESOLUTION_TIMEOUT_CODE: &str = "p4-s11-nse-resolution-timeout";
const NSE_RESOLUTION_TIMEOUT_DESCRIPTION: &str = "The NSE notification resolution timed out.";
const NSE_CLOSE_FAILED_CODE: &str = "p4-s11-nse-close-failed";
const NSE_CLOSE_FAILED_DESCRIPTION: &str = "The NSE read-only store could not be closed.";
const NSE_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const NSE_RESOLUTION_TIMEOUT: Duration = Duration::from_secs(20);
const NSE_STORE_LOCK_HOLDER: &str = "synara-nse-parent";
const LEFTOVER_NO_SESSION_CODE: &str = "p4-s10-leftover-no-session";
const LEFTOVER_NO_SESSION_DESCRIPTION: &str = "The leftover command requires a session.";
const LEFTOVER_OVERSIZE_CODE: &str = "p4-s10-leftover-oversize";
const LEFTOVER_OVERSIZE_DESCRIPTION: &str = "The leftover request exceeds the payload limit.";
const LEFTOVER_FAILED_CODE: &str = "p4-s10-leftover-failed";
const LEFTOVER_FAILED_DESCRIPTION: &str = "The leftover command could not be completed.";
const LEFTOVER_UNAVAILABLE_CODE: &str = "p4-s10-leftover-unavailable";
const LEFTOVER_UNAVAILABLE_DESCRIPTION: &str = "The leftover command is unavailable.";
const LEFTOVER_STORE_ROOT_INVALID_CODE: &str = "p4-s10-leftover-store-root-invalid";
const LEFTOVER_STORE_ROOT_INVALID_DESCRIPTION: &str = "The leftover store root is invalid.";
const AGENT_APPROVAL_NO_SESSION_CODE: &str = "p4-s34-agent-approval-no-session";
const AGENT_APPROVAL_NO_SESSION_DESCRIPTION: &str = "No agent approval session is available.";
const AGENT_APPROVAL_INVALID_CODE: &str = "p4-s34-agent-approval-invalid";
const AGENT_APPROVAL_INVALID_DESCRIPTION: &str = "The agent approval request is invalid.";
const AGENT_APPROVAL_FAILED_CODE: &str = "p4-s34-agent-approval-failed";
const AGENT_APPROVAL_FAILED_DESCRIPTION: &str = "The agent approval could not be sent.";
const BACKUP_STATUS_COMMAND: &str = "matrix_backup_status";
const CRYPTO_STATUS_COMMAND: &str = "matrix_crypto_status";
const CROSS_SIGNING_STATUS_COMMAND: &str = "matrix_cross_signing_status";
const ROOM_KEY_TRANSFER_STATUS_COMMAND: &str = "matrix_room_key_transfer_status";
const LEFTOVER_STATUS_GENERATION: u64 = 0;
const ATTACHED_OWNER_NAMES: &[&str] = &[
    "typing",
    "presence",
    "verification",
    "devices",
    "join_rules",
    "image_packs",
    "http_pusher",
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
const TIMELINE_SNAPSHOT_COMMAND: &str = "matrix_timeline_snapshot";
const TIMELINE_OPEN_NO_SESSION_CODE: &str = "p2-timeline-open-no-session";
const TIMELINE_CLOSE_NO_SESSION_CODE: &str = "p2-timeline-close-no-session";
const TIMELINE_PAGINATE_NO_SESSION_CODE: &str = "p2-timeline-paginate-no-session";
const TIMELINE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-timeline-snapshot-no-session";
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
const PRESENCE_SET_COMMAND: &str = "matrix_presence_set";
const TYPING_SNAPSHOT_NO_SESSION_CODE: &str = "p2-typing-snapshot-no-session";
const TYPING_SET_NO_SESSION_CODE: &str = "p2-typing-set-no-session";
const PRESENCE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-presence-snapshot-no-session";
const PRESENCE_SUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-subscribe-no-session";
const PRESENCE_UNSUBSCRIBE_NO_SESSION_CODE: &str = "p2-presence-unsubscribe-no-session";
const PRESENCE_SET_NO_SESSION_CODE: &str = "p2-presence-set-no-session";
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
const PRESENCE_SET_FAILED_CODE: &str = "p4-s7-presence-set-failed";
const PRESENCE_SET_FAILED_DESCRIPTION: &str = "The presence status could not be updated.";
const PRESENCE_INVALID_STATE_CODE: &str = "v-presence-state-unsupported";
const PRESENCE_INVALID_STATE_DESCRIPTION: &str = "The presence state is invalid.";
const PRESENCE_STATUS_MSG_CAP_CODE: &str = "p4.7-status-msg-cap";
const PRESENCE_STATUS_MSG_CAP_DESCRIPTION: &str = "The presence status message is too long.";
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
const DEVICE_DELETE_PASSWORD_NO_SESSION_CODE: &str = "p2-device-delete-password-no-session";
const DEVICE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-device-snapshot-no-session";
const DEVICE_RENAME_NO_SESSION_CODE: &str = "p2-device-rename-no-session";
const DEVICE_DELETE_START_NO_SESSION_CODE: &str = "p2-device-delete-start-no-session";
const DEVICE_DELETE_CANCEL_NO_SESSION_CODE: &str = "p2-device-delete-cancel-no-session";
const DEVICE_NO_SESSION_DESCRIPTION: &str = "No device session is available.";
const DEVICE_FAILED_CODE: &str = "p4-s9-2-device-failed";
const DEVICE_FAILED_DESCRIPTION: &str = "The device request could not be completed.";
const DEVICE_OWNER_DESCRIPTION: &str = "The device request is not available.";
const JOIN_RULE_SNAPSHOT_COMMAND: &str = "matrix_room_join_rule_snapshot";
const JOIN_RULE_SET_COMMAND: &str = "matrix_room_set_join_rule";
const JOIN_RULE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-join-rule-snapshot-no-session";
const JOIN_RULE_SET_NO_SESSION_CODE: &str = "p2-room-set-join-rule-no-session";
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
const GET_OWN_PROFILE_COMMAND: &str = "matrix_get_own_profile";
const SET_OWN_DISPLAY_NAME_NO_SESSION_CODE: &str = "p2-set-own-display-name-no-session";
const SET_OWN_AVATAR_NO_SESSION_CODE: &str = "p2-set-own-avatar-no-session";
const GET_OWN_PROFILE_NO_SESSION_CODE: &str = "p2-get-own-profile-no-session";
const OWN_PROFILE_NO_SESSION_DESCRIPTION: &str = "No own-profile session is available.";
const OWN_PROFILE_FAILED_CODE: &str = "p4-s9-8-own-profile-failed";
const OWN_PROFILE_FAILED_DESCRIPTION: &str = "The own-profile request could not be completed.";
const OWN_PROFILE_OWNER_DESCRIPTION: &str = "The own-profile request is not available.";
const IGNORED_USERS_COMMAND_GENERATION: u64 = 0;
const IGNORED_USERS_SNAPSHOT_COMMAND: &str = "matrix_ignored_users_snapshot";
const IGNORED_USERS_IGNORE_COMMAND: &str = "matrix_ignored_users_ignore";
const IGNORED_USERS_UNIGNORE_COMMAND: &str = "matrix_ignored_users_unignore";
const IGNORED_USERS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-ignored-users-snapshot-no-session";
const IGNORED_USERS_IGNORE_NO_SESSION_CODE: &str = "p2-ignored-users-ignore-no-session";
const IGNORED_USERS_UNIGNORE_NO_SESSION_CODE: &str = "p2-ignored-users-unignore-no-session";
const IGNORED_USERS_NO_SESSION_DESCRIPTION: &str = "No ignored-users session is available.";
const IGNORED_USERS_FAILED_CODE: &str = "p4-s9-ignored-users-failed";
const IGNORED_USERS_FAILED_DESCRIPTION: &str = "The ignored-users request could not be completed.";
const IGNORED_USERS_OWNER_DESCRIPTION: &str = "The ignored-users request is not available.";
const USER_DIRECTORY_SEARCH_COMMAND_GENERATION: u64 = 0;
const USER_DIRECTORY_SEARCH_COMMAND: &str = "matrix_user_directory_search";
const USER_DIRECTORY_SEARCH_NO_SESSION_CODE: &str = "p2-user-directory-search-no-session";
const USER_DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION: &str =
    "No user-directory session is available.";
const USER_DIRECTORY_SEARCH_FAILED_CODE: &str = "p4-s9-user-directory-search-failed";
const USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION: &str =
    "The user-directory search request could not be completed.";
const USER_DIRECTORY_SEARCH_OWNER_DESCRIPTION: &str =
    "The user-directory search request is not available.";
const MESSAGE_SEARCH_COMMAND_GENERATION: u64 = 0;
const MESSAGE_SEARCH_COMMAND: &str = "matrix_message_search";
const MESSAGE_SEARCH_NO_SESSION_CODE: &str = "p2-message-search-no-session";
const MESSAGE_SEARCH_NO_SESSION_DESCRIPTION: &str = "No message-search session is available.";
const MESSAGE_SEARCH_FAILED_CODE: &str = "p4-s9-message-search-failed";
const MESSAGE_SEARCH_FAILED_DESCRIPTION: &str =
    "The message-search request could not be completed.";
const MESSAGE_SEARCH_OWNER_DESCRIPTION: &str = "The message-search request is not available.";
const PUSH_RULES_COMMAND_GENERATION: u64 = 0;
const PUSH_RULES_SNAPSHOT_COMMAND: &str = "matrix_push_rules_snapshot";
const PUSH_RULES_SET_DEFAULT_COMMAND: &str = "matrix_push_rules_set_default";
const PUSH_RULES_SET_MENTION_COMMAND: &str = "matrix_push_rules_set_mention";
const PUSH_RULES_ADD_KEYWORD_COMMAND: &str = "matrix_push_rules_add_keyword";
const PUSH_RULES_REMOVE_KEYWORD_COMMAND: &str = "matrix_push_rules_remove_keyword";
const PUSH_RULES_SNAPSHOT_NO_SESSION_CODE: &str = "p2-push-rules-snapshot-no-session";
const PUSH_RULES_SET_DEFAULT_NO_SESSION_CODE: &str = "p2-push-rules-set-default-no-session";
const PUSH_RULES_SET_MENTION_NO_SESSION_CODE: &str = "p2-push-rules-set-mention-no-session";
const PUSH_RULES_ADD_KEYWORD_NO_SESSION_CODE: &str = "p2-push-rules-add-keyword-no-session";
const PUSH_RULES_REMOVE_KEYWORD_NO_SESSION_CODE: &str = "p2-push-rules-remove-keyword-no-session";
const PUSH_RULES_NO_SESSION_DESCRIPTION: &str = "No push-rules session is available.";
const PUSH_RULES_FAILED_CODE: &str = "p4-s9-push-rules-failed";
const PUSH_RULES_FAILED_DESCRIPTION: &str = "The push-rules request could not be completed.";
const PUSH_RULES_OWNER_DESCRIPTION: &str = "The push-rules request is not available.";
const ROOM_NOTIFICATION_COMMAND_GENERATION: u64 = 0;
const ROOM_NOTIFICATION_SNAPSHOT_COMMAND: &str = "matrix_room_notification_snapshot";
const ROOM_NOTIFICATION_SET_COMMAND: &str = "matrix_room_notification_set";
const ROOM_NOTIFICATIONS_SNAPSHOT_COMMAND: &str = "matrix_room_notifications_snapshot";
const ROOM_NOTIFICATION_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-notification-snapshot-no-session";
const ROOM_NOTIFICATION_SET_NO_SESSION_CODE: &str = "p2-room-notification-set-no-session";
const ROOM_NOTIFICATIONS_SNAPSHOT_NO_SESSION_CODE: &str =
    "p2-room-notifications-snapshot-no-session";
const ROOM_NOTIFICATION_NO_SESSION_DESCRIPTION: &str = "No room-notification session is available.";
const ROOM_NOTIFICATION_FAILED_CODE: &str = "p4-s9-room-notification-failed";
const ROOM_NOTIFICATION_FAILED_DESCRIPTION: &str =
    "The room-notification request could not be completed.";
const ROOM_NOTIFICATION_OWNER_DESCRIPTION: &str = "The room-notification request is not available.";
const THREEPID_COMMAND_GENERATION: u64 = 0;
const THREEPID_SNAPSHOT_COMMAND: &str = "matrix_threepid_snapshot";
const THREEPID_DELETE_COMMAND: &str = "matrix_threepid_delete";
const THREEPID_REQUEST_EMAIL_TOKEN_COMMAND: &str = "matrix_threepid_request_email_token";
const THREEPID_ADD_EMAIL_COMMAND: &str = "matrix_threepid_add_email";
const THREEPID_SNAPSHOT_NO_SESSION_CODE: &str = "p2-threepid-snapshot-no-session";
const THREEPID_DELETE_NO_SESSION_CODE: &str = "p2-threepid-delete-no-session";
const THREEPID_REQUEST_EMAIL_TOKEN_NO_SESSION_CODE: &str =
    "p2-threepid-request-email-token-no-session";
const THREEPID_ADD_EMAIL_NO_SESSION_CODE: &str = "p2-threepid-add-email-no-session";
const THREEPID_ADD_EMAIL_PASSWORD_NO_SESSION_CODE: &str =
    "p2-threepid-add-email-password-no-session";
const THREEPID_NO_SESSION_DESCRIPTION: &str = "No 3PID session is available.";
const THREEPID_FAILED_CODE: &str = "p4-s9-threepid-failed";
const THREEPID_FAILED_DESCRIPTION: &str = "The 3PID request could not be completed.";
const THREEPID_OWNER_DESCRIPTION: &str = "The 3PID request is not available.";
const UPLOAD_AVATAR_NO_SESSION_CODE: &str = "p2-upload-avatar-no-session";
const UPLOAD_CONTENT_NO_SESSION_CODE: &str = "p2-upload-content-no-session";
const UPLOAD_CONTENT_NO_SESSION_DESCRIPTION: &str = "No content-upload session is available.";
const UPLOAD_CONTENT_FAILED_CODE: &str = "p4-s9-media-upload-failed";
const UPLOAD_CONTENT_FAILED_DESCRIPTION: &str =
    "The content-upload request could not be completed.";
const UPLOAD_CONTENT_OWNER_DESCRIPTION: &str = "The content-upload request is not available.";
const SEND_ROOM_ATTACHMENT_NO_SESSION_CODE: &str = "p2-send-room-attachment-no-session";
const SEND_ROOM_ATTACHMENT_NO_SESSION_DESCRIPTION: &str =
    "No room-attachment session is available.";
const SEND_ROOM_ATTACHMENT_FAILED_CODE: &str = "p4-s9-send-room-attachment-failed";
const SEND_ROOM_ATTACHMENT_FAILED_DESCRIPTION: &str =
    "The room-attachment request could not be completed.";
const SEND_ROOM_ATTACHMENT_OWNER_DESCRIPTION: &str =
    "The room-attachment request is not available.";
const DOWNLOAD_PLAIN_MEDIA_NO_SESSION_CODE: &str = "p2-download-plain-media-no-session";
const THUMBNAIL_PLAIN_MEDIA_NO_SESSION_CODE: &str = "p2-thumbnail-plain-media-no-session";
const PLAIN_MEDIA_NO_SESSION_DESCRIPTION: &str = "No plain-media session is available.";
const PLAIN_MEDIA_FAILED_CODE: &str = "p4-s9-plain-media-failed";
const PLAIN_MEDIA_FAILED_DESCRIPTION: &str = "The plain-media request could not be completed.";
const PLAIN_MEDIA_OWNER_DESCRIPTION: &str = "The plain-media request is not available.";
const REGISTER_HTTP_PUSHER_NO_SESSION_CODE: &str = "p2-register-http-pusher-no-session";
const DELETE_HTTP_PUSHER_NO_SESSION_CODE: &str = "p2-delete-http-pusher-no-session";
const BIND_HTTP_PUSHER_NO_SESSION_CODE: &str = "p2-bind-http-pusher-no-session";
const HTTP_PUSHER_SESSION_MISMATCH_CODE: &str = "v-pusher.session-mismatch";
const HTTP_PUSHER_NO_SESSION_DESCRIPTION: &str = "No HTTP pusher session is available.";
const HTTP_PUSHER_FAILED_CODE: &str = "p4-s9-http-pusher-failed";
const HTTP_PUSHER_FAILED_DESCRIPTION: &str = "The HTTP pusher request could not be completed.";
const HTTP_PUSHER_OWNER_DESCRIPTION: &str = "The HTTP pusher request is not available.";
const RESTORE_BACKUP_NO_SESSION_CODE: &str = "p2-restore-backup-no-session";
const RESTORE_BACKUP_NO_SESSION_DESCRIPTION: &str = "No backup restore session is available.";
const RESTORE_BACKUP_FAILED_CODE: &str = "p4-s9-backup-restore-failed";
const RESTORE_BACKUP_FAILED_DESCRIPTION: &str =
    "The backup restore request could not be completed.";
const RESTORE_BACKUP_OWNER_DESCRIPTION: &str = "The backup restore request is not available.";
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
const ROOM_SET_FAVORITE_COMMAND: &str = "matrix_room_set_favorite";
const ROOM_LEAVE_NO_SESSION_CODE: &str = "p2-room-leave-no-session";
const ROOM_JOIN_NO_SESSION_CODE: &str = "p2-room-join-no-session";
const ROOM_SET_FAVORITE_NO_SESSION_CODE: &str = "p2-room-set-favorite-no-session";
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
const TIMELINE_READ_STATE_GENERATION: u64 = 0;
const TIMELINE_EVENT_READBACK_COMMAND: &str = "matrix_timeline_event_readback";
const TIMELINE_SET_READ_STATE_COMMAND: &str = "matrix_timeline_set_read_state";
const TIMELINE_JUMP_LATEST_COMMAND: &str = "matrix_timeline_jump_latest";
const TIMELINE_EVENT_READBACK_NO_SESSION_CODE: &str = "p2-timeline-event-readback-no-session";
const TIMELINE_SET_READ_STATE_NO_SESSION_CODE: &str = "p2-timeline-set-read-state-no-session";
const TIMELINE_JUMP_LATEST_NO_SESSION_CODE: &str = "p2-timeline-jump-latest-no-session";
const TIMELINE_READ_STATE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_READ_STATE_FAILED_CODE: &str = "p4-s9-19-timeline-read-state-failed";
const TIMELINE_READ_STATE_FAILED_DESCRIPTION: &str =
    "The timeline read-state request could not be completed.";
const TIMELINE_READ_STATE_OWNER_DESCRIPTION: &str =
    "The timeline read-state request is not available.";
const TIMELINE_REACTION_GENERATION: u64 = 0;
const REACTION_ENSURE_COMMAND: &str = "matrix_reaction_ensure";
const AGENT_APPROVAL_DECIDE_COMMAND: &str = "matrix_agent_approval_decide";
const REACTION_REDACT_COMMAND: &str = "matrix_reaction_redact";
const TIMELINE_REACTION_TOGGLE_COMMAND: &str = "matrix_timeline_reaction_toggle";
const REACTION_ENSURE_NO_SESSION_CODE: &str = "p2-reaction-ensure-no-session";
const REACTION_REDACT_NO_SESSION_CODE: &str = "p2-reaction-redact-no-session";
const TIMELINE_REACTION_TOGGLE_NO_SESSION_CODE: &str = "p2-timeline-reaction-toggle-no-session";
const TIMELINE_REACTION_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_REACTION_FAILED_CODE: &str = "p4-s9-20-timeline-reactions-failed";
const TIMELINE_REACTION_FAILED_DESCRIPTION: &str =
    "The timeline reaction request could not be completed.";
const TIMELINE_REACTION_OWNER_DESCRIPTION: &str = "The timeline reaction request is not available.";
const COMPOSER_REPLY_DRAFT_GENERATION: u64 = 0;
const COMPOSER_SET_REPLY_DRAFT_COMMAND: &str = "matrix_composer_set_reply_draft";
const COMPOSER_GET_REPLY_DRAFT_COMMAND: &str = "matrix_composer_get_reply_draft";
const COMPOSER_CLEAR_REPLY_DRAFT_COMMAND: &str = "matrix_composer_clear_reply_draft";
const COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-set-reply-draft-no-session";
const COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-get-reply-draft-no-session";
const COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-clear-reply-draft-no-session";
const COMPOSER_REPLY_DRAFT_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const COMPOSER_REPLY_DRAFT_FAILED_CODE: &str = "p4-s9-21-composer-reply-draft-failed";
const COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION: &str =
    "The composer reply-draft request could not be completed.";
const COMPOSER_REPLY_DRAFT_OWNER_DESCRIPTION: &str =
    "The composer reply-draft request is not available.";
const SEND_TEXT_GENERATION: u64 = 0;
const SEND_TEXT_COMMAND: &str = "matrix_send_text";
const SEND_TEXT_NO_SESSION_CODE: &str = "p2-send-text-no-session";
const SEND_TEXT_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const SEND_TEXT_FAILED_CODE: &str = "p4-s9-22-send-text-failed";
const SEND_TEXT_FAILED_DESCRIPTION: &str = "The send-text request could not be completed.";
const SEND_TEXT_OWNER_DESCRIPTION: &str = "The send-text request is not available.";
const SEND_POLL_GENERATION: u64 = 0;
const SEND_POLL_COMMAND: &str = "matrix_send_poll";
const SEND_POLL_NO_SESSION_CODE: &str = "p2-send-poll-no-session";
const SEND_POLL_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const SEND_POLL_FAILED_CODE: &str = "p4-s9-24-send-poll-failed";
const SEND_POLL_FAILED_DESCRIPTION: &str = "The send-poll request could not be completed.";
const SEND_POLL_OWNER_DESCRIPTION: &str = "The send-poll request is not available.";
const EDIT_MESSAGE_GENERATION: u64 = 0;
const EDIT_MESSAGE_COMMAND: &str = "matrix_edit_message";
const EDIT_MESSAGE_NO_SESSION_CODE: &str = "p2-edit-message-no-session";
const EDIT_MESSAGE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const EDIT_MESSAGE_FAILED_CODE: &str = "p4-s9-25-edit-message-failed";
const EDIT_MESSAGE_FAILED_DESCRIPTION: &str = "The edit-message request could not be completed.";
const EDIT_MESSAGE_OWNER_DESCRIPTION: &str = "The edit-message request is not available.";
const POLL_RESPOND_GENERATION: u64 = 0;
const POLL_RESPOND_COMMAND: &str = "matrix_poll_respond";
const POLL_RESPOND_NO_SESSION_CODE: &str = "p2-poll-respond-no-session";
const POLL_RESPOND_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const POLL_RESPOND_FAILED_CODE: &str = "p4-s9-26-poll-respond-failed";
const POLL_RESPOND_FAILED_DESCRIPTION: &str = "The poll-respond request could not be completed.";
const POLL_RESPOND_OWNER_DESCRIPTION: &str = "The poll-respond request is not available.";
const TIMELINE_MUTATE_GENERATION: u64 = 0;
const TIMELINE_EDIT_TEXT_COMMAND: &str = "matrix_timeline_edit_text";
const TIMELINE_REDACT_COMMAND: &str = "matrix_timeline_redact";
const TIMELINE_REPORT_COMMAND: &str = "matrix_timeline_report";
const TIMELINE_EDIT_TEXT_NO_SESSION_CODE: &str = "p2-timeline-edit-text-no-session";
const TIMELINE_REDACT_NO_SESSION_CODE: &str = "p2-timeline-redact-no-session";
const TIMELINE_REPORT_NO_SESSION_CODE: &str = "p2-timeline-report-no-session";
const TIMELINE_MUTATE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_MUTATE_FAILED_CODE: &str = "p4-s9-27-timeline-mutate-failed";
const TIMELINE_MUTATE_FAILED_DESCRIPTION: &str =
    "The timeline mutation request could not be completed.";
const TIMELINE_MUTATE_OWNER_DESCRIPTION: &str = "The timeline mutation request is not available.";
const TIMELINE_PIN_GENERATION: u64 = 0;
const TIMELINE_PIN_COMMAND: &str = "matrix_timeline_pin";
const TIMELINE_UNPIN_COMMAND: &str = "matrix_timeline_unpin";
const TIMELINE_PIN_NO_SESSION_CODE: &str = "p2-timeline-pin-no-session";
const TIMELINE_UNPIN_NO_SESSION_CODE: &str = "p2-timeline-unpin-no-session";
const TIMELINE_PIN_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_PIN_FAILED_CODE: &str = "p4-s9-28-timeline-pin-failed";
const TIMELINE_PIN_FAILED_DESCRIPTION: &str = "The timeline pin request could not be completed.";
const TIMELINE_PIN_OWNER_DESCRIPTION: &str = "The timeline pin request is not available.";
const TIMELINE_VOTE_DECLINE_GENERATION: u64 = 0;
const TIMELINE_POLL_VOTE_COMMAND: &str = "matrix_timeline_poll_vote";
const TIMELINE_CALL_DECLINE_COMMAND: &str = "matrix_timeline_call_decline";
const TIMELINE_POLL_VOTE_NO_SESSION_CODE: &str = "p2-timeline-poll-vote-no-session";
const TIMELINE_CALL_DECLINE_NO_SESSION_CODE: &str = "p2-timeline-call-decline-no-session";
const TIMELINE_VOTE_DECLINE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_VOTE_DECLINE_FAILED_CODE: &str = "p4-s9-29-timeline-vote-decline-failed";
const TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION: &str =
    "The timeline vote or decline request could not be completed.";
const TIMELINE_VOTE_DECLINE_OWNER_DESCRIPTION: &str =
    "The timeline vote or decline request is not available.";
const TIMELINE_FORWARD_GENERATION: u64 = 0;
const TIMELINE_FORWARD_TEXT_COMMAND: &str = "matrix_timeline_forward_text";
const TIMELINE_FORWARD_MEDIA_COMMAND: &str = "matrix_timeline_forward_media";
const TIMELINE_FORWARD_TEXT_NO_SESSION_CODE: &str = "p2-timeline-forward-text-no-session";
const TIMELINE_FORWARD_MEDIA_NO_SESSION_CODE: &str = "p2-timeline-forward-media-no-session";
const TIMELINE_FORWARD_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_FORWARD_FAILED_CODE: &str = "p4-s9-30-timeline-forward-failed";
const TIMELINE_FORWARD_FAILED_DESCRIPTION: &str =
    "The timeline forward request could not be completed.";
const TIMELINE_FORWARD_OWNER_DESCRIPTION: &str = "The timeline forward request is not available.";
const SESSION_STATUS_GENERATION: u64 = 0;
const SESSION_SNAPSHOT_COMMAND: &str = "matrix_session_snapshot";
const SYNC_STATUS_COMMAND: &str = "matrix_sync_status";
const MEDIA_CONFIG_COMMAND: &str = "matrix_media_config";
const SECRET_STORAGE_STATUS_COMMAND: &str = "matrix_secret_storage_status";
const SESSION_STATUS_FAILED_CODE: &str = "p4-s9-31-session-status-failed";
const SESSION_STATUS_FAILED_DESCRIPTION: &str =
    "The session or status request could not be completed.";
const SESSION_STATUS_OWNER_DESCRIPTION: &str = "The session or status request is not available.";
const SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID: &str = "p4.1-sync-service-error";

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

/// Privacy-safe start outcome. No tokens, URLs, or SDK error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStartDto {
    pub readiness: String,
    pub session_generation: u64,
    pub started: bool,
    pub offline_mode_enabled: bool,
}

/// Privacy-safe stop outcome. No tokens, URLs, paths, or SDK error text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStopDto {
    pub readiness: String,
    pub session_generation: u64,
    pub stopped: bool,
    pub offline_mode_enabled: bool,
}

/// Static fail-closed stop error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStopError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SyncStopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SyncStopError {}

fn sync_stop_failed(code: &'static str, description: &'static str) -> SyncStopError {
    SyncStopError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Static fail-closed start error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStartError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SyncStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SyncStartError {}

fn sync_start_failed(code: &'static str, description: &'static str) -> SyncStartError {
    SyncStartError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe drained timeline view-delta summary. No row bodies or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewUpdateDto {
    pub schema_version: u32,
    pub session_generation: u64,
    pub stream_id: String,
    pub room_id: String,
    pub revision: u64,
    pub op_count: u32,
}

/// Static fail-closed timeline view-update poll error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineViewUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineViewUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineViewUpdateError {}

fn timeline_view_poll_failed(
    code: &'static str,
    description: &'static str,
) -> TimelineViewUpdateError {
    TimelineViewUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn timeline_view_update_dto(batch: TimelineViewDeltaBatch) -> TimelineViewUpdateDto {
    TimelineViewUpdateDto {
        schema_version: batch.schema_version,
        session_generation: batch.session_generation,
        stream_id: batch.stream_id,
        room_id: batch.room_id,
        revision: batch.revision,
        op_count: u32::try_from(batch.ops.len()).unwrap_or(u32::MAX),
    }
}

/// Privacy-safe owner emit summary. No user id, tokens, or password.
/// iOS re-fetches via the existing snapshot commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerUpdateDto {
    pub family: String,
    pub session_generation: u64,
    pub room_id: Option<String>,
}

/// Static fail-closed owner-update poll error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnerUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for OwnerUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for OwnerUpdateError {}

fn owner_update_poll_failed(code: &'static str, description: &'static str) -> OwnerUpdateError {
    OwnerUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

/// Privacy-safe room-list wake-up. No room ids, names, tokens, or password.
/// iOS re-fetches via the existing snapshot command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomListUpdateDto {
    pub session_generation: u64,
}

/// Static fail-closed room-list update poll error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomListUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomListUpdateError {}

fn room_list_update_poll_failed(
    code: &'static str,
    description: &'static str,
) -> RoomListUpdateError {
    RoomListUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn push_room_list_update(queue: &Mutex<Vec<RoomListUpdateDto>>, session_generation: u64) {
    if let Ok(mut guard) = queue.lock() {
        if guard.len() >= ROOM_LIST_UPDATE_QUEUE_CAP {
            guard.remove(0);
        }
        guard.push(RoomListUpdateDto { session_generation });
    }
}

fn push_owner_update(
    queue: &Mutex<Vec<OwnerUpdateDto>>,
    family: impl Into<String>,
    session_generation: u64,
    room_id: Option<String>,
) {
    if let Ok(mut guard) = queue.lock() {
        if guard.len() >= OWNER_UPDATE_QUEUE_CAP {
            guard.remove(0);
        }
        guard.push(OwnerUpdateDto {
            family: family.into(),
            session_generation,
            room_id,
        });
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
    pub last_message_preview: Option<String>,
    pub is_encrypted: bool,
    pub encryption_status: crate::dto::RoomEncryptionStatus,
    pub notification_mode: Option<String>,
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
    pub visible_tail_event_id: Option<String>,
    pub receipt_tail_event_id: Option<String>,
    pub own_read_event_id: Option<String>,
    pub unread_anchor_event_id: Option<String>,
    pub is_marked_unread: bool,
    pub pinned_event_ids: Vec<String>,
    pub row_count: u32,
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
    pub rows: Vec<TimelineViewRowDto>,
}

/// Privacy-safe timeline view row. Message text only; no media bytes or tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewRowDto {
    pub kind: String,
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    /// Core-resolved display name with the Matrix user localpart as fallback.
    pub sender_name: String,
    /// Optional SDK-projected sender avatar. Metadata only and restricted to
    /// the Matrix `mxc://` content URI carried by the timeline profile.
    pub sender_avatar_url: Option<String>,
    pub body: String,
    pub origin_server_ts: u64,
    pub edited: bool,
    pub reply_to_event_id: Option<String>,
    pub reply_preview: Option<TimelineViewReplyPreviewDto>,
    pub thread_root_event_id: Option<String>,
    pub thread_summary: Option<TimelineViewThreadSummaryDto>,
    pub poll: Option<TimelineViewPollDto>,
    pub capabilities: Option<TimelineViewRowCapabilitiesDto>,
    pub decryption_state: Option<String>,
    pub message_type: Option<String>,
    /// Closed Core-owned dispatch route: `text` or `media`.
    pub forward_transport: Option<String>,
    pub formatted_body: Option<String>,
    pub agent_card_json: Option<String>,
    pub is_agent_approval: bool,
    pub media_filename: Option<String>,
    pub media_caption: Option<String>,
    pub reactions: Vec<TimelineViewReactionDto>,
    pub media_handle_id: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_width: Option<u32>,
    pub media_height: Option<u32>,
    pub media_duration_ms: Option<u64>,
}

/// Privacy-safe reaction count on a view row. No user ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewReactionDto {
    pub key: String,
    pub count: u32,
    pub own: Option<bool>,
}

/// Privacy-safe reply preview projected by Core. No raw event content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewReplyPreviewDto {
    pub event_id: String,
    pub sender_id: Option<String>,
    pub sender_name: String,
    pub body: String,
}

/// Privacy-safe thread summary projected by Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewThreadSummaryDto {
    pub root_event_id: String,
    pub reply_count: u32,
    pub latest_event_id: Option<String>,
}

/// One privacy-safe poll answer. Vote ownership is for the active account only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewPollAnswerDto {
    pub id: String,
    pub text: String,
    pub vote_count: u32,
    pub own: bool,
}

/// Privacy-safe poll presentation projected by Core. No voter identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewPollDto {
    pub question: String,
    pub closed: bool,
    pub max_selections: u32,
    pub answers: Vec<TimelineViewPollAnswerDto>,
}

/// Core-authoritative affordance gates for one timeline row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineViewRowCapabilitiesDto {
    pub react: bool,
    pub reply: bool,
    pub edit: bool,
    pub redact: bool,
    pub report: bool,
    pub pin: bool,
    pub forward: bool,
    pub vote: bool,
    pub decline_call: bool,
}

/// Privacy-safe timeline open readback. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineOpenDto {
    pub schema_version: u32,
    pub stream_id: String,
    pub position: TimelineViewPositionDto,
    pub snapshot: TimelineSnapshotDto,
}

/// Privacy-safe single-event item. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEventItemDto {
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    pub event_type: String,
    pub body: String,
    pub origin_server_ts: u64,
    pub decryption_state: Option<String>,
}

/// Privacy-safe single-event readback from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEventReadbackDto {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub item: TimelineEventItemDto,
}

/// Privacy-safe read-state write ack. Reuses the S6 snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineReadStateDto {
    pub action: String,
    pub receipt_sent: Option<bool>,
    pub acknowledged_event_id: Option<String>,
    pub snapshot: TimelineSnapshotDto,
}

/// Static fail-closed timeline read-state error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineReadStateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineReadStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineReadStateError {}

/// Privacy-safe reaction sender. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineReactionSenderDto {
    pub user_id: String,
    pub reaction_event_id: Option<String>,
}

/// Privacy-safe aggregated reaction. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineReactionDto {
    pub key: String,
    pub count: u32,
    pub me: bool,
    pub senders: Vec<TimelineReactionSenderDto>,
}

/// Privacy-safe reaction mutation ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineReactionMutationDto {
    pub room_id: String,
    pub target_event_id: String,
    pub key: String,
    pub mutation: String,
    pub readback: Option<TimelineReactionDto>,
}

/// Privacy-safe result of the shared-core approval decision route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApprovalDecisionDto {
    pub room_id: String,
    pub event_id: String,
    pub status: String,
    pub reaction: Option<TimelineReactionMutationDto>,
}

/// Static fail-closed timeline reaction error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineReactionError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineReactionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineReactionError {}

/// Privacy-safe composer reply-draft preview. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerReplyDraftPreviewDto {
    pub event_id: String,
    pub sender_id: String,
    pub body: String,
    pub formatted_body: Option<String>,
    pub thread_root_event_id: Option<String>,
}

/// Privacy-safe composer reply-draft readback from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerReplyDraftDto {
    pub schema_version: u32,
    pub room_id: String,
    pub status: String,
    pub draft: Option<ComposerReplyDraftPreviewDto>,
}

/// Static fail-closed composer reply-draft error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerReplyDraftError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ComposerReplyDraftError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ComposerReplyDraftError {}

/// Privacy-safe send-text write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendTextDto {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: String,
}

/// Privacy-safe agent-approval write acknowledgement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentApprovalSendDto {
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed agent-approval error. Input values are never echoed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentApprovalSendError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for AgentApprovalSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for AgentApprovalSendError {}

/// Static fail-closed send-text error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendTextError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendTextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendTextError {}

/// Privacy-safe generic content upload result. mxc URI only; never bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaUploadDto {
    pub mxc: String,
}

/// Static fail-closed content-upload error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaUploadError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MediaUploadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MediaUploadError {}

/// Privacy-safe room attachment send ack. Event id and status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendRoomAttachmentDto {
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed room-attachment error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendRoomAttachmentError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendRoomAttachmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendRoomAttachmentError {}

/// Original-file or thumbnail bytes for a plain `mxc://`. Callers must not log the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaBytesDto {
    pub payload: Vec<u8>,
}

/// Static fail-closed plain-media download error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlainMediaError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PlainMediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PlainMediaError {}

/// Privacy-safe send-poll write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendPollDto {
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed send-poll error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendPollError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SendPollError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SendPollError {}

/// Privacy-safe edit-message write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditMessageDto {
    pub room_id: String,
    pub event_id: String,
    pub local_txn_id: String,
    pub status: String,
}

/// Static fail-closed edit-message error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMessageError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for EditMessageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for EditMessageError {}

/// Privacy-safe poll-respond write ack from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PollRespondDto {
    pub room_id: String,
    pub poll_event_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed poll-respond error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PollRespondError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PollRespondError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PollRespondError {}

/// Privacy-safe timeline edit/redact/report write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineMutateDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline edit/redact/report error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineMutateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineMutateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineMutateError {}

/// Privacy-safe timeline pin/unpin write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelinePinDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline pin/unpin error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelinePinError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelinePinError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelinePinError {}

/// Privacy-safe timeline poll-vote / call-decline write ack from the
/// registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineVoteDeclineDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline poll-vote / call-decline error. Fields are
/// source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineVoteDeclineError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineVoteDeclineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineVoteDeclineError {}

/// Privacy-safe timeline forward write ack from the registered Core commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineForwardDto {
    pub schema_version: u32,
    pub action: String,
    pub room_id: String,
    pub event_id: String,
    pub status: String,
}

/// Static fail-closed timeline forward error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineForwardError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineForwardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineForwardError {}

/// Privacy-safe live session snapshot from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshotDto {
    pub status: String,
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub homeserver_url: Option<String>,
    pub session_generation: Option<u64>,
}

/// Privacy-safe sync readiness from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncStatusDto {
    pub readiness: String,
    pub session_generation: u64,
    pub offline_mode_enabled: bool,
    pub failure_diagnostic_id: Option<String>,
    pub sliding_sync_capable: Option<bool>,
}

/// Privacy-safe media upload-size config from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConfigDto {
    pub upload_size: u64,
}

/// Privacy-safe secret-storage status from the registered Core command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretStorageStatusDto {
    pub session_generation: u64,
    pub state: String,
    pub exists: bool,
    pub unlocked: bool,
    pub default_key_set: bool,
    pub passphrase_configured: bool,
    pub bootstrap_ready: bool,
    pub missing_secrets: Vec<String>,
    pub action: String,
}

/// Static fail-closed session/status error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatusError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for SessionStatusError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for SessionStatusError {}

/// Privacy-safe NSE store status. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseStoreDto {
    pub read_only: bool,
    pub owners_attached: bool,
    pub sync_started: bool,
}

/// Local-store notification preview. Tokens never appear here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NseEventPreviewDto {
    pub event_type: String,
    pub sender_id: Option<String>,
    pub body: Option<String>,
    pub message_type: Option<String>,
}

/// Static fail-closed NSE store error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NseStoreError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for NseStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for NseStoreError {}

fn nse_failed(code: &'static str, description: &'static str) -> NseStoreError {
    NseStoreError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn bounded_nse_preview_text(value: &str, maximum_characters: usize) -> String {
    value.chars().take(maximum_characters).collect()
}

fn nse_message_type(message: &MessageType) -> Option<&'static str> {
    match message {
        MessageType::Audio(_) => Some("m.audio"),
        MessageType::Emote(_) => Some("m.emote"),
        MessageType::File(_) => Some("m.file"),
        MessageType::Image(_) => Some("m.image"),
        MessageType::Location(_) => Some("m.location"),
        MessageType::Notice(_) => Some("m.notice"),
        MessageType::ServerNotice(_) => Some("m.server_notice"),
        MessageType::Text(_) => Some("m.text"),
        MessageType::Video(_) => Some("m.video"),
        MessageType::VerificationRequest(_) => Some("m.key.verification.request"),
        _ => None,
    }
}

/// Privacy-safe leftover backup status. No passphrase or recovery secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupStatusDto {
    pub session_generation: u64,
    pub availability: String,
    pub enabled: bool,
    pub device_state: String,
    pub recovery_state: String,
    pub action: String,
}

/// Privacy-safe leftover crypto status. No key material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoStatusDto {
    pub session_generation: u64,
    pub encryption_enabled: bool,
    pub cross_signing_state: String,
}

/// Privacy-safe leftover cross-signing status. No private keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossSigningStatusDto {
    pub session_generation: u64,
    pub readiness: String,
}

/// Privacy-safe leftover room-key transfer status. No passphrase or path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomKeyTransferStatusDto {
    pub session_generation: u64,
    pub phase: String,
    pub keys_processed: u32,
}

/// Privacy-safe leftover write ack. Status only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftoverAckDto {
    pub status: String,
}

/// Privacy-safe leftover bytes readback. Callers must not log the payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftoverBytesDto {
    pub payload: Vec<u8>,
}

/// Static fail-closed leftover error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftoverCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for LeftoverCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for LeftoverCommandError {}

fn leftover_failed(code: &str, description: &'static str) -> LeftoverCommandError {
    LeftoverCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_restore_to_nse(error: SessionRestoreError) -> NseStoreError {
    match error {
        SessionRestoreError::Failed { code, .. }
            if code == IDENTITY_INVALID_CODE
                || code == STORE_ROOT_INVALID_CODE
                || code == MATERIAL_MISSING_CODE
                || code == VAULT_UNAVAILABLE_CODE =>
        {
            let description = if code == IDENTITY_INVALID_CODE {
                IDENTITY_INVALID_DESCRIPTION
            } else if code == STORE_ROOT_INVALID_CODE {
                STORE_ROOT_INVALID_DESCRIPTION
            } else if code == MATERIAL_MISSING_CODE {
                MATERIAL_MISSING_DESCRIPTION
            } else {
                VAULT_UNAVAILABLE_DESCRIPTION
            };
            NseStoreError::Failed {
                code,
                description: description.to_owned(),
            }
        }
        _ => nse_failed(NSE_RESTORE_FAILED_CODE, NSE_RESTORE_FAILED_DESCRIPTION),
    }
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

/// Static fail-closed native media-handle error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineMediaError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for TimelineMediaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for TimelineMediaError {}

fn timeline_media_failed(code: &'static str, description: &'static str) -> TimelineMediaError {
    TimelineMediaError::Failed {
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

fn map_timeline_snapshot_core_error(error: MatrixIpcError) -> TimelineError {
    match error.diagnostic_id.as_deref() {
        Some("p2-timeline-snapshot-no-session") => timeline_failed(
            TIMELINE_SNAPSHOT_NO_SESSION_CODE,
            TIMELINE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-timeline-view-not-open") => timeline_failed(
            TIMELINE_VIEW_NOT_OPEN_CODE,
            TIMELINE_VIEW_NOT_OPEN_DESCRIPTION,
        ),
        _ => timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION),
    }
}

fn open_position_from_dto(
    position: TimelineOpenPositionDto,
) -> Result<NativeTimelineOpenPosition, TimelineError> {
    match position.kind.as_str() {
        "live" | "live_bottom" => Ok(NativeTimelineOpenPosition::LiveBottom),
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
        visible_tail_event_id: snapshot.read_state.visible_tail_event_id,
        receipt_tail_event_id: snapshot.read_state.receipt_tail_event_id,
        own_read_event_id: snapshot.read_state.own_read_event_id,
        unread_anchor_event_id: snapshot.read_state.unread_anchor_event_id,
        is_marked_unread: snapshot.read_state.is_marked_unread,
        pinned_event_ids: snapshot.pinned_event_ids,
        row_count: u32::try_from(snapshot.rows.len()).unwrap_or(u32::MAX),
        mark_read: snapshot.capabilities.mark_read,
        mark_unread: snapshot.capabilities.mark_unread,
        paginate_backward: snapshot.capabilities.paginate_backward,
        paginate_forward: snapshot.capabilities.paginate_forward,
        rows: snapshot
            .rows
            .into_iter()
            .map(timeline_view_row_dto)
            .collect(),
    }
}

fn view_reaction_dtos(reactions: Vec<TimelineReaction>) -> Vec<TimelineViewReactionDto> {
    reactions
        .into_iter()
        .map(|reaction| TimelineViewReactionDto {
            key: reaction.key,
            count: reaction.count,
            own: reaction.own,
        })
        .collect()
}

type ViewMediaFields = (
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<u64>,
);

fn view_media_fields(media: Option<TimelineMediaHandle>) -> ViewMediaFields {
    match media {
        Some(handle) => (
            Some(handle.handle_id),
            handle.mime_type,
            handle.width,
            handle.height,
            handle.duration_ms,
        ),
        None => (None, None, None, None, None),
    }
}

fn view_reply_preview_dto(preview: TimelineReplyPreview) -> TimelineViewReplyPreviewDto {
    TimelineViewReplyPreviewDto {
        event_id: preview.event_id,
        sender_id: preview.sender_id,
        sender_name: preview.sender_name,
        body: preview.body,
    }
}

fn view_thread_summary_dto(summary: TimelineThreadSummary) -> TimelineViewThreadSummaryDto {
    TimelineViewThreadSummaryDto {
        root_event_id: summary.root_event_id,
        reply_count: summary.reply_count,
        latest_event_id: summary.latest_event_id,
    }
}

fn view_poll_answer_dto(answer: TimelinePollAnswer) -> TimelineViewPollAnswerDto {
    TimelineViewPollAnswerDto {
        id: answer.id,
        text: answer.text,
        vote_count: answer.vote_count,
        own: answer.own,
    }
}

fn view_poll_dto(poll: &TimelinePollRow) -> TimelineViewPollDto {
    TimelineViewPollDto {
        question: poll.question.clone(),
        closed: poll.closed,
        max_selections: poll.max_selections,
        answers: poll
            .answers
            .iter()
            .cloned()
            .map(view_poll_answer_dto)
            .collect(),
    }
}

fn view_row_capabilities_dto(
    capabilities: TimelineRowCapabilities,
) -> TimelineViewRowCapabilitiesDto {
    TimelineViewRowCapabilitiesDto {
        react: capabilities.react,
        reply: capabilities.reply,
        edit: capabilities.edit,
        redact: capabilities.redact,
        report: capabilities.report,
        pin: capabilities.pin,
        forward: capabilities.forward,
        vote: capabilities.vote,
        decline_call: capabilities.decline_call,
    }
}

fn timeline_view_row_dto(row: TimelineViewRow) -> TimelineViewRowDto {
    match row {
        TimelineViewRow::Message(message) => {
            let (media_handle_id, media_mime_type, media_width, media_height, media_duration_ms) =
                view_media_fields(message.media);
            let reply_preview = message.reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            let thread_summary = message.thread.map(view_thread_summary_dto);
            let capabilities = Some(view_row_capabilities_dto(message.event.capabilities));
            TimelineViewRowDto {
                kind: "message".to_owned(),
                item_id: message.event.item_id,
                event_id: message.event.event_id.unwrap_or_default(),
                sender: message.event.sender_id,
                sender_name: message.event.sender_name,
                sender_avatar_url: message.event.sender_avatar_url,
                body: message.body,
                origin_server_ts: message.event.origin_server_ts,
                edited: message.edited,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: message.thread_root,
                thread_summary,
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: message.message_type,
                forward_transport: message
                    .forward_transport
                    .map(|transport| transport.as_str().to_owned()),
                formatted_body: message.formatted_body,
                agent_card_json: message.agent_card_json,
                is_agent_approval: message.is_agent_approval,
                media_filename: message.media_filename,
                media_caption: message.media_caption,
                reactions: view_reaction_dtos(message.reactions),
                media_handle_id,
                media_mime_type,
                media_width,
                media_height,
                media_duration_ms,
            }
        }
        TimelineViewRow::Sticker {
            event,
            media,
            forward_transport,
            reply,
            thread_root,
            thread,
            reactions,
        } => {
            let (media_handle_id, media_mime_type, media_width, media_height, media_duration_ms) =
                view_media_fields(Some(media));
            let capabilities = Some(view_row_capabilities_dto(event.capabilities));
            let reply_preview = reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            TimelineViewRowDto {
                kind: "sticker".to_owned(),
                item_id: event.item_id,
                event_id: event.event_id.unwrap_or_default(),
                sender: event.sender_id,
                sender_name: event.sender_name,
                sender_avatar_url: event.sender_avatar_url,
                body: String::new(),
                origin_server_ts: event.origin_server_ts,
                edited: false,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: thread_root,
                thread_summary: thread.map(view_thread_summary_dto),
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: Some("m.sticker".to_owned()),
                forward_transport: Some(forward_transport.as_str().to_owned()),
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: view_reaction_dtos(reactions),
                media_handle_id,
                media_mime_type,
                media_width,
                media_height,
                media_duration_ms,
            }
        }
        TimelineViewRow::Poll(poll) => {
            let poll_dto = view_poll_dto(&poll);
            let capabilities = view_row_capabilities_dto(poll.event.capabilities);
            let reply_preview = poll.reply.map(view_reply_preview_dto);
            let reply_to_event_id = reply_preview
                .as_ref()
                .map(|preview| preview.event_id.clone());
            TimelineViewRowDto {
                kind: "poll".to_owned(),
                item_id: poll.event.item_id,
                event_id: poll.event.event_id.unwrap_or_default(),
                sender: poll.event.sender_id,
                sender_name: poll.event.sender_name,
                sender_avatar_url: poll.event.sender_avatar_url,
                body: poll.question,
                origin_server_ts: poll.event.origin_server_ts,
                edited: false,
                reply_to_event_id,
                reply_preview,
                thread_root_event_id: poll.thread_root,
                thread_summary: poll.thread.map(view_thread_summary_dto),
                poll: Some(poll_dto),
                capabilities: Some(capabilities),
                decryption_state: None,
                message_type: None,
                forward_transport: None,
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: view_reaction_dtos(poll.reactions),
                media_handle_id: None,
                media_mime_type: None,
                media_width: None,
                media_height: None,
                media_duration_ms: None,
            }
        }
        TimelineViewRow::Membership(membership) => TimelineViewRowDto {
            kind: "membership".to_owned(),
            item_id: membership.event.item_id,
            event_id: membership.event.event_id.unwrap_or_default(),
            sender: membership.event.sender_id,
            sender_name: membership.event.sender_name,
            sender_avatar_url: membership.event.sender_avatar_url,
            body: membership.summary,
            origin_server_ts: membership.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(membership.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::State(state) => TimelineViewRowDto {
            kind: "state".to_owned(),
            item_id: state.event.item_id,
            event_id: state.event.event_id.unwrap_or_default(),
            sender: state.event.sender_id,
            sender_name: state.event.sender_name,
            sender_avatar_url: state.event.sender_avatar_url,
            body: state.summary,
            origin_server_ts: state.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(state.event.capabilities)),
            decryption_state: None,
            message_type: Some(state.state_type),
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::Call(call) => TimelineViewRowDto {
            kind: "call".to_owned(),
            item_id: call.event.item_id,
            event_id: call.event.event_id.unwrap_or_default(),
            sender: call.event.sender_id,
            sender_name: call.event.sender_name,
            sender_avatar_url: call.event.sender_avatar_url,
            body: call.call_kind,
            origin_server_ts: call.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(call.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::Redacted(redacted) => TimelineViewRowDto {
            kind: "redacted".to_owned(),
            item_id: redacted.event.item_id,
            event_id: redacted.event.event_id.unwrap_or_default(),
            sender: redacted.event.sender_id,
            sender_name: redacted.event.sender_name,
            sender_avatar_url: redacted.event.sender_avatar_url,
            body: redacted.summary,
            origin_server_ts: redacted.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(redacted.event.capabilities)),
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::EncryptedUnavailable(encrypted) => TimelineViewRowDto {
            kind: "encrypted".to_owned(),
            item_id: encrypted.event.item_id,
            event_id: encrypted.event.event_id.unwrap_or_default(),
            sender: encrypted.event.sender_id,
            sender_name: encrypted.event.sender_name,
            sender_avatar_url: encrypted.event.sender_avatar_url,
            body: encrypted.reason_code.clone(),
            origin_server_ts: encrypted.event.origin_server_ts,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: Some(view_row_capabilities_dto(encrypted.event.capabilities)),
            decryption_state: Some(encrypted.reason_code),
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::Other(other) => {
            let (sender, sender_name, sender_avatar_url, origin_server_ts, capabilities) = other
                .event
                .map(|event| {
                    (
                        event.sender_id,
                        event.sender_name,
                        event.sender_avatar_url,
                        event.origin_server_ts,
                        Some(view_row_capabilities_dto(event.capabilities)),
                    )
                })
                .unwrap_or_else(|| (String::new(), String::new(), None, 0, None));
            TimelineViewRowDto {
                kind: "other".to_owned(),
                item_id: other.item_id,
                event_id: other.event_id.unwrap_or_default(),
                sender,
                sender_name,
                sender_avatar_url,
                body: other.summary,
                origin_server_ts,
                edited: false,
                reply_to_event_id: None,
                reply_preview: None,
                thread_root_event_id: None,
                thread_summary: None,
                poll: None,
                capabilities,
                decryption_state: None,
                message_type: other.event_type,
                forward_transport: other
                    .forward_transport
                    .map(|transport| transport.as_str().to_owned()),
                formatted_body: None,
                agent_card_json: None,
                is_agent_approval: false,
                media_filename: None,
                media_caption: None,
                reactions: Vec::new(),
                media_handle_id: None,
                media_mime_type: None,
                media_width: None,
                media_height: None,
                media_duration_ms: None,
            }
        }
        TimelineViewRow::DateSeparator {
            item_id,
            timestamp_ms,
        } => TimelineViewRowDto {
            kind: "date_separator".to_owned(),
            item_id,
            event_id: String::new(),
            sender: String::new(),
            sender_name: String::new(),
            sender_avatar_url: None,
            body: String::new(),
            origin_server_ts: timestamp_ms,
            edited: false,
            reply_to_event_id: None,
            reply_preview: None,
            thread_root_event_id: None,
            thread_summary: None,
            poll: None,
            capabilities: None,
            decryption_state: None,
            message_type: None,
            forward_transport: None,
            formatted_body: None,
            agent_card_json: None,
            is_agent_approval: false,
            media_filename: None,
            media_caption: None,
            reactions: Vec::new(),
            media_handle_id: None,
            media_mime_type: None,
            media_width: None,
            media_height: None,
            media_duration_ms: None,
        },
        TimelineViewRow::ReadMarker { item_id } => virtual_row_dto("read_marker", item_id),
        TimelineViewRow::UnreadMarker { item_id } => virtual_row_dto("unread_marker", item_id),
        TimelineViewRow::TimelineStart { item_id } => virtual_row_dto("timeline_start", item_id),
        TimelineViewRow::Pagination { item_id, .. } => virtual_row_dto("pagination", item_id),
    }
}

fn virtual_row_dto(kind: &str, item_id: String) -> TimelineViewRowDto {
    TimelineViewRowDto {
        kind: kind.to_owned(),
        item_id,
        event_id: String::new(),
        sender: String::new(),
        sender_name: String::new(),
        sender_avatar_url: None,
        body: String::new(),
        origin_server_ts: 0,
        edited: false,
        reply_to_event_id: None,
        reply_preview: None,
        thread_root_event_id: None,
        thread_summary: None,
        poll: None,
        capabilities: None,
        decryption_state: None,
        message_type: None,
        forward_transport: None,
        formatted_body: None,
        agent_card_json: None,
        is_agent_approval: false,
        media_filename: None,
        media_caption: None,
        reactions: Vec::new(),
        media_handle_id: None,
        media_mime_type: None,
        media_width: None,
        media_height: None,
        media_duration_ms: None,
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

/// Privacy-safe presence SET ack. Status only; never echo state or statusMsg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceWriteDto {
    pub status: String,
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

fn map_presence_set_core_error(error: MatrixIpcError) -> PresenceCommandError {
    match error.diagnostic_id.as_deref() {
        Some("p2-presence-set-no-session") => presence_failed(
            PRESENCE_SET_NO_SESSION_CODE,
            PRESENCE_NO_SESSION_DESCRIPTION,
        ),
        Some("v-presence-state-unsupported") => presence_failed(
            PRESENCE_INVALID_STATE_CODE,
            PRESENCE_INVALID_STATE_DESCRIPTION,
        ),
        Some("p4.7-status-msg-cap") => presence_failed(
            PRESENCE_STATUS_MSG_CAP_CODE,
            PRESENCE_STATUS_MSG_CAP_DESCRIPTION,
        ),
        _ => presence_failed(PRESENCE_SET_FAILED_CODE, PRESENCE_SET_FAILED_DESCRIPTION),
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

/// Privacy-safe verification request row. Identity/flow fields and optional
/// display-only SAS values; no tokens, MACs, or key material.
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
        NativeVerificationPhase::KeysExchanging => "keys_exchanging",
        NativeVerificationPhase::SasReady => "sas_ready",
        NativeVerificationPhase::Confirmed => "confirmed",
        NativeVerificationPhase::Done => "done",
        NativeVerificationPhase::Mismatched => "mismatched",
        NativeVerificationPhase::Cancelled => "cancelled",
        NativeVerificationPhase::Failed => "failed",
    }
    .to_owned()
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
    /// Serializes the complete SyncService/Client lifecycle transaction.
    /// Shell-side ordering remains useful, but the persistence boundary must
    /// remain correct for every current and future FFI caller.
    sync_lifecycle: tokio::sync::Mutex<()>,
    nse_read_only: Mutex<bool>,
    timeline_view_updates: Arc<Mutex<Vec<TimelineViewDeltaBatch>>>,
    owner_updates: Arc<Mutex<Vec<OwnerUpdateDto>>>,
    room_list_updates: Arc<Mutex<Vec<RoomListUpdateDto>>>,
    room_list_live: Arc<Mutex<Option<NativeRoomListOwner>>>,
}

impl Default for SharedCore {
    fn default() -> Self {
        Self::new()
    }
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
            sync_lifecycle: tokio::sync::Mutex::new(()),
            nse_read_only: Mutex::new(false),
            timeline_view_updates: Arc::new(Mutex::new(Vec::new())),
            owner_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_live: Arc::new(Mutex::new(None)),
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
            sync_lifecycle: tokio::sync::Mutex::new(()),
            nse_read_only: Mutex::new(false),
            timeline_view_updates: Arc::new(Mutex::new(Vec::new())),
            owner_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_updates: Arc::new(Mutex::new(Vec::new())),
            room_list_live: Arc::new(Mutex::new(None)),
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
        self.restore_persisted_session_with_policy(user_id, homeserver_url, store_root, false, None)
            .await
    }

    async fn restore_persisted_session_with_policy(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        nse_read_only: bool,
        room_load_settings: Option<RoomLoadSettings>,
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

        let store_key = if nse_read_only {
            store_key_for_read_only(&self.secret_store, &identity)?
        } else {
            store_key_for(&self.secret_store, &identity)?
        };
        let mut config =
            ClientBuildConfig::product_default(root, identity.clone(), Some(store_key))
                .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if nse_read_only {
            config = config
                .with_timeouts(TimeoutPolicy {
                    request_timeout: NSE_REQUEST_TIMEOUT,
                    retry_limit: 0,
                })
                .and_then(|config| {
                    config.with_cross_process_store_lock_holder(NSE_STORE_LOCK_HOLDER)
                })
                .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
            config.handle_refresh_tokens = false;
        }
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        if !nse_read_only {
            install_session_rotation_callbacks(
                &client,
                identity.clone(),
                Arc::clone(&self.secret_store),
            )
            .map_err(|_| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))?;
        }
        let outcome = if let Some(room_load_settings) = room_load_settings {
            restore_session_from_vault_with_room_load_settings(
                &client,
                &identity,
                &vault,
                room_load_settings,
            )
            .await
        } else {
            restore_session_from_vault(&client, &identity, &vault).await
        }
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
        let existing_device_id = existing_sqlite_crypto_device_id(
            config.state_store_path(),
            config.store_passphrase_hex().as_deref(),
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let client = build_unauthenticated_client(&config)
            .await
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        install_session_rotation_callbacks(
            &client,
            identity.clone(),
            Arc::clone(&self.secret_store),
        )
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let outcome = core_login_with_password(
            &client,
            identity.user_id(),
            password,
            &LoginOptions {
                request_refresh_token: true,
                device_display_name: Some(DevicePlatform::Ios.device_display_name().to_owned()),
                device_id: existing_device_id,
            },
        )
        .await
        .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        let live_identity = AccountIdentity::new(&outcome.user_id, &outcome.homeserver_url)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;
        if live_identity != identity {
            return Err(login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION));
        }
        self.persist_open_and_retain(client, &live_identity, &vault, claim, outcome.device_id)
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
        install_session_rotation_callbacks(
            &client,
            identity.clone(),
            Arc::clone(&self.secret_store),
        )
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
        self.persist_open_and_retain(client, &identity, &vault, claim, device_id)
            .await
    }

    async fn persist_open_and_retain(
        &self,
        client: Client,
        identity: &AccountIdentity,
        vault: &SecretStoreSessionVault,
        claim: RestoreClaim<'_>,
        device_id: String,
    ) -> Result<SessionLoginDto, SessionLoginError> {
        persist_session_after_login(&client, identity, vault)
            .map_err(|_| login_failed(LOGIN_FAILED_CODE, LOGIN_FAILED_DESCRIPTION))?;

        let snapshot = SessionSnapshot {
            session_generation: 1,
            user_id: identity.user_id().to_owned(),
            device_id: device_id.clone(),
            homeserver_url: identity.homeserver_url().to_owned(),
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
            user_id: identity.user_id().to_owned(),
            device_id,
            homeserver_url: identity.homeserver_url().to_owned(),
        })
    }

    /// Attach the desktop owner set on the retained Client. No Core.command.
    ///
    /// Builds owners with a queued timeline view-delta sink (P4-S14).
    /// Other product emits stay no-op. Platform::emit is still not used
    /// for product events. SyncService is attached but not started;
    /// P4-S12 `start_sync` starts it. Fail-closed if no Client is
    /// retained or owners are already attached.
    pub async fn attach_session_owners(&self) -> Result<SessionAttachDto, SessionAttachError> {
        if self.is_nse_read_only() {
            return Err(attach_failed(
                NSE_FORBIDS_ATTACH_CODE,
                NSE_FORBIDS_ATTACH_DESCRIPTION,
            ));
        }
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

        let owner_updates = Arc::clone(&self.owner_updates);
        let typing_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeTypingUpdateSignal| {
                push_owner_update(
                    &queue,
                    "typing",
                    update.session_generation,
                    Some(update.room_id),
                );
            })
        };
        let typing = Arc::new(
            NativeTypingOwner::with_emit(&client, typing_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let presence_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativePresenceUpdate| {
                let _ = update;
                push_owner_update(&queue, "presence", generation, None);
            })
        };
        let presence = Arc::new(
            NativePresenceOwner::start(&client, presence_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let verification_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeVerificationUpdateSignal| {
                push_owner_update(&queue, "verification", update.session_generation, None);
            })
        };
        let verification = Arc::new(NativeVerificationOwner::with_emit(
            &client,
            verification_emit,
            generation,
        ));
        let devices_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeDeviceUpdateSignal| {
                push_owner_update(&queue, "devices", update.session_generation, None);
            })
        };
        let devices = Arc::new(
            NativeDeviceOwner::start(&client, devices_emit, generation)
                .await
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let join_rules_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeRoomJoinRuleUpdate| {
                let (session_generation, room_id) = match update {
                    NativeRoomJoinRuleUpdate::Ready {
                        room_id,
                        session_generation,
                        ..
                    }
                    | NativeRoomJoinRuleUpdate::Unavailable {
                        room_id,
                        session_generation,
                    } => (session_generation, Some(room_id)),
                };
                push_owner_update(&queue, "join_rules", session_generation, room_id);
            })
        };
        let join_rules = Arc::new(
            NativeRoomJoinRuleOwner::start(&client, join_rules_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let image_packs_emit = {
            let queue = Arc::clone(&owner_updates);
            Arc::new(move |update: NativeImagePackUpdateSignal| {
                push_owner_update(&queue, "image_packs", update.session_generation, None);
            })
        };
        let image_packs = Arc::new(
            NativeImagePackOwner::start(&client, image_packs_emit, generation)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let http_pusher = Arc::new(
            NativeHttpPusherOwner::new(&client)
                .map_err(|_| attach_failed(ATTACH_FAILED_CODE, ATTACH_FAILED_DESCRIPTION))?,
        );
        let timeline_updates = Arc::clone(&self.timeline_view_updates);
        let timeline_emit: TimelineViewUpdateEmit = Arc::new(move |batch| {
            if let Ok(mut guard) = timeline_updates.lock() {
                if guard.len() >= TIMELINE_VIEW_UPDATE_QUEUE_CAP {
                    guard.remove(0);
                }
                guard.push(batch);
            }
        });
        let timelines = Arc::new(NativeTimelineOwner::new(&client, timeline_emit, generation));
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
            .attach_http_pusher(http_pusher)
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

    /// Start the already-attached SyncService. Not `Core.command`.
    ///
    /// NSE forbids this. Missing attach fail-closes. Start failures stay
    /// static and never echo user id, homeserver, device id, or tokens.
    /// Reopens every retained Client store before starting sync. `resume()` is
    /// idempotent, so the first start and repeated foreground activation share
    /// one lifecycle route. A second start is a restart of the same owner.
    /// This is not iOS-on-engine and not P4 acceptance.
    pub async fn start_sync(&self) -> Result<SyncStartDto, SyncStartError> {
        if self.is_nse_read_only() {
            return Err(sync_start_failed(
                NSE_FORBIDS_START_CODE,
                NSE_FORBIDS_START_DESCRIPTION,
            ));
        }
        let _lifecycle = self.sync_lifecycle.lock().await;
        if !self.owners_attached() {
            return Err(sync_start_failed(
                SYNC_NOT_ATTACHED_CODE,
                SYNC_NOT_ATTACHED_DESCRIPTION,
            ));
        }
        let client = self.retained_client().map_err(|_| {
            sync_start_failed(CLIENT_RESUME_FAILED_CODE, CLIENT_RESUME_FAILED_DESCRIPTION)
        })?;
        client.resume().await.map_err(|_| {
            sync_start_failed(CLIENT_RESUME_FAILED_CODE, CLIENT_RESUME_FAILED_DESCRIPTION)
        })?;
        let snapshot = self.core.start_attached_sync().await.map_err(|code| {
            if code == SYNC_NOT_ATTACHED_CODE {
                sync_start_failed(SYNC_NOT_ATTACHED_CODE, SYNC_NOT_ATTACHED_DESCRIPTION)
            } else {
                sync_start_failed(SYNC_START_FAILED_CODE, SYNC_START_FAILED_DESCRIPTION)
            }
        })?;
        self.spawn_room_list_live();
        let snapshot = match self.core.attached_sync_owner() {
            Some(owner) => wait_for_started_readiness(owner.as_ref(), snapshot).await,
            None => snapshot,
        };
        Ok(sync_start_dto_from_snapshot(snapshot))
    }

    /// Quiesce the retained Client for iOS suspension without logging out or
    /// replacing the session owner set.
    ///
    /// Matrix SDK lifecycle order is authoritative: stop SyncService first,
    /// then `Client::pause()` to disable send queues, await every in-flight
    /// store operation, and release all SQLite connections and file locks.
    /// Returning `stopped = true` therefore means the complete persistence
    /// boundary is safe for OS suspension, not merely that network sync ended.
    pub async fn stop_sync(&self) -> Result<SyncStopDto, SyncStopError> {
        if self.is_nse_read_only() {
            return Err(sync_stop_failed(
                NSE_FORBIDS_STOP_CODE,
                NSE_FORBIDS_STOP_DESCRIPTION,
            ));
        }
        let _lifecycle = self.sync_lifecycle.lock().await;
        if !self.owners_attached() {
            return Err(sync_stop_failed(
                SYNC_NOT_ATTACHED_CODE,
                SYNC_NOT_ATTACHED_DESCRIPTION,
            ));
        }
        if let Ok(mut live) = self.room_list_live.lock() {
            *live = None;
        }
        let snapshot = self.core.stop_attached_sync().await.map_err(|code| {
            if code == SYNC_NOT_ATTACHED_CODE {
                sync_stop_failed(SYNC_NOT_ATTACHED_CODE, SYNC_NOT_ATTACHED_DESCRIPTION)
            } else {
                sync_stop_failed(SYNC_STOP_FAILED_CODE, SYNC_STOP_FAILED_DESCRIPTION)
            }
        })?;
        let client = self.retained_client().map_err(|_| {
            sync_stop_failed(CLIENT_PAUSE_FAILED_CODE, CLIENT_PAUSE_FAILED_DESCRIPTION)
        })?;
        client.pause().await.map_err(|_| {
            sync_stop_failed(CLIENT_PAUSE_FAILED_CODE, CLIENT_PAUSE_FAILED_DESCRIPTION)
        })?;
        Ok(sync_stop_dto_from_snapshot(snapshot))
    }

    fn spawn_room_list_live(&self) {
        let Some(owner) = self.core.attached_sync_owner() else {
            return;
        };
        let queue = Arc::clone(&self.room_list_updates);
        let emit = Arc::new(move |update: NativeRoomListUpdateSignal| {
            push_room_list_update(&queue, update.session_generation);
        });
        let live = NativeRoomListOwner::start(&owner, emit);
        if let Ok(mut guard) = self.room_list_live.lock() {
            *guard = Some(live);
        }
    }

    /// Drain queued timeline view-delta summaries. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. This is
    /// not Platform::emit. Failed errors stay static.
    pub async fn poll_timeline_view_updates(
        &self,
    ) -> Result<Vec<TimelineViewUpdateDto>, TimelineViewUpdateError> {
        if self.is_nse_read_only() {
            return Err(timeline_view_poll_failed(
                NSE_FORBIDS_POLL_CODE,
                NSE_FORBIDS_POLL_DESCRIPTION,
            ));
        }
        let mut guard = self.timeline_view_updates.lock().map_err(|_| {
            timeline_view_poll_failed(
                TIMELINE_VIEW_POLL_FAILED_CODE,
                TIMELINE_VIEW_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).map(timeline_view_update_dto).collect())
    }

    /// Test-only enqueue onto the attach timeline emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_timeline_view_update_for_test(
        &self,
        stream_id: String,
        room_id: String,
        revision: u64,
    ) {
        use crate::app::timeline::TimelineReadState;
        let batch = TimelineViewDeltaBatch {
            schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
            session_generation: 1,
            stream_id,
            room_id,
            revision,
            ops: Vec::new(),
            read_state: Some(TimelineReadState {
                visible_tail_event_id: None,
                receipt_tail_event_id: None,
                own_read_event_id: None,
                unread_anchor_event_id: None,
                is_marked_unread: false,
            }),
            pagination: None,
            pinned_event_ids: None,
        };
        if let Ok(mut guard) = self.timeline_view_updates.lock() {
            if guard.len() >= TIMELINE_VIEW_UPDATE_QUEUE_CAP {
                guard.remove(0);
            }
            guard.push(batch);
        }
    }

    /// Drain queued owner emit summaries. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. Presence
    /// user ids are never included. This is not Platform::emit.
    pub async fn poll_owner_updates(&self) -> Result<Vec<OwnerUpdateDto>, OwnerUpdateError> {
        if self.is_nse_read_only() {
            return Err(owner_update_poll_failed(
                NSE_FORBIDS_OWNER_POLL_CODE,
                NSE_FORBIDS_OWNER_POLL_DESCRIPTION,
            ));
        }
        let mut guard = self.owner_updates.lock().map_err(|_| {
            owner_update_poll_failed(
                OWNER_UPDATE_POLL_FAILED_CODE,
                OWNER_UPDATE_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).collect())
    }

    /// Test-only enqueue onto the attach owner emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_owner_update_for_test(
        &self,
        family: String,
        session_generation: u64,
        room_id: Option<String>,
    ) {
        push_owner_update(&self.owner_updates, family, session_generation, room_id);
    }

    /// Drain queued room-list wake-ups. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. Room ids
    /// and names are never included. This is not Platform::emit.
    pub async fn poll_room_list_updates(
        &self,
    ) -> Result<Vec<RoomListUpdateDto>, RoomListUpdateError> {
        if self.is_nse_read_only() {
            return Err(room_list_update_poll_failed(
                NSE_FORBIDS_ROOM_LIST_POLL_CODE,
                NSE_FORBIDS_ROOM_LIST_POLL_DESCRIPTION,
            ));
        }
        let mut guard = self.room_list_updates.lock().map_err(|_| {
            room_list_update_poll_failed(
                ROOM_LIST_UPDATE_POLL_FAILED_CODE,
                ROOM_LIST_UPDATE_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).collect())
    }

    /// Test-only enqueue onto the room-list emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_room_list_update_for_test(&self, session_generation: u64) {
        push_room_list_update(&self.room_list_updates, session_generation);
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
                    last_message_preview: room.last_message_preview,
                    is_encrypted: room.encryption_status.is_encrypted(),
                    encryption_status: room.encryption_status,
                    notification_mode: room.notification_mode.map(|mode| mode.as_str().to_owned()),
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

    pub async fn timeline_snapshot(
        &self,
        stream_id: String,
    ) -> Result<TimelineSnapshotDto, TimelineError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: TIMELINE_SNAPSHOT_COMMAND.to_owned(),
                session_generation: TIMELINE_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "streamId": stream_id }),
            })
            .await
            .map_err(map_timeline_snapshot_core_error)?;
        let snapshot: TimelineViewSnapshot =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_failed(TIMELINE_OPEN_FAILED_CODE, TIMELINE_OPEN_FAILED_DESCRIPTION)
            })?;
        Ok(timeline_snapshot_dto(snapshot))
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

    pub async fn presence_set(
        &self,
        state: String,
        status_msg: Option<String>,
    ) -> Result<PresenceWriteDto, PresenceCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: PRESENCE_SET_COMMAND.to_owned(),
                session_generation: TYPING_PRESENCE_GENERATION,
                request_id: None,
                payload: serde_json::json!({ "state": state, "statusMsg": status_msg }),
            })
            .await
            .map_err(map_presence_set_core_error)?;
        let result: NativePresenceWriteResult =
            serde_json::from_value(response.payload).map_err(|_| {
                presence_failed(PRESENCE_SET_FAILED_CODE, PRESENCE_SET_FAILED_DESCRIPTION)
            })?;
        Ok(PresenceWriteDto {
            status: result.status,
        })
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
                // `verification_list` is the long-lived UI observation path.
                // Preserve the display-only SAS payload here; returning it only
                // from one-shot mutations makes `sas_ready` impossible to render.
                .map(verification_request_dto_with_sas)
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

    /// Password UIAA for a pending device delete. Password is a method argument,
    /// never a Core JSON field.
    pub async fn device_delete_password(
        &self,
        operation_id: u64,
        session_generation: u64,
        password: String,
    ) -> Result<DeviceDeleteDto, DeviceCommandError> {
        let result = self
            .core
            .device_delete_password(operation_id, session_generation, &password)
            .await
            .map_err(|error| {
                map_device_core_error(DEVICE_DELETE_PASSWORD_NO_SESSION_CODE, error)
            })?;
        drop(password);
        Ok(device_delete_dto(result))
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
            .map_err(map_join_rule_core_error)?;
        let snapshot: MatrixRoomJoinRuleSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleSnapshotDto {
            status: snapshot.status,
            room_id: snapshot.room_id,
            session_generation: snapshot.session_generation,
            join_rule: snapshot.join_rule,
        })
    }

    pub async fn room_set_join_rule(
        &self,
        room_id: String,
        join_rule: String,
        allow_room_ids: Option<Vec<String>>,
    ) -> Result<RoomJoinRuleWriteDto, JoinRuleCommandError> {
        let payload = join_rule_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "joinRule": join_rule,
            "allowRoomIds": allow_room_ids,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: JOIN_RULE_SET_COMMAND.to_owned(),
                session_generation: 0,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_join_rule_core_error_with_no_session(JOIN_RULE_SET_NO_SESSION_CODE, error)
            })?;
        let status = response
            .payload
            .get("status")
            .and_then(|value| value.as_str())
            .ok_or_else(|| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleWriteDto {
            status: status.to_owned(),
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
        .and_then(own_profile_write_dto)
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
        .and_then(own_profile_write_dto)
    }

    pub async fn get_own_profile(&self) -> Result<OwnProfileDto, OwnProfileCommandError> {
        self.own_profile_command(
            GET_OWN_PROFILE_COMMAND,
            GET_OWN_PROFILE_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(own_profile_dto)
    }

    pub async fn upload_avatar(
        &self,
        payload: Vec<u8>,
        mime_type: String,
    ) -> Result<OwnProfileUploadDto, OwnProfileCommandError> {
        if mime_type.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
            return Err(own_profile_failed(
                OWN_PROFILE_FAILED_CODE,
                OWN_PROFILE_FAILED_DESCRIPTION,
            ));
        }
        let result = self
            .core
            .upload_avatar(payload, &mime_type)
            .await
            .map_err(|error| map_own_profile_core_error(UPLOAD_AVATAR_NO_SESSION_CODE, error))?;
        Ok(OwnProfileUploadDto { mxc: result.mxc })
    }

    pub async fn upload_content(
        &self,
        payload: Vec<u8>,
        mime_type: String,
        filename: Option<String>,
    ) -> Result<MediaUploadDto, MediaUploadError> {
        upload_content_reject_oversize(mime_type.len())?;
        if let Some(filename) = filename.as_ref() {
            upload_content_reject_oversize(filename.len())?;
        }
        let result = self
            .core
            .upload_content(payload, &mime_type, filename.as_deref())
            .await
            .map_err(|error| {
                map_upload_content_core_error(UPLOAD_CONTENT_NO_SESSION_CODE, error)
            })?;
        Ok(MediaUploadDto { mxc: result.mxc })
    }

    #[allow(clippy::too_many_arguments)] // Stable UniFFI fields remain explicit for compatibility.
    pub async fn send_room_attachment(
        &self,
        room_id: String,
        filename: String,
        mime_type: String,
        payload: Vec<u8>,
        caption: Option<String>,
        formatted_caption: Option<String>,
        reply_to: Option<String>,
        thread_root: Option<String>,
        transaction_id: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
    ) -> Result<SendRoomAttachmentDto, SendRoomAttachmentError> {
        send_room_attachment_reject_oversize(room_id.len())?;
        send_room_attachment_reject_oversize(filename.len())?;
        send_room_attachment_reject_oversize(mime_type.len())?;
        if let Some(caption) = caption.as_ref() {
            send_room_attachment_reject_oversize(caption.len())?;
        }
        if let Some(formatted_caption) = formatted_caption.as_ref() {
            send_room_attachment_reject_oversize(formatted_caption.len())?;
        }
        if let Some(reply_to) = reply_to.as_ref() {
            send_room_attachment_reject_oversize(reply_to.len())?;
        }
        if let Some(thread_root) = thread_root.as_ref() {
            send_room_attachment_reject_oversize(thread_root.len())?;
        }
        if let Some(transaction_id) = transaction_id.as_ref() {
            send_room_attachment_reject_oversize(transaction_id.len())?;
        }
        if let Some(mention_user_ids) = mention_user_ids.as_ref() {
            for user_id in mention_user_ids {
                send_room_attachment_reject_oversize(user_id.len())?;
            }
        }
        let result = self
            .core
            .send_room_attachment(crate::app::send::SendRoomAttachmentRequest {
                room_id,
                filename,
                mime_type,
                payload,
                caption,
                formatted_caption,
                reply_to,
                thread_root,
                transaction_id,
                mention_user_ids,
                mention_room: mention_room.unwrap_or(false),
            })
            .await
            .map_err(|error| {
                map_send_room_attachment_core_error(SEND_ROOM_ATTACHMENT_NO_SESSION_CODE, error)
            })?;
        Ok(SendRoomAttachmentDto {
            event_id: result.event_id,
            status: result.status.to_owned(),
        })
    }

    pub async fn download_plain_media(
        &self,
        content_uri: String,
    ) -> Result<MediaBytesDto, PlainMediaError> {
        plain_media_reject_oversize(content_uri.len())?;
        let payload = self
            .core
            .download_plain_media(&content_uri)
            .await
            .map_err(|error| {
                map_plain_media_core_error(DOWNLOAD_PLAIN_MEDIA_NO_SESSION_CODE, error)
            })?;
        Ok(MediaBytesDto { payload })
    }

    pub async fn thumbnail_plain_media(
        &self,
        content_uri: String,
        width: u64,
        height: u64,
    ) -> Result<MediaBytesDto, PlainMediaError> {
        plain_media_reject_oversize(content_uri.len())?;
        let payload = self
            .core
            .thumbnail_plain_media(&content_uri, width, height)
            .await
            .map_err(|error| {
                map_plain_media_core_error(THUMBNAIL_PLAIN_MEDIA_NO_SESSION_CODE, error)
            })?;
        Ok(MediaBytesDto { payload })
    }

    pub async fn register_http_pusher(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        lang: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        http_pusher_reject_oversize(gateway_url.len())?;
        http_pusher_reject_oversize(app_display_name.len())?;
        http_pusher_reject_oversize(lang.len())?;
        let result = self
            .core
            .register_http_pusher(&push_key, &app_id, &gateway_url, &app_display_name, &lang)
            .await
            .map_err(|error| {
                map_http_pusher_core_error(REGISTER_HTTP_PUSHER_NO_SESSION_CODE, error)
            })?;
        Ok(PusherWriteDto {
            status: result.status.to_owned(),
        })
    }

    /// Capture a pusher owner bound to the exact retained Matrix client.
    /// Identity inputs are used only for fail-closed owner selection and are
    /// never returned or included in errors.
    pub fn bind_http_pusher_owner(
        &self,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<Arc<HttpPusherOwner>, PusherCommandError> {
        http_pusher_reject_oversize(user_id.len())?;
        http_pusher_reject_oversize(device_id.len())?;
        http_pusher_reject_oversize(homeserver_url.len())?;
        let owner = self
            .core
            .http_pusher_owner()
            .map_err(|error| map_http_pusher_core_error(BIND_HTTP_PUSHER_NO_SESSION_CODE, error))?;
        if !owner.owns_session(&user_id, &device_id, &homeserver_url) {
            return Err(http_pusher_failed(
                HTTP_PUSHER_SESSION_MISMATCH_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            ));
        }
        Ok(Arc::new(HttpPusherOwner { owner }))
    }

    /// Test-only attach of the production HTTP-pusher owner from the retained
    /// Matrix client, without starting unrelated account/device owners.
    /// Not exported through UniFFI.
    #[doc(hidden)]
    pub fn attach_http_pusher_owner_for_test(&self) -> Result<(), PusherCommandError> {
        let client = {
            let guard = self.restored_client.lock().map_err(|_| {
                http_pusher_failed(
                    BIND_HTTP_PUSHER_NO_SESSION_CODE,
                    HTTP_PUSHER_OWNER_DESCRIPTION,
                )
            })?;
            match &*guard {
                RestoredClientSlot::Ready(client) => client.clone(),
                RestoredClientSlot::Empty | RestoredClientSlot::InFlight => {
                    return Err(http_pusher_failed(
                        BIND_HTTP_PUSHER_NO_SESSION_CODE,
                        HTTP_PUSHER_OWNER_DESCRIPTION,
                    ));
                }
            }
        };
        let owner = Arc::new(NativeHttpPusherOwner::new(&client).map_err(|_| {
            http_pusher_failed(
                BIND_HTTP_PUSHER_NO_SESSION_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            )
        })?);
        self.core.attach_http_pusher(owner).map_err(|_| {
            http_pusher_failed(
                BIND_HTTP_PUSHER_NO_SESSION_CODE,
                HTTP_PUSHER_OWNER_DESCRIPTION,
            )
        })
    }

    pub async fn delete_http_pusher(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        let result = self
            .core
            .delete_http_pusher(&push_key, &app_id)
            .await
            .map_err(|error| {
                map_http_pusher_core_error(DELETE_HTTP_PUSHER_NO_SESSION_CODE, error)
            })?;
        Ok(PusherWriteDto {
            status: result.status.to_owned(),
        })
    }

    /// Restore encryption backup. Recovery secret is a dedicated FFI argument,
    /// never a Core JSON field. Leftover `recover` remains fail-closed.
    pub async fn restore_backup(
        &self,
        recovery_secret: String,
    ) -> Result<RestoreBackupDto, RestoreBackupError> {
        let recovery_secret = Zeroizing::new(recovery_secret);
        restore_backup_reject_oversize(recovery_secret.len())?;
        let result = self
            .core
            .restore_backup(recovery_secret.as_str())
            .await
            .map_err(|error| {
                map_restore_backup_core_error(RESTORE_BACKUP_NO_SESSION_CODE, error)
            })?;
        Ok(RestoreBackupDto {
            status: result.status.to_owned(),
        })
    }

    pub async fn ignored_users_snapshot(
        &self,
    ) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
        self.ignored_users_command(
            IGNORED_USERS_SNAPSHOT_COMMAND,
            IGNORED_USERS_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(ignored_users_snapshot_dto)
    }

    pub async fn ignored_users_ignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        let payload = ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_IGNORE_COMMAND,
            IGNORED_USERS_IGNORE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn ignored_users_unignore(
        &self,
        user_id: String,
    ) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
        let payload = ignored_users_envelope_payload(serde_json::json!({ "userId": user_id }))?;
        self.ignored_users_command(
            IGNORED_USERS_UNIGNORE_COMMAND,
            IGNORED_USERS_UNIGNORE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(ignored_users_write_dto)
    }

    pub async fn user_directory_search(
        &self,
        term: String,
        limit: Option<u64>,
    ) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
        let payload = user_directory_search_envelope_payload(serde_json::json!({
            "term": term,
            "limit": limit,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: USER_DIRECTORY_SEARCH_COMMAND.to_owned(),
                session_generation: USER_DIRECTORY_SEARCH_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_user_directory_search_core_error(USER_DIRECTORY_SEARCH_NO_SESSION_CODE, error)
            })?;
        user_directory_search_dto(response.payload)
    }

    pub async fn message_search(
        &self,
        term: String,
        next_token: Option<String>,
        rooms: Option<Vec<String>>,
        senders: Option<Vec<String>>,
        order: Option<String>,
    ) -> Result<MessageSearchDto, MessageSearchError> {
        let payload = message_search_envelope_payload(serde_json::json!({
            "term": term,
            "nextToken": next_token,
            "rooms": rooms,
            "senders": senders,
            "order": order,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: MESSAGE_SEARCH_COMMAND.to_owned(),
                session_generation: MESSAGE_SEARCH_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_message_search_core_error(MESSAGE_SEARCH_NO_SESSION_CODE, error)
            })?;
        message_search_dto(response.payload)
    }

    pub async fn push_rules_snapshot(&self) -> Result<PushRulesSnapshotDto, PushRulesCommandError> {
        self.push_rules_command(
            PUSH_RULES_SNAPSHOT_COMMAND,
            PUSH_RULES_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(push_rules_snapshot_dto)
    }

    pub async fn push_rules_set_default(
        &self,
        encrypted: bool,
        one_to_one: bool,
        mode: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        let payload = push_rules_envelope_payload(serde_json::json!({
            "encrypted": encrypted,
            "oneToOne": one_to_one,
            "mode": mode,
        }))?;
        self.push_rules_command(
            PUSH_RULES_SET_DEFAULT_COMMAND,
            PUSH_RULES_SET_DEFAULT_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_set_mention(
        &self,
        rule_id: String,
        enabled: bool,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        let payload = push_rules_envelope_payload(serde_json::json!({
            "ruleId": rule_id,
            "enabled": enabled,
        }))?;
        self.push_rules_command(
            PUSH_RULES_SET_MENTION_COMMAND,
            PUSH_RULES_SET_MENTION_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_add_keyword(
        &self,
        keyword: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        let payload = push_rules_envelope_payload(serde_json::json!({ "keyword": keyword }))?;
        self.push_rules_command(
            PUSH_RULES_ADD_KEYWORD_COMMAND,
            PUSH_RULES_ADD_KEYWORD_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn push_rules_remove_keyword(
        &self,
        keyword: String,
    ) -> Result<PushRulesWriteDto, PushRulesCommandError> {
        let payload = push_rules_envelope_payload(serde_json::json!({ "keyword": keyword }))?;
        self.push_rules_command(
            PUSH_RULES_REMOVE_KEYWORD_COMMAND,
            PUSH_RULES_REMOVE_KEYWORD_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(push_rules_write_dto)
    }

    pub async fn room_notification_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomNotificationSnapshotDto, RoomNotificationCommandError> {
        let payload = room_notification_envelope_payload(serde_json::json!({ "roomId": room_id }))?;
        self.room_notification_command(
            ROOM_NOTIFICATION_SNAPSHOT_COMMAND,
            ROOM_NOTIFICATION_SNAPSHOT_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(room_notification_snapshot_dto)
    }

    pub async fn room_notification_set(
        &self,
        room_id: String,
        mode: String,
    ) -> Result<RoomNotificationWriteDto, RoomNotificationCommandError> {
        let payload = room_notification_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "mode": mode,
        }))?;
        self.room_notification_command(
            ROOM_NOTIFICATION_SET_COMMAND,
            ROOM_NOTIFICATION_SET_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(room_notification_write_dto)
    }

    pub async fn room_notifications_snapshot(
        &self,
    ) -> Result<RoomNotificationsSnapshotDto, RoomNotificationCommandError> {
        self.room_notification_command(
            ROOM_NOTIFICATIONS_SNAPSHOT_COMMAND,
            ROOM_NOTIFICATIONS_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(room_notifications_snapshot_dto)
    }

    pub async fn threepid_snapshot(&self) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_SNAPSHOT_COMMAND,
            THREEPID_SNAPSHOT_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(threepid_snapshot_dto)
    }

    pub async fn threepid_delete(
        &self,
        address: String,
    ) -> Result<ThreepidWriteDto, ThreepidCommandError> {
        let payload = threepid_envelope_payload(serde_json::json!({ "address": address }))?;
        self.threepid_command(
            THREEPID_DELETE_COMMAND,
            THREEPID_DELETE_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(threepid_write_dto)
    }

    pub async fn threepid_request_email_token(
        &self,
        email: String,
    ) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
        let payload = threepid_envelope_payload(serde_json::json!({ "email": email }))?;
        self.threepid_command(
            THREEPID_REQUEST_EMAIL_TOKEN_COMMAND,
            THREEPID_REQUEST_EMAIL_TOKEN_NO_SESSION_CODE,
            payload,
        )
        .await
        .and_then(threepid_email_token_dto)
    }

    pub async fn threepid_add_email(&self) -> Result<ThreepidAddDto, ThreepidCommandError> {
        self.threepid_command(
            THREEPID_ADD_EMAIL_COMMAND,
            THREEPID_ADD_EMAIL_NO_SESSION_CODE,
            serde_json::Value::Null,
        )
        .await
        .and_then(threepid_add_dto)
    }

    pub async fn threepid_add_email_password(
        &self,
        password: String,
    ) -> Result<ThreepidAddDto, ThreepidCommandError> {
        if password.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
            return Err(threepid_failed(
                THREEPID_FAILED_CODE,
                THREEPID_FAILED_DESCRIPTION,
            ));
        }
        let result = self
            .core
            .threepid_add_email_password(&password)
            .await
            .map_err(|error| {
                map_threepid_core_error(THREEPID_ADD_EMAIL_PASSWORD_NO_SESSION_CODE, error)
            })?;
        drop(password);
        Ok(ThreepidAddDto {
            status: result.status,
        })
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

    #[allow(clippy::too_many_arguments)] // UniFFI preserves the typed Matrix directory query fields.
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

    pub async fn room_set_favorite(
        &self,
        room_id: String,
        favorite: bool,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "favorite": favorite,
        }))?;
        self.room_membership_command(
            ROOM_SET_FAVORITE_COMMAND,
            ROOM_SET_FAVORITE_NO_SESSION_CODE,
            payload,
        )
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

    pub async fn timeline_event_readback(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelineEventReadbackDto, TimelineReadStateError> {
        let payload = timeline_read_state_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_EVENT_READBACK_COMMAND,
                TIMELINE_EVENT_READBACK_NO_SESSION_CODE,
                payload,
            )
            .await?;
        let readback: NativeTimelineEventReadback =
            serde_json::from_value(response).map_err(|_| {
                timeline_read_state_failed(
                    TIMELINE_READ_STATE_FAILED_CODE,
                    TIMELINE_READ_STATE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TimelineEventReadbackDto {
            session_generation: readback.session_generation,
            room_id: readback.room_id,
            event_id: readback.event_id,
            item: timeline_event_item_dto(readback.item),
        })
    }

    pub async fn timeline_set_read_state(
        &self,
        stream_id: String,
        action: String,
        intent: String,
        observed_live_tail_event_id: Option<String>,
    ) -> Result<TimelineReadStateDto, TimelineReadStateError> {
        let action = read_action_from_str(&action)?;
        let intent = read_intent_from_str(&intent)?;
        let payload = timeline_read_state_envelope_payload(serde_json::json!({
            "streamId": stream_id,
            "action": read_action_as_str(action),
            "intent": read_intent_as_str(intent),
            "observedLiveTailEventId": observed_live_tail_event_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_SET_READ_STATE_COMMAND,
                TIMELINE_SET_READ_STATE_NO_SESSION_CODE,
                payload,
            )
            .await?;
        let readback: NativeTimelineReadStateReadback =
            serde_json::from_value(response).map_err(|_| {
                timeline_read_state_failed(
                    TIMELINE_READ_STATE_FAILED_CODE,
                    TIMELINE_READ_STATE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TimelineReadStateDto {
            action: read_action_as_str(readback.action).to_owned(),
            receipt_sent: readback.receipt_sent,
            acknowledged_event_id: readback.acknowledged_event_id,
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn timeline_jump_latest(
        &self,
        stream_id: String,
    ) -> Result<TimelineOpenDto, TimelineReadStateError> {
        let payload = timeline_read_state_envelope_payload(serde_json::json!({
            "streamId": stream_id,
        }))?;
        let response = self
            .timeline_read_state_command(
                TIMELINE_JUMP_LATEST_COMMAND,
                TIMELINE_JUMP_LATEST_NO_SESSION_CODE,
                payload,
            )
            .await?;
        let readback: NativeTimelineOpenReadback =
            serde_json::from_value(response).map_err(|_| {
                timeline_read_state_failed(
                    TIMELINE_READ_STATE_FAILED_CODE,
                    TIMELINE_READ_STATE_FAILED_DESCRIPTION,
                )
            })?;
        Ok(TimelineOpenDto {
            schema_version: readback.schema_version,
            stream_id: readback.stream_id,
            position: view_position_dto(readback.position),
            snapshot: timeline_snapshot_dto(readback.snapshot),
        })
    }

    pub async fn reaction_ensure(
        &self,
        room_id: String,
        event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            REACTION_ENSURE_COMMAND,
            REACTION_ENSURE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn agent_approval_decide(
        &self,
        room_id: String,
        event_id: String,
        action_id: String,
    ) -> Result<AgentApprovalDecisionDto, TimelineReactionError> {
        let payload = timeline_reaction_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
            "actionId": action_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: AGENT_APPROVAL_DECIDE_COMMAND.to_owned(),
                session_generation: TIMELINE_REACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_timeline_reaction_core_error("agent-approval-no-session", error)
            })?;
        let result: NativeAgentApprovalDecisionResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_reaction_failed(
                    TIMELINE_REACTION_FAILED_CODE,
                    TIMELINE_REACTION_FAILED_DESCRIPTION,
                )
            })?;
        Ok(AgentApprovalDecisionDto {
            room_id: result.room_id,
            event_id: result.event_id,
            status: match result.status {
                crate::app::agent_approvals::AgentApprovalDecisionStatus::Applied => "applied",
                crate::app::agent_approvals::AgentApprovalDecisionStatus::AlreadyDecided => {
                    "already_decided"
                }
            }
            .to_owned(),
            reaction: result.reaction.map(timeline_reaction_mutation_dto),
        })
    }

    pub async fn reaction_redact(
        &self,
        room_id: String,
        target_event_id: String,
        reaction_event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            REACTION_REDACT_COMMAND,
            REACTION_REDACT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "targetEventId": target_event_id,
                "reactionEventId": reaction_event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn timeline_reaction_toggle(
        &self,
        room_id: String,
        event_id: String,
        key: String,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        self.timeline_reaction_command(
            TIMELINE_REACTION_TOGGLE_COMMAND,
            TIMELINE_REACTION_TOGGLE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "key": key,
            }),
        )
        .await
    }

    pub async fn composer_set_reply_draft(
        &self,
        room_id: String,
        event_id: String,
        start_thread: bool,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        self.composer_reply_draft_command(
            COMPOSER_SET_REPLY_DRAFT_COMMAND,
            COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "startThread": start_thread,
            }),
        )
        .await
    }

    pub async fn composer_get_reply_draft(
        &self,
        room_id: String,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        self.composer_reply_draft_command(
            COMPOSER_GET_REPLY_DRAFT_COMMAND,
            COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
            }),
        )
        .await
    }

    pub async fn composer_clear_reply_draft(
        &self,
        room_id: String,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        // This compatibility wrapper predates Core-issued draft revisions.
        // Snapshot the current revision, then let the Core owner perform the
        // atomic comparison. A selection made between these commands is
        // intentionally preserved rather than cleared by the older caller.
        let current = self
            .composer_reply_draft_command_wire(
                COMPOSER_GET_REPLY_DRAFT_COMMAND,
                COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE,
                serde_json::json!({
                    "roomId": room_id,
                }),
            )
            .await?;
        let expected_draft_revision = current
            .draft
            .as_ref()
            .map_or(0, |draft| draft.draft_revision);
        self.composer_reply_draft_command(
            COMPOSER_CLEAR_REPLY_DRAFT_COMMAND,
            COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "expectedDraftRevision": expected_draft_revision,
            }),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn send_text(
        &self,
        room_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        reply_to: Option<String>,
        thread_root: Option<String>,
        txn_id: Option<String>,
    ) -> Result<SendTextDto, SendTextError> {
        self.send_text_command(serde_json::json!({
            "roomId": room_id,
            "body": body,
            "msgType": msg_type,
            "formattedBody": formatted_body,
            "mentionUserIds": mention_user_ids,
            "mentionRoom": mention_room,
            "replyTo": reply_to,
            "threadRoot": thread_root,
            "txnId": txn_id,
        }))
        .await
    }

    pub async fn send_poll(
        &self,
        room_id: String,
        question: String,
        answers: Vec<String>,
        max_selections: u32,
        thread_root: Option<String>,
        reply_to: Option<String>,
    ) -> Result<SendPollDto, SendPollError> {
        self.send_poll_command(serde_json::json!({
            "roomId": room_id,
            "question": question,
            "answers": answers,
            "maxSelections": max_selections,
            "threadRoot": thread_root,
            "replyTo": reply_to,
        }))
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn edit_message(
        &self,
        room_id: String,
        event_id: String,
        body: String,
        msg_type: Option<String>,
        formatted_body: Option<String>,
        mention_user_ids: Option<Vec<String>>,
        mention_room: Option<bool>,
        txn_id: Option<String>,
    ) -> Result<EditMessageDto, EditMessageError> {
        self.edit_message_command(serde_json::json!({
            "roomId": room_id,
            "eventId": event_id,
            "body": body,
            "msgType": msg_type,
            "formattedBody": formatted_body,
            "mentionUserIds": mention_user_ids,
            "mentionRoom": mention_room,
            "txnId": txn_id,
        }))
        .await
    }

    pub async fn poll_respond(
        &self,
        room_id: String,
        poll_event_id: String,
        answer_ids: Vec<String>,
    ) -> Result<PollRespondDto, PollRespondError> {
        self.poll_respond_command(serde_json::json!({
            "roomId": room_id,
            "pollEventId": poll_event_id,
            "answerIds": answer_ids,
        }))
        .await
    }

    pub async fn timeline_edit_text(
        &self,
        room_id: String,
        event_id: String,
        body: String,
        formatted_body: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_EDIT_TEXT_COMMAND,
            TIMELINE_EDIT_TEXT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "body": body,
                "formattedBody": formatted_body,
            }),
        )
        .await
    }

    pub async fn timeline_redact(
        &self,
        room_id: String,
        event_id: String,
        reason: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_REDACT_COMMAND,
            TIMELINE_REDACT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "reason": reason,
            }),
        )
        .await
    }

    pub async fn timeline_report(
        &self,
        room_id: String,
        event_id: String,
        reason: Option<String>,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        self.timeline_mutate_command(
            TIMELINE_REPORT_COMMAND,
            TIMELINE_REPORT_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "reason": reason,
            }),
        )
        .await
    }

    pub async fn timeline_pin(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        self.timeline_pin_command(
            TIMELINE_PIN_COMMAND,
            TIMELINE_PIN_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_unpin(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        self.timeline_pin_command(
            TIMELINE_UNPIN_COMMAND,
            TIMELINE_UNPIN_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_poll_vote(
        &self,
        room_id: String,
        event_id: String,
        answer_ids: Vec<String>,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        self.timeline_vote_decline_command(
            TIMELINE_POLL_VOTE_COMMAND,
            TIMELINE_POLL_VOTE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
                "answerIds": answer_ids,
            }),
        )
        .await
    }

    pub async fn timeline_call_decline(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        self.timeline_vote_decline_command(
            TIMELINE_CALL_DECLINE_COMMAND,
            TIMELINE_CALL_DECLINE_NO_SESSION_CODE,
            serde_json::json!({
                "roomId": room_id,
                "eventId": event_id,
            }),
        )
        .await
    }

    pub async fn timeline_forward_text(
        &self,
        source_room_id: String,
        event_id: String,
        target_room_id: String,
        as_quote: bool,
        confirmed_encryption_downgrade: bool,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        self.timeline_forward_command(
            TIMELINE_FORWARD_TEXT_COMMAND,
            TIMELINE_FORWARD_TEXT_NO_SESSION_CODE,
            serde_json::json!({
                "sourceRoomId": source_room_id,
                "eventId": event_id,
                "targetRoomId": target_room_id,
                "asQuote": as_quote,
                "confirmedEncryptionDowngrade": confirmed_encryption_downgrade,
            }),
        )
        .await
    }

    pub async fn timeline_forward_media(
        &self,
        source_room_id: String,
        event_id: String,
        target_room_id: String,
        confirmed_encryption_downgrade: bool,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        self.timeline_forward_command(
            TIMELINE_FORWARD_MEDIA_COMMAND,
            TIMELINE_FORWARD_MEDIA_NO_SESSION_CODE,
            serde_json::json!({
                "sourceRoomId": source_room_id,
                "eventId": event_id,
                "targetRoomId": target_room_id,
                "confirmedEncryptionDowngrade": confirmed_encryption_downgrade,
            }),
        )
        .await
    }

    pub async fn session_snapshot(&self) -> Result<SessionSnapshotDto, SessionStatusError> {
        let payload = self
            .session_status_command(SESSION_SNAPSHOT_COMMAND)
            .await?;
        session_snapshot_dto(payload)
    }

    pub async fn sync_status(&self) -> Result<SyncStatusDto, SessionStatusError> {
        if let Some(owner) = self.core.attached_sync_owner() {
            return sync_status_from_owner_snapshot(owner.observe());
        }
        let payload = self.session_status_command(SYNC_STATUS_COMMAND).await?;
        sync_status_dto(payload)
    }

    pub async fn media_config(&self) -> Result<MediaConfigDto, SessionStatusError> {
        let payload = self.session_status_command(MEDIA_CONFIG_COMMAND).await?;
        media_config_dto(payload)
    }

    pub async fn secret_storage_status(
        &self,
    ) -> Result<SecretStorageStatusDto, SessionStatusError> {
        let payload = self
            .session_status_command(SECRET_STORAGE_STATUS_COMMAND)
            .await?;
        secret_storage_status_dto(payload)
    }

    /// Open the persisted store for NSE preview. Never attaches owners or
    /// starts SyncService. An already-retained planted/restored client is
    /// adopted as read-only; otherwise this restores from the vault.
    pub async fn nse_open_read_only_store(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
    ) -> Result<NseStoreDto, NseStoreError> {
        self.nse_open_read_only_store_with_room(user_id, homeserver_url, store_root, None)
            .await
    }

    async fn nse_open_read_only_store_with_room(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        room_id: Option<matrix_sdk::ruma::OwnedRoomId>,
    ) -> Result<NseStoreDto, NseStoreError> {
        if self.owners_attached() {
            return Err(nse_failed(
                NSE_OWNERS_ATTACHED_CODE,
                NSE_OWNERS_ATTACHED_DESCRIPTION,
            ));
        }
        AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| nse_failed(IDENTITY_INVALID_CODE, IDENTITY_INVALID_DESCRIPTION))?;
        validate_store_root(&store_root).map_err(map_restore_to_nse)?;
        if self.has_retained_client() {
            self.set_nse_read_only(true)?;
            return self.nse_store_dto();
        }
        self.restore_persisted_session_with_policy(
            user_id,
            homeserver_url,
            store_root,
            true,
            room_id.map(RoomLoadSettings::One),
        )
        .await
        .map_err(map_restore_to_nse)?;
        self.set_nse_read_only(true)?;
        self.nse_store_dto()
    }

    pub async fn nse_store_status(&self) -> Result<NseStoreDto, NseStoreError> {
        if !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            ));
        }
        self.nse_store_dto()
    }

    /// Drop the short-lived NSE client while this async call is still executing
    /// on the Rust runtime. SQLite pool cleanup may require that runtime; leaving
    /// the retained client for UniFFI object deallocation can abort the extension.
    pub async fn nse_close_read_only_store(&self) -> Result<(), NseStoreError> {
        if self.owners_attached() || !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_CLOSE_FAILED_CODE,
                NSE_CLOSE_FAILED_DESCRIPTION,
            ));
        }
        let retained = {
            let mut guard = self
                .restored_client
                .lock()
                .map_err(|_| nse_failed(NSE_CLOSE_FAILED_CODE, NSE_CLOSE_FAILED_DESCRIPTION))?;
            std::mem::replace(&mut *guard, RestoredClientSlot::Empty)
        };
        drop(retained);
        self.set_nse_read_only(false)
            .map_err(|_| nse_failed(NSE_CLOSE_FAILED_CODE, NSE_CLOSE_FAILED_DESCRIPTION))?;
        Ok(())
    }

    /// Resolve one push notification with the Matrix SDK's dedicated
    /// multi-process notification client. This may run the SDK's bounded,
    /// short-lived notification/decryption sync, but it never starts the
    /// product `SyncService` or attaches product session owners.
    pub async fn nse_resolve_event_preview(
        &self,
        user_id: String,
        homeserver_url: String,
        store_root: String,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        tokio::time::timeout(NSE_RESOLUTION_TIMEOUT, async {
            if room_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
                return Err(nse_failed(
                    NSE_PAYLOAD_OVERSIZE_CODE,
                    NSE_PAYLOAD_OVERSIZE_DESCRIPTION,
                ));
            }
            let parsed_room =
                matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()).map_err(|_| {
                    nse_failed(
                        NSE_EVENT_NOT_IN_STORE_CODE,
                        NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
                    )
                })?;
            self.nse_open_read_only_store_with_room(
                user_id,
                homeserver_url,
                store_root,
                Some(parsed_room),
            )
            .await?;
            self.nse_event_preview_unbounded(room_id, event_id).await
        })
        .await
        .map_err(|_| {
            nse_failed(
                NSE_RESOLUTION_TIMEOUT_CODE,
                NSE_RESOLUTION_TIMEOUT_DESCRIPTION,
            )
        })?
    }

    pub async fn nse_event_preview(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        tokio::time::timeout(
            NSE_RESOLUTION_TIMEOUT,
            self.nse_event_preview_unbounded(room_id, event_id),
        )
        .await
        .map_err(|_| {
            nse_failed(
                NSE_RESOLUTION_TIMEOUT_CODE,
                NSE_RESOLUTION_TIMEOUT_DESCRIPTION,
            )
        })?
    }

    async fn nse_event_preview_unbounded(
        &self,
        room_id: String,
        event_id: String,
    ) -> Result<NseEventPreviewDto, NseStoreError> {
        if room_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
            || event_id.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
        {
            return Err(nse_failed(
                NSE_PAYLOAD_OVERSIZE_CODE,
                NSE_PAYLOAD_OVERSIZE_DESCRIPTION,
            ));
        }
        if !self.is_nse_read_only() {
            return Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            ));
        }
        let client = self.retained_client()?;
        let Ok(parsed_room) = matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()) else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let Ok(parsed_event) = matrix_sdk::ruma::OwnedEventId::try_from(event_id.trim()) else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let notification_client =
            NotificationClient::new(client, NotificationProcessSetup::MultipleProcesses)
                .await
                .map_err(|_| {
                    nse_failed(
                        NSE_CLIENT_INIT_FAILED_CODE,
                        NSE_CLIENT_INIT_FAILED_DESCRIPTION,
                    )
                })?;
        let status = notification_client
            .get_notification(&parsed_room, &parsed_event)
            .await
            .map_err(|_| {
                nse_failed(
                    NSE_EVENT_FETCH_FAILED_CODE,
                    NSE_EVENT_FETCH_FAILED_DESCRIPTION,
                )
            })?;
        let NotificationStatus::Event(item) = status else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let NotificationEvent::Timeline(event) = &item.event else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(message)) =
            event.as_ref()
        else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let Some(original) = message.as_original() else {
            return Err(nse_failed(
                NSE_EVENT_NOT_IN_STORE_CODE,
                NSE_EVENT_NOT_IN_STORE_DESCRIPTION,
            ));
        };
        let body = Some(bounded_nse_preview_text(original.content.body(), 240));
        let message_type = nse_message_type(&original.content.msgtype)
            .map(|value| bounded_nse_preview_text(value, 64));

        Ok(NseEventPreviewDto {
            event_type: "m.room.message".to_owned(),
            sender_id: Some(bounded_nse_preview_text(
                item.sender_display_name
                    .as_deref()
                    .unwrap_or_else(|| item.event.sender().as_str()),
                255,
            )),
            body,
            message_type,
        })
    }

    pub async fn backup_status(&self) -> Result<BackupStatusDto, LeftoverCommandError> {
        let payload = self.leftover_status_command(BACKUP_STATUS_COMMAND).await?;
        leftover_backup_status_dto(payload)
    }

    pub async fn crypto_status(&self) -> Result<CryptoStatusDto, LeftoverCommandError> {
        let payload = self.leftover_status_command(CRYPTO_STATUS_COMMAND).await?;
        leftover_crypto_status_dto(payload)
    }

    pub async fn cross_signing_status(
        &self,
    ) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(CROSS_SIGNING_STATUS_COMMAND)
            .await?;
        leftover_cross_signing_status_dto(payload)
    }

    pub async fn room_key_transfer_status(
        &self,
    ) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
        let payload = self
            .leftover_status_command(ROOM_KEY_TRANSFER_STATUS_COMMAND)
            .await?;
        leftover_room_key_transfer_status_dto(payload)
    }

    pub async fn wipe_persisted_stores(
        &self,
        store_root: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(store_root.len())?;
        let root = parse_store_root(&store_root).map_err(|_| {
            leftover_failed(
                LEFTOVER_STORE_ROOT_INVALID_CODE,
                LEFTOVER_STORE_ROOT_INVALID_DESCRIPTION,
            )
        })?;
        if root.exists() {
            std::fs::remove_dir_all(root)
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        Ok(LeftoverAckDto {
            status: "wiped".to_owned(),
        })
    }

    /// Best-effort remote revocation on the already-loaded, exact device.
    /// No store restore or new client is permitted on the logout route.
    pub async fn revoke_server_session(
        &self,
        user_id: String,
        device_id: String,
        homeserver_url: String,
    ) -> Result<bool, LeftoverCommandError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        let Ok(client) = self.retained_client() else {
            return Ok(false);
        };
        let matches = client.user_id().map(|id| id.as_str()) == Some(identity.user_id())
            && client.device_id().map(|id| id.as_str()) == Some(device_id.as_str())
            && AccountIdentity::new(&user_id, client.homeserver().as_str())
                .ok()
                .as_ref()
                == Some(&identity);
        if !matches || self.is_nse_read_only() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        Ok(matches!(
            tokio::time::timeout(
                std::time::Duration::from_secs(5),
                client.matrix_auth().logout()
            )
            .await,
            Ok(Ok(_))
        ))
    }

    /// Remove account authentication even when its SDK store cannot restore.
    /// The encrypted history/key are retained for a later password login.
    pub async fn forget_session(
        &self,
        user_id: String,
        homeserver_url: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        let identity = AccountIdentity::new(&user_id, &homeserver_url)
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        if self.is_nse_read_only() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        if let Some(snapshot) = self
            .core
            .session_snapshot()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?
        {
            if AccountIdentity::new(&snapshot.user_id, &snapshot.homeserver_url)
                .ok()
                .as_ref()
                != Some(&identity)
            {
                return Err(leftover_failed(
                    LEFTOVER_FAILED_CODE,
                    LEFTOVER_FAILED_DESCRIPTION,
                ));
            }
        }
        self.logout().await?;
        self.secret_store
            .delete(SessionMaterialId::from_identity(&identity).account())
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        Ok(LeftoverAckDto {
            status: "forgotten".to_owned(),
        })
    }

    pub async fn logout(&self) -> Result<LeftoverAckDto, LeftoverCommandError> {
        // Serialize teardown with foreground resume and release every store
        // before dropping ownership. This operation performs no remote logout.
        let _lifecycle = self.sync_lifecycle.lock().await;
        if self.owners_attached() {
            self.core
                .stop_attached_sync()
                .await
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        if let Ok(client) = self.retained_client() {
            client
                .pause()
                .await
                .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        }
        self.core
            .close()
            .await
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        if let Ok(mut live) = self.room_list_live.lock() {
            *live = None;
        }
        if let Ok(mut updates) = self.timeline_view_updates.lock() {
            updates.clear();
        }
        if let Ok(mut updates) = self.room_list_updates.lock() {
            updates.clear();
        }
        if let Ok(mut updates) = self.owner_updates.lock() {
            updates.clear();
        }
        let mut guard = self
            .restored_client
            .lock()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        *guard = RestoredClientSlot::Empty;
        drop(guard);
        let mut attach = self
            .owner_attach
            .lock()
            .map_err(|_| leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION))?;
        *attach = OwnerAttachSlot::Empty;
        Ok(LeftoverAckDto {
            status: "logged_out".to_owned(),
        })
    }

    pub async fn recover(
        &self,
        recovery_key: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        let recovery_key = Zeroizing::new(recovery_key);
        leftover_reject_oversize(recovery_key.len())?;
        if recovery_key.is_empty() {
            return Err(leftover_failed(
                LEFTOVER_FAILED_CODE,
                LEFTOVER_FAILED_DESCRIPTION,
            ));
        }
        // Recovery requires live secret-storage I/O. Planted tests stay
        // fail-closed and never echo the recovery key.
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn send_agent_approval(
        &self,
        room_id: String,
        action_id: String,
        action_title: String,
        decision: String,
        source_event_id: Option<String>,
        created_at: u64,
    ) -> Result<AgentApprovalSendDto, AgentApprovalSendError> {
        let size = room_id.len()
            + action_id.len()
            + action_title.len()
            + decision.len()
            + source_event_id.as_deref().map(str::len).unwrap_or_default();
        if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES
            || action_id.trim().is_empty()
            || action_title.trim().is_empty()
            || !matches!(decision.as_str(), "approve" | "reject")
        {
            return Err(agent_approval_failed(
                AGENT_APPROVAL_INVALID_CODE,
                AGENT_APPROVAL_INVALID_DESCRIPTION,
            ));
        }
        let room_id = matrix_sdk::ruma::OwnedRoomId::try_from(room_id.trim()).map_err(|_| {
            agent_approval_failed(
                AGENT_APPROVAL_INVALID_CODE,
                AGENT_APPROVAL_INVALID_DESCRIPTION,
            )
        })?;
        if let Some(event_id) = source_event_id.as_deref() {
            matrix_sdk::ruma::OwnedEventId::try_from(event_id.trim()).map_err(|_| {
                agent_approval_failed(
                    AGENT_APPROVAL_INVALID_CODE,
                    AGENT_APPROVAL_INVALID_DESCRIPTION,
                )
            })?;
        }
        let client = self.retained_client().map_err(|_| {
            agent_approval_failed(
                AGENT_APPROVAL_NO_SESSION_CODE,
                AGENT_APPROVAL_NO_SESSION_DESCRIPTION,
            )
        })?;
        let room = client.get_room(&room_id).ok_or_else(|| {
            agent_approval_failed(
                AGENT_APPROVAL_FAILED_CODE,
                AGENT_APPROVAL_FAILED_DESCRIPTION,
            )
        })?;
        let content = serde_json::json!({
            "msgtype": "m.notice",
            "body": format!("{} agent action: {}", if decision == "approve" { "Approved" } else { "Rejected" }, action_title),
            "in.synara.agent.action": {
                "version": 1,
                "action_id": action_id,
                "action_title": action_title,
                "decision": decision,
                "source_event_id": source_event_id,
                "created_at": created_at,
            }
        });
        let response = room
            .send_raw("m.room.message", content)
            .await
            .map_err(|_| {
                agent_approval_failed(
                    AGENT_APPROVAL_FAILED_CODE,
                    AGENT_APPROVAL_FAILED_DESCRIPTION,
                )
            })?;
        Ok(AgentApprovalSendDto {
            event_id: response.response.event_id.to_string(),
            status: "sent".to_owned(),
        })
    }

    pub async fn set_notification_mode(
        &self,
        room_id: String,
        mode: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(room_id.len() + mode.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    /// Download bytes for an opaque timeline media handle.
    ///
    /// Dedicated UniFFI bytes, not a `Core.command` envelope. NSE cannot
    /// download. Unknown handles and missing owners fail closed without
    /// echoing the handle, mxc, or tokens.
    pub async fn timeline_media_bytes(
        &self,
        handle_id: String,
    ) -> Result<LeftoverBytesDto, TimelineMediaError> {
        if self.is_nse_read_only() {
            return Err(timeline_media_failed(
                NSE_FORBIDS_MEDIA_CODE,
                NSE_FORBIDS_MEDIA_DESCRIPTION,
            ));
        }
        let Some(owner) = self.core.attached_timeline_owner() else {
            return Err(timeline_media_failed(
                TIMELINE_MEDIA_NO_SESSION_CODE,
                TIMELINE_MEDIA_NO_SESSION_DESCRIPTION,
            ));
        };
        match owner.media_bytes(&handle_id).await {
            Ok(payload) => Ok(LeftoverBytesDto { payload }),
            Err("p4-s33-media-unknown-handle") => Err(timeline_media_failed(
                TIMELINE_MEDIA_UNKNOWN_HANDLE_CODE,
                TIMELINE_MEDIA_UNKNOWN_HANDLE_DESCRIPTION,
            )),
            Err("p4-s33-media-too-large") => Err(timeline_media_failed(
                TIMELINE_MEDIA_TOO_LARGE_CODE,
                TIMELINE_MEDIA_TOO_LARGE_DESCRIPTION,
            )),
            Err(_) => Err(timeline_media_failed(
                TIMELINE_MEDIA_FAILED_CODE,
                TIMELINE_MEDIA_FAILED_DESCRIPTION,
            )),
        }
    }

    pub async fn media_download(
        &self,
        mxc: String,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(mxc.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn media_thumbnail(
        &self,
        mxc: String,
        width: u64,
        height: u64,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(mxc.len())?;
        let _ = (width, height);
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn media_upload(
        &self,
        payload: Vec<u8>,
        mime_type: String,
        filename: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(payload.len() + mime_type.len() + filename.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn room_avatar_bytes(
        &self,
        room_id: String,
    ) -> Result<LeftoverBytesDto, LeftoverCommandError> {
        leftover_reject_oversize(room_id.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn pusher_set(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        device_display_name: String,
        lang: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(
            push_key.len()
                + app_id.len()
                + gateway_url.len()
                + app_display_name.len()
                + device_display_name.len()
                + lang.len(),
        )?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    pub async fn pusher_delete(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<LeftoverAckDto, LeftoverCommandError> {
        leftover_reject_oversize(push_key.len() + app_id.len())?;
        if !self.has_retained_client() {
            return Err(leftover_failed(
                LEFTOVER_NO_SESSION_CODE,
                LEFTOVER_NO_SESSION_DESCRIPTION,
            ));
        }
        Err(leftover_failed(
            LEFTOVER_UNAVAILABLE_CODE,
            LEFTOVER_UNAVAILABLE_DESCRIPTION,
        ))
    }

    async fn leftover_status_command(
        &self,
        command: &'static str,
    ) -> Result<serde_json::Value, LeftoverCommandError> {
        let payload = leftover_status_envelope_payload(serde_json::Value::Null)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: LEFTOVER_STATUS_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(map_leftover_status_core_error)?;
        Ok(response.payload)
    }

    fn is_nse_read_only(&self) -> bool {
        self.nse_read_only
            .lock()
            .map(|guard| *guard)
            .unwrap_or(false)
    }

    fn set_nse_read_only(&self, value: bool) -> Result<(), NseStoreError> {
        let mut guard = self
            .nse_read_only
            .lock()
            .map_err(|_| nse_failed(NSE_FAILED_CODE, NSE_FAILED_DESCRIPTION))?;
        *guard = value;
        Ok(())
    }

    fn has_retained_client(&self) -> bool {
        self.restored_client
            .lock()
            .map(|guard| matches!(*guard, RestoredClientSlot::Ready(_)))
            .unwrap_or(false)
    }

    fn owners_attached(&self) -> bool {
        self.owner_attach
            .lock()
            .map(|guard| matches!(*guard, OwnerAttachSlot::Ready))
            .unwrap_or(false)
    }

    fn retained_client(&self) -> Result<Client, NseStoreError> {
        let guard = self
            .restored_client
            .lock()
            .map_err(|_| nse_failed(NSE_FAILED_CODE, NSE_FAILED_DESCRIPTION))?;
        match &*guard {
            RestoredClientSlot::Ready(client) => Ok(client.clone()),
            RestoredClientSlot::Empty | RestoredClientSlot::InFlight => Err(nse_failed(
                NSE_STORE_NOT_OPEN_CODE,
                NSE_STORE_NOT_OPEN_DESCRIPTION,
            )),
        }
    }

    fn nse_store_dto(&self) -> Result<NseStoreDto, NseStoreError> {
        Ok(NseStoreDto {
            read_only: self.is_nse_read_only(),
            owners_attached: self.owners_attached(),
            sync_started: self.core.sync_service_started(),
        })
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

    async fn timeline_read_state_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, TimelineReadStateError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_READ_STATE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_read_state_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn timeline_reaction_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineReactionMutationDto, TimelineReactionError> {
        let payload = timeline_reaction_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_REACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_reaction_core_error(no_session, error))?;
        let result: NativeReactionMutationResult = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_reaction_failed(
                    TIMELINE_REACTION_FAILED_CODE,
                    TIMELINE_REACTION_FAILED_DESCRIPTION,
                )
            })?;
        Ok(timeline_reaction_mutation_dto(result))
    }

    async fn composer_reply_draft_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<ComposerReplyDraftDto, ComposerReplyDraftError> {
        let readback = self
            .composer_reply_draft_command_wire(command, no_session, payload)
            .await?;
        Ok(composer_reply_draft_dto(readback))
    }

    async fn composer_reply_draft_command_wire(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<ComposerReplyDraftReadbackWire, ComposerReplyDraftError> {
        let payload = composer_reply_draft_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: COMPOSER_REPLY_DRAFT_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_composer_reply_draft_core_error(no_session, error))?;
        serde_json::from_value(response.payload).map_err(|_| {
            composer_reply_draft_failed(
                COMPOSER_REPLY_DRAFT_FAILED_CODE,
                COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
            )
        })
    }

    async fn send_text_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<SendTextDto, SendTextError> {
        let payload = send_text_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SEND_TEXT_COMMAND.to_owned(),
                session_generation: SEND_TEXT_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_send_text_core_error(SEND_TEXT_NO_SESSION_CODE, error))?;
        let result: SendTextResultWire = serde_json::from_value(response.payload)
            .map_err(|_| send_text_failed(SEND_TEXT_FAILED_CODE, SEND_TEXT_FAILED_DESCRIPTION))?;
        Ok(send_text_dto(result))
    }

    async fn send_poll_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<SendPollDto, SendPollError> {
        let payload = send_poll_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: SEND_POLL_COMMAND.to_owned(),
                session_generation: SEND_POLL_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_send_poll_core_error(SEND_POLL_NO_SESSION_CODE, error))?;
        let result: SendPollResultWire = serde_json::from_value(response.payload)
            .map_err(|_| send_poll_failed(SEND_POLL_FAILED_CODE, SEND_POLL_FAILED_DESCRIPTION))?;
        Ok(send_poll_dto(result))
    }

    async fn edit_message_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<EditMessageDto, EditMessageError> {
        let payload = edit_message_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: EDIT_MESSAGE_COMMAND.to_owned(),
                session_generation: EDIT_MESSAGE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_edit_message_core_error(EDIT_MESSAGE_NO_SESSION_CODE, error))?;
        let result: EditMessageResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                edit_message_failed(EDIT_MESSAGE_FAILED_CODE, EDIT_MESSAGE_FAILED_DESCRIPTION)
            })?;
        Ok(edit_message_dto(result))
    }

    async fn poll_respond_command(
        &self,
        payload: serde_json::Value,
    ) -> Result<PollRespondDto, PollRespondError> {
        let payload = poll_respond_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: POLL_RESPOND_COMMAND.to_owned(),
                session_generation: POLL_RESPOND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_poll_respond_core_error(POLL_RESPOND_NO_SESSION_CODE, error))?;
        let result: PollRespondResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                poll_respond_failed(POLL_RESPOND_FAILED_CODE, POLL_RESPOND_FAILED_DESCRIPTION)
            })?;
        Ok(poll_respond_dto(result))
    }

    async fn timeline_mutate_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineMutateDto, TimelineMutateError> {
        let payload = timeline_mutate_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_MUTATE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_mutate_core_error(no_session, error))?;
        let result: TimelineMutateResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_mutate_failed(
                    TIMELINE_MUTATE_FAILED_CODE,
                    TIMELINE_MUTATE_FAILED_DESCRIPTION,
                )
            })?;
        timeline_mutate_dto(result)
    }

    async fn timeline_pin_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelinePinDto, TimelinePinError> {
        let payload = timeline_pin_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_PIN_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_pin_core_error(no_session, error))?;
        let result: TimelinePinResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
            })?;
        timeline_pin_dto(result)
    }

    async fn timeline_vote_decline_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
        let payload = timeline_vote_decline_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_VOTE_DECLINE_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_vote_decline_core_error(no_session, error))?;
        let result: TimelineVoteDeclineResultWire = serde_json::from_value(response.payload)
            .map_err(|_| {
                timeline_vote_decline_failed(
                    TIMELINE_VOTE_DECLINE_FAILED_CODE,
                    TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
                )
            })?;
        timeline_vote_decline_dto(result)
    }

    async fn timeline_forward_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<TimelineForwardDto, TimelineForwardError> {
        let payload = timeline_forward_envelope_payload(payload)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: TIMELINE_FORWARD_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_timeline_forward_core_error(no_session, error))?;
        let result: TimelineForwardResultWire =
            serde_json::from_value(response.payload).map_err(|_| {
                timeline_forward_failed(
                    TIMELINE_FORWARD_FAILED_CODE,
                    TIMELINE_FORWARD_FAILED_DESCRIPTION,
                )
            })?;
        timeline_forward_dto(result)
    }

    async fn session_status_command(
        &self,
        command: &'static str,
    ) -> Result<serde_json::Value, SessionStatusError> {
        let payload = session_status_envelope_payload(serde_json::Value::Null)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: SESSION_STATUS_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(map_session_status_core_error)?;
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
    ) -> Result<serde_json::Value, OwnProfileCommandError> {
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
        Ok(response.payload)
    }

    async fn ignored_users_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, IgnoredUsersCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: IGNORED_USERS_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_ignored_users_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn push_rules_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, PushRulesCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: PUSH_RULES_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_push_rules_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn room_notification_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, RoomNotificationCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_NOTIFICATION_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_notification_core_error(no_session, error))?;
        Ok(response.payload)
    }

    async fn threepid_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value, ThreepidCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: THREEPID_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_threepid_core_error(no_session, error))?;
        Ok(response.payload)
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
    pub own_verification: String,
    pub has_devices_to_verify_against: Option<bool>,
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

/// Privacy-safe join-rule write ack. Status only; no room id, join rule, or allow-list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomJoinRuleWriteDto {
    pub status: String,
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
    map_join_rule_core_error_with_no_session(JOIN_RULE_SNAPSHOT_NO_SESSION_CODE, error)
}

fn map_join_rule_core_error_with_no_session(
    no_session: &'static str,
    error: MatrixIpcError,
) -> JoinRuleCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code == JOIN_RULE_SNAPSHOT_NO_SESSION_CODE
                || code == JOIN_RULE_SET_NO_SESSION_CODE =>
        {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-room-profile-join-rule-") => {
            join_rule_failed(code, JOIN_RULE_OWNER_DESCRIPTION)
        }
        _ => join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION),
    }
}

fn join_rule_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, JoinRuleCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(join_rule_failed(
            JOIN_RULE_FAILED_CODE,
            JOIN_RULE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
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

/// Privacy-safe own-profile read. Avatar is an `mxc://` URI only; never bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnProfileDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
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

fn upload_content_failed(code: &str, description: &'static str) -> MediaUploadError {
    MediaUploadError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_upload_content_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> MediaUploadError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            upload_content_failed(code, UPLOAD_CONTENT_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            upload_content_failed(code, UPLOAD_CONTENT_OWNER_DESCRIPTION)
        }
        _ => upload_content_failed(
            UPLOAD_CONTENT_FAILED_CODE,
            UPLOAD_CONTENT_FAILED_DESCRIPTION,
        ),
    }
}

fn upload_content_reject_oversize(size: usize) -> Result<(), MediaUploadError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(upload_content_failed(
            UPLOAD_CONTENT_FAILED_CODE,
            UPLOAD_CONTENT_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

fn send_room_attachment_failed(code: &str, description: &'static str) -> SendRoomAttachmentError {
    SendRoomAttachmentError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_send_room_attachment_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> SendRoomAttachmentError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_room_attachment_failed(code, SEND_ROOM_ATTACHMENT_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            send_room_attachment_failed(code, SEND_ROOM_ATTACHMENT_OWNER_DESCRIPTION)
        }
        _ => send_room_attachment_failed(
            SEND_ROOM_ATTACHMENT_FAILED_CODE,
            SEND_ROOM_ATTACHMENT_FAILED_DESCRIPTION,
        ),
    }
}

fn send_room_attachment_reject_oversize(size: usize) -> Result<(), SendRoomAttachmentError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_room_attachment_failed(
            SEND_ROOM_ATTACHMENT_FAILED_CODE,
            SEND_ROOM_ATTACHMENT_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

fn plain_media_failed(code: &str, description: &'static str) -> PlainMediaError {
    PlainMediaError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_plain_media_core_error(no_session: &'static str, error: MatrixIpcError) -> PlainMediaError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            plain_media_failed(code, PLAIN_MEDIA_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.") => {
            plain_media_failed(code, PLAIN_MEDIA_OWNER_DESCRIPTION)
        }
        _ => plain_media_failed(PLAIN_MEDIA_FAILED_CODE, PLAIN_MEDIA_FAILED_DESCRIPTION),
    }
}

fn plain_media_reject_oversize(size: usize) -> Result<(), PlainMediaError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(plain_media_failed(
            PLAIN_MEDIA_FAILED_CODE,
            PLAIN_MEDIA_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
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

fn own_profile_dto(payload: serde_json::Value) -> Result<OwnProfileDto, OwnProfileCommandError> {
    let profile: crate::app::user_profile::MatrixOwnProfile = serde_json::from_value(payload)
        .map_err(|_| own_profile_failed(OWN_PROFILE_FAILED_CODE, OWN_PROFILE_FAILED_DESCRIPTION))?;
    let avatar_url = profile.avatar_url.filter(|mxc| mxc.starts_with("mxc://"));
    Ok(OwnProfileDto {
        user_id: profile.user_id,
        display_name: profile.display_name,
        avatar_url,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnProfileUploadDto {
    pub mxc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredUsersSnapshotDto {
    pub user_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredUsersWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IgnoredUsersCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for IgnoredUsersCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for IgnoredUsersCommandError {}

fn ignored_users_failed(code: &str, description: &'static str) -> IgnoredUsersCommandError {
    IgnoredUsersCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_ignored_users_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> IgnoredUsersCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            ignored_users_failed(code, IGNORED_USERS_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-profile.ignore-") => {
            ignored_users_failed(code, IGNORED_USERS_OWNER_DESCRIPTION)
        }
        _ => ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION),
    }
}

fn ignored_users_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, IgnoredUsersCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(ignored_users_failed(
            IGNORED_USERS_FAILED_CODE,
            IGNORED_USERS_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn ignored_users_snapshot_dto(
    payload: serde_json::Value,
) -> Result<IgnoredUsersSnapshotDto, IgnoredUsersCommandError> {
    let snapshot: crate::app::user_profile::MatrixIgnoredUsersSnapshot =
        serde_json::from_value(payload).map_err(|_| {
            ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION)
        })?;
    Ok(IgnoredUsersSnapshotDto {
        user_ids: snapshot.user_ids,
    })
}

fn ignored_users_write_dto(
    payload: serde_json::Value,
) -> Result<IgnoredUsersWriteDto, IgnoredUsersCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            ignored_users_failed(IGNORED_USERS_FAILED_CODE, IGNORED_USERS_FAILED_DESCRIPTION)
        })?;
    Ok(IgnoredUsersWriteDto {
        status: status.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDirectoryHitDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDirectorySearchDto {
    pub limited: bool,
    pub results: Vec<UserDirectoryHitDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserDirectorySearchError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for UserDirectorySearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for UserDirectorySearchError {}

fn user_directory_search_failed(code: &str, description: &'static str) -> UserDirectorySearchError {
    UserDirectorySearchError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_user_directory_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> UserDirectorySearchError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            user_directory_search_failed(code, USER_DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-search.") || code.starts_with("v-directory.") => {
            user_directory_search_failed(code, USER_DIRECTORY_SEARCH_OWNER_DESCRIPTION)
        }
        _ => user_directory_search_failed(
            USER_DIRECTORY_SEARCH_FAILED_CODE,
            USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ),
    }
}

fn user_directory_search_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, UserDirectorySearchError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(user_directory_search_failed(
            USER_DIRECTORY_SEARCH_FAILED_CODE,
            USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn user_directory_search_dto(
    payload: serde_json::Value,
) -> Result<UserDirectorySearchDto, UserDirectorySearchError> {
    let result: crate::app::user_profile::MatrixUserDirectorySearchResult =
        serde_json::from_value(payload).map_err(|_| {
            user_directory_search_failed(
                USER_DIRECTORY_SEARCH_FAILED_CODE,
                USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut results = Vec::with_capacity(result.results.len());
    for hit in result.results {
        if matrix_sdk::ruma::UserId::parse(hit.user_id.as_str()).is_err() {
            continue;
        }
        let avatar_url = hit.avatar_url.filter(|mxc| mxc.starts_with("mxc://"));
        results.push(UserDirectoryHitDto {
            user_id: hit.user_id,
            display_name: hit.display_name,
            avatar_url,
        });
    }
    Ok(UserDirectorySearchDto {
        limited: result.limited,
        results,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageSearchItemDto {
    pub rank: f64,
    pub event_id: String,
    pub sender: String,
    pub origin_server_ts: u64,
    pub body: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageSearchGroupDto {
    pub room_id: String,
    pub items: Vec<MessageSearchItemDto>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MessageSearchDto {
    pub next_token: Option<String>,
    pub highlights: Vec<String>,
    pub groups: Vec<MessageSearchGroupDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageSearchError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for MessageSearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for MessageSearchError {}

fn message_search_failed(code: &str, description: &'static str) -> MessageSearchError {
    MessageSearchError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_message_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> MessageSearchError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            message_search_failed(code, MESSAGE_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-search.") => {
            message_search_failed(code, MESSAGE_SEARCH_OWNER_DESCRIPTION)
        }
        _ => message_search_failed(
            MESSAGE_SEARCH_FAILED_CODE,
            MESSAGE_SEARCH_FAILED_DESCRIPTION,
        ),
    }
}

fn message_search_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, MessageSearchError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(message_search_failed(
            MESSAGE_SEARCH_FAILED_CODE,
            MESSAGE_SEARCH_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn message_search_dto(payload: serde_json::Value) -> Result<MessageSearchDto, MessageSearchError> {
    let result: crate::app::search::MatrixMessageSearchResult = serde_json::from_value(payload)
        .map_err(|_| {
            message_search_failed(
                MESSAGE_SEARCH_FAILED_CODE,
                MESSAGE_SEARCH_FAILED_DESCRIPTION,
            )
        })?;
    let mut highlights = Vec::new();
    for highlight in result.highlights {
        let trimmed = highlight.trim();
        if trimmed.is_empty() {
            continue;
        }
        if highlights.len() >= crate::app::search::MAX_MESSAGE_SEARCH_HIGHLIGHTS {
            break;
        }
        highlights.push(
            trimmed
                .chars()
                .take(crate::app::search::MAX_MESSAGE_SEARCH_HIGHLIGHT_CHARS)
                .collect(),
        );
    }
    let next_token = result.next_token.filter(|token| {
        !token.is_empty()
            && token.chars().count() <= crate::app::search::MAX_MESSAGE_SEARCH_NEXT_TOKEN_CHARS
            && !token.contains("syt_")
            && !token.contains("access_token")
    });
    let mut groups = Vec::new();
    let mut items_seen = 0usize;
    for group in result.groups {
        if groups.len() >= crate::app::search::MAX_MESSAGE_SEARCH_GROUPS {
            break;
        }
        if !group.room_id.starts_with('!') {
            continue;
        }
        let mut items = Vec::new();
        for item in group.items {
            if items_seen >= crate::app::search::MAX_MESSAGE_SEARCH_ITEMS {
                break;
            }
            if !item.event_id.starts_with('$') || !item.sender.starts_with('@') {
                continue;
            }
            let body = item
                .body
                .chars()
                .take(crate::app::search::MAX_MESSAGE_SEARCH_BODY_CHARS)
                .collect();
            items.push(MessageSearchItemDto {
                rank: item.rank,
                event_id: item.event_id,
                sender: item.sender,
                origin_server_ts: item.origin_server_ts,
                body,
                room_id: group.room_id.clone(),
            });
            items_seen += 1;
        }
        if items.is_empty() {
            continue;
        }
        groups.push(MessageSearchGroupDto {
            room_id: group.room_id,
            items,
        });
    }
    Ok(MessageSearchDto {
        next_token,
        highlights,
        groups,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRuleMentionsDto {
    pub user_mention: bool,
    pub display_name: bool,
    pub user_name: bool,
    pub room_mention: bool,
    pub at_room: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRulesSnapshotDto {
    pub dm: String,
    pub dm_encrypted: String,
    pub group: String,
    pub group_encrypted: String,
    pub mentions: PushRuleMentionsDto,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushRulesWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushRulesCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PushRulesCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PushRulesCommandError {}

fn push_rules_failed(code: &str, description: &'static str) -> PushRulesCommandError {
    PushRulesCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_push_rules_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PushRulesCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            push_rules_failed(code, PUSH_RULES_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-push.") => {
            push_rules_failed(code, PUSH_RULES_OWNER_DESCRIPTION)
        }
        _ => push_rules_failed(PUSH_RULES_FAILED_CODE, PUSH_RULES_FAILED_DESCRIPTION),
    }
}

fn push_rules_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, PushRulesCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(push_rules_failed(
            PUSH_RULES_FAILED_CODE,
            PUSH_RULES_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn push_rules_snapshot_dto(
    payload: serde_json::Value,
) -> Result<PushRulesSnapshotDto, PushRulesCommandError> {
    let snapshot: crate::app::notifications::MatrixPushRulesSnapshot =
        serde_json::from_value(payload).map_err(|_| {
            push_rules_failed(PUSH_RULES_FAILED_CODE, PUSH_RULES_FAILED_DESCRIPTION)
        })?;
    Ok(PushRulesSnapshotDto {
        dm: snapshot.dm,
        dm_encrypted: snapshot.dm_encrypted,
        group: snapshot.group,
        group_encrypted: snapshot.group_encrypted,
        mentions: PushRuleMentionsDto {
            user_mention: snapshot.mentions.user_mention,
            display_name: snapshot.mentions.display_name,
            user_name: snapshot.mentions.user_name,
            room_mention: snapshot.mentions.room_mention,
            at_room: snapshot.mentions.at_room,
        },
        keywords: snapshot.keywords,
    })
}

fn push_rules_write_dto(
    payload: serde_json::Value,
) -> Result<PushRulesWriteDto, PushRulesCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| push_rules_failed(PUSH_RULES_FAILED_CODE, PUSH_RULES_FAILED_DESCRIPTION))?;
    Ok(PushRulesWriteDto {
        status: status.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomNotificationSnapshotDto {
    pub room_id: String,
    pub mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomNotificationsSnapshotDto {
    pub rooms: Vec<RoomNotificationSnapshotDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomNotificationWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomNotificationCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomNotificationCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomNotificationCommandError {}

fn room_notification_failed(code: &str, description: &'static str) -> RoomNotificationCommandError {
    RoomNotificationCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_room_notification_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomNotificationCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_notification_failed(code, ROOM_NOTIFICATION_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-push.") => {
            room_notification_failed(code, ROOM_NOTIFICATION_OWNER_DESCRIPTION)
        }
        _ => room_notification_failed(
            ROOM_NOTIFICATION_FAILED_CODE,
            ROOM_NOTIFICATION_FAILED_DESCRIPTION,
        ),
    }
}

fn room_notification_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomNotificationCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_notification_failed(
            ROOM_NOTIFICATION_FAILED_CODE,
            ROOM_NOTIFICATION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn room_notification_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomNotificationSnapshotDto, RoomNotificationCommandError> {
    let snapshot: crate::app::notifications::MatrixRoomNotificationSnapshot =
        serde_json::from_value(payload).map_err(|_| {
            room_notification_failed(
                ROOM_NOTIFICATION_FAILED_CODE,
                ROOM_NOTIFICATION_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomNotificationSnapshotDto {
        room_id: snapshot.room_id,
        mode: snapshot.mode,
    })
}

fn room_notifications_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomNotificationsSnapshotDto, RoomNotificationCommandError> {
    let snapshot: crate::app::notifications::MatrixRoomNotificationsSnapshot =
        serde_json::from_value(payload).map_err(|_| {
            room_notification_failed(
                ROOM_NOTIFICATION_FAILED_CODE,
                ROOM_NOTIFICATION_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomNotificationsSnapshotDto {
        rooms: snapshot
            .rooms
            .into_iter()
            .map(|room| RoomNotificationSnapshotDto {
                room_id: room.room_id,
                mode: room.mode,
            })
            .collect(),
    })
}

fn room_notification_write_dto(
    payload: serde_json::Value,
) -> Result<RoomNotificationWriteDto, RoomNotificationCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_notification_failed(
                ROOM_NOTIFICATION_FAILED_CODE,
                ROOM_NOTIFICATION_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomNotificationWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe HTTP pusher write ack. Status only; never push key or URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PusherWriteDto {
    pub status: String,
}

/// Account-bound HTTP pusher capability. It retains the exact Core owner—and
/// therefore the exact authenticated Matrix client—captured at bind time.
/// No account identity, token, push key, or gateway is projected back out.
pub struct HttpPusherOwner {
    owner: Arc<NativeHttpPusherOwner>,
}

impl HttpPusherOwner {
    pub async fn register_http_pusher(
        &self,
        push_key: String,
        app_id: String,
        gateway_url: String,
        app_display_name: String,
        lang: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        http_pusher_reject_oversize(gateway_url.len())?;
        http_pusher_reject_oversize(app_display_name.len())?;
        http_pusher_reject_oversize(lang.len())?;
        // UniFFI 0.28 does not propagate Swift Task cancellation into this
        // future. Logout awaits reconciliation, so bound the entire write.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner
                .register(&push_key, &app_id, &gateway_url, &app_display_name, &lang),
        )
        .await
        .unwrap_or(Err("pusher-registration-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                REGISTER_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.to_owned(),
        })
    }

    pub async fn delete_http_pusher(
        &self,
        push_key: String,
        app_id: String,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(push_key.len())?;
        http_pusher_reject_oversize(app_id.len())?;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner.delete(&push_key, &app_id),
        )
        .await
        .unwrap_or(Err("pusher-deletion-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                DELETE_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.to_owned(),
        })
    }

    pub async fn delete_http_pushers_for_device(
        &self,
        app_id: String,
        last_push_key: Option<String>,
    ) -> Result<PusherWriteDto, PusherCommandError> {
        http_pusher_reject_oversize(app_id.len())?;
        if let Some(push_key) = last_push_key.as_ref() {
            http_pusher_reject_oversize(push_key.len())?;
        }
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.owner
                .delete_for_device(&app_id, last_push_key.as_deref()),
        )
        .await
        .unwrap_or(Err("pusher-cleanup-timeout"))
        .map_err(|error| {
            map_http_pusher_core_error(
                DELETE_HTTP_PUSHER_NO_SESSION_CODE,
                MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant).with_diagnostic(error),
            )
        })?;
        Ok(PusherWriteDto {
            status: result.status.to_owned(),
        })
    }
}

/// Static fail-closed HTTP pusher-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PusherCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for PusherCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for PusherCommandError {}

fn http_pusher_failed(code: &str, description: &'static str) -> PusherCommandError {
    PusherCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn http_pusher_reject_oversize(size: usize) -> Result<(), PusherCommandError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(http_pusher_failed(
            HTTP_PUSHER_FAILED_CODE,
            HTTP_PUSHER_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

fn map_http_pusher_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PusherCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            http_pusher_failed(code, HTTP_PUSHER_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-pusher.") || code.starts_with("v-push.") => {
            http_pusher_failed(code, HTTP_PUSHER_OWNER_DESCRIPTION)
        }
        _ => http_pusher_failed(HTTP_PUSHER_FAILED_CODE, HTTP_PUSHER_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe backup restore ack. Status only; never recovery key or passphrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreBackupDto {
    pub status: String,
}

/// Static fail-closed backup restore error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreBackupError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RestoreBackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RestoreBackupError {}

fn restore_backup_failed(code: &str, description: &'static str) -> RestoreBackupError {
    RestoreBackupError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn restore_backup_reject_oversize(size: usize) -> Result<(), RestoreBackupError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(restore_backup_failed(
            RESTORE_BACKUP_FAILED_CODE,
            RESTORE_BACKUP_FAILED_DESCRIPTION,
        ));
    }
    Ok(())
}

fn map_restore_backup_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RestoreBackupError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            restore_backup_failed(code, RESTORE_BACKUP_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-crypto.3-") => {
            restore_backup_failed(code, RESTORE_BACKUP_OWNER_DESCRIPTION)
        }
        _ => restore_backup_failed(
            RESTORE_BACKUP_FAILED_CODE,
            RESTORE_BACKUP_FAILED_DESCRIPTION,
        ),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreepidEmailDto {
    pub address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreepidSnapshotDto {
    pub emails: Vec<ThreepidEmailDto>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreepidWriteDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreepidEmailTokenDto {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreepidAddDto {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreepidCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for ThreepidCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ThreepidCommandError {}

fn threepid_failed(code: &str, description: &'static str) -> ThreepidCommandError {
    ThreepidCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_threepid_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ThreepidCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => threepid_failed(code, THREEPID_NO_SESSION_DESCRIPTION),
        Some(code) if code.starts_with("v-threepid.") => {
            threepid_failed(code, THREEPID_OWNER_DESCRIPTION)
        }
        _ => threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION),
    }
}

fn threepid_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, ThreepidCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(threepid_failed(
            THREEPID_FAILED_CODE,
            THREEPID_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn threepid_snapshot_dto(
    payload: serde_json::Value,
) -> Result<ThreepidSnapshotDto, ThreepidCommandError> {
    let snapshot: crate::app::user_profile::MatrixThreepidSnapshot =
        serde_json::from_value(payload)
            .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidSnapshotDto {
        emails: snapshot
            .emails
            .into_iter()
            .map(|email| ThreepidEmailDto {
                address: email.address,
            })
            .collect(),
    })
}

fn threepid_write_dto(
    payload: serde_json::Value,
) -> Result<ThreepidWriteDto, ThreepidCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidWriteDto {
        status: status.to_owned(),
    })
}

fn threepid_email_token_dto(
    payload: serde_json::Value,
) -> Result<ThreepidEmailTokenDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidEmailTokenResult =
        serde_json::from_value(payload)
            .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidEmailTokenDto {
        session_id: result.session_id,
    })
}

fn threepid_add_dto(payload: serde_json::Value) -> Result<ThreepidAddDto, ThreepidCommandError> {
    let result: crate::app::user_profile::MatrixThreepidAddResult = serde_json::from_value(payload)
        .map_err(|_| threepid_failed(THREEPID_FAILED_CODE, THREEPID_FAILED_DESCRIPTION))?;
    Ok(ThreepidAddDto {
        status: result.status,
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
                || code.starts_with("v-rooms-room-favorite-")
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

fn timeline_read_state_failed(code: &str, description: &'static str) -> TimelineReadStateError {
    TimelineReadStateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_read_state_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineReadStateError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_read_state_failed(code, TIMELINE_READ_STATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-event-readback-")
                || code.starts_with("p2-timeline-set-read-state-")
                || code.starts_with("p2-timeline-jump-latest-")
                || code.starts_with("d0.3-timeline-")
                || code.starts_with("v-crypto.6-")
                || code.starts_with("v-timeline-") =>
        {
            timeline_read_state_failed(code, TIMELINE_READ_STATE_OWNER_DESCRIPTION)
        }
        _ => timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        ),
    }
}

fn timeline_read_state_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineReadStateError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn read_action_from_str(action: &str) -> Result<NativeTimelineReadAction, TimelineReadStateError> {
    match action {
        "mark_read" => Ok(NativeTimelineReadAction::MarkRead),
        "mark_unread" => Ok(NativeTimelineReadAction::MarkUnread),
        _ => Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        )),
    }
}

fn read_action_as_str(action: NativeTimelineReadAction) -> &'static str {
    match action {
        NativeTimelineReadAction::MarkRead => "mark_read",
        NativeTimelineReadAction::MarkUnread => "mark_unread",
    }
}

fn read_intent_from_str(intent: &str) -> Result<NativeTimelineReadIntent, TimelineReadStateError> {
    match intent {
        "automatic_visibility" => Ok(NativeTimelineReadIntent::AutomaticVisibility),
        "explicit_user" => Ok(NativeTimelineReadIntent::ExplicitUser),
        _ => Err(timeline_read_state_failed(
            TIMELINE_READ_STATE_FAILED_CODE,
            TIMELINE_READ_STATE_FAILED_DESCRIPTION,
        )),
    }
}

fn read_intent_as_str(intent: NativeTimelineReadIntent) -> &'static str {
    match intent {
        NativeTimelineReadIntent::AutomaticVisibility => "automatic_visibility",
        NativeTimelineReadIntent::ExplicitUser => "explicit_user",
    }
}

fn timeline_reaction_failed(code: &str, description: &'static str) -> TimelineReactionError {
    TimelineReactionError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_reaction_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineReactionError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_reaction_failed(code, TIMELINE_REACTION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-reaction-ensure-")
                || code.starts_with("p2-reaction-redact-")
                || code.starts_with("p2-timeline-reaction-toggle-")
                || code.starts_with("d0.3-timeline-")
                || code.starts_with("v-crypto.6-")
                || code.starts_with("v-send.2-reaction-")
                || code.starts_with("agent-approval-") =>
        {
            timeline_reaction_failed(code, TIMELINE_REACTION_OWNER_DESCRIPTION)
        }
        _ => timeline_reaction_failed(
            TIMELINE_REACTION_FAILED_CODE,
            TIMELINE_REACTION_FAILED_DESCRIPTION,
        ),
    }
}

fn timeline_reaction_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineReactionError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_reaction_failed(
            TIMELINE_REACTION_FAILED_CODE,
            TIMELINE_REACTION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

fn reaction_mutation_as_str(mutation: NativeReactionMutation) -> &'static str {
    match mutation {
        NativeReactionMutation::Added => "added",
        NativeReactionMutation::Removed => "removed",
        NativeReactionMutation::AlreadyPresent => "already_present",
        NativeReactionMutation::Redacted => "redacted",
    }
}

fn timeline_reaction_sender_dto(sender: NativeTimelineReactionSender) -> TimelineReactionSenderDto {
    TimelineReactionSenderDto {
        user_id: sender.user_id,
        reaction_event_id: sender.reaction_event_id,
    }
}

fn timeline_reaction_dto(reaction: NativeTimelineReaction) -> TimelineReactionDto {
    TimelineReactionDto {
        key: reaction.key,
        count: reaction.count,
        me: reaction.me,
        senders: reaction
            .senders
            .into_iter()
            .map(timeline_reaction_sender_dto)
            .collect(),
    }
}

fn timeline_reaction_mutation_dto(
    result: NativeReactionMutationResult,
) -> TimelineReactionMutationDto {
    TimelineReactionMutationDto {
        room_id: result.room_id,
        target_event_id: result.target_event_id,
        key: result.key,
        mutation: reaction_mutation_as_str(result.mutation).to_owned(),
        readback: result.readback.map(timeline_reaction_dto),
    }
}

fn composer_reply_draft_failed(code: &str, description: &'static str) -> ComposerReplyDraftError {
    ComposerReplyDraftError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_composer_reply_draft_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> ComposerReplyDraftError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code == COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE
                || code == COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE
                || code == COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE =>
        {
            // The revision-less clear compatibility route snapshots through
            // the get command before it performs the atomic clear. Preserve
            // the public operation's diagnostic rather than leaking that
            // internal owner hop to callers.
            composer_reply_draft_failed(no_session, COMPOSER_REPLY_DRAFT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-composer-set-reply-draft-")
                || code.starts_with("p2-composer-get-reply-draft-")
                || code.starts_with("p2-composer-clear-reply-draft-")
                || code.starts_with("v-timeline-reply-draft-")
                || code == "d0.4-send-invalid-room-id" =>
        {
            composer_reply_draft_failed(code, COMPOSER_REPLY_DRAFT_OWNER_DESCRIPTION)
        }
        _ => composer_reply_draft_failed(
            COMPOSER_REPLY_DRAFT_FAILED_CODE,
            COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
        ),
    }
}

fn composer_reply_draft_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, ComposerReplyDraftError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(composer_reply_draft_failed(
            COMPOSER_REPLY_DRAFT_FAILED_CODE,
            COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComposerReplyDraftReadbackWire {
    schema_version: u32,
    room_id: String,
    status: String,
    #[serde(default)]
    draft: Option<NativeComposerReplyDraft>,
}

fn composer_reply_draft_preview_dto(
    draft: NativeComposerReplyDraft,
) -> ComposerReplyDraftPreviewDto {
    ComposerReplyDraftPreviewDto {
        event_id: draft.event_id,
        sender_id: draft.sender_id,
        body: draft.body,
        formatted_body: draft.formatted_body,
        thread_root_event_id: draft.thread_root_event_id,
    }
}

fn composer_reply_draft_dto(readback: ComposerReplyDraftReadbackWire) -> ComposerReplyDraftDto {
    ComposerReplyDraftDto {
        schema_version: readback.schema_version,
        room_id: readback.room_id,
        status: readback.status,
        draft: readback.draft.map(composer_reply_draft_preview_dto),
    }
}

fn send_text_failed(code: &str, description: &'static str) -> SendTextError {
    SendTextError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn agent_approval_failed(code: &str, description: &'static str) -> AgentApprovalSendError {
    AgentApprovalSendError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_send_text_core_error(no_session: &'static str, error: MatrixIpcError) -> SendTextError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_text_failed(code, SEND_TEXT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-send-text-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.4-")
                || code.starts_with("v-send.5-")
                || code.starts_with("p6.1-") =>
        {
            send_text_failed(code, SEND_TEXT_OWNER_DESCRIPTION)
        }
        _ => send_text_failed(SEND_TEXT_FAILED_CODE, SEND_TEXT_FAILED_DESCRIPTION),
    }
}

fn send_text_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SendTextError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_text_failed(
            SEND_TEXT_FAILED_CODE,
            SEND_TEXT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendTextResultWire {
    room_id: String,
    event_id: String,
    local_txn_id: String,
    status: String,
}

fn send_text_dto(result: SendTextResultWire) -> SendTextDto {
    SendTextDto {
        room_id: result.room_id,
        event_id: result.event_id,
        local_txn_id: result.local_txn_id,
        status: result.status,
    }
}

fn send_poll_failed(code: &str, description: &'static str) -> SendPollError {
    SendPollError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_send_poll_core_error(no_session: &'static str, error: MatrixIpcError) -> SendPollError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            send_poll_failed(code, SEND_POLL_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-send-poll-")
                || code.starts_with("v-send.3-poll-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.5-") =>
        {
            send_poll_failed(code, SEND_POLL_OWNER_DESCRIPTION)
        }
        _ => send_poll_failed(SEND_POLL_FAILED_CODE, SEND_POLL_FAILED_DESCRIPTION),
    }
}

fn send_poll_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SendPollError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(send_poll_failed(
            SEND_POLL_FAILED_CODE,
            SEND_POLL_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendPollResultWire {
    room_id: String,
    event_id: String,
    status: String,
}

fn send_poll_dto(result: SendPollResultWire) -> SendPollDto {
    SendPollDto {
        room_id: result.room_id,
        event_id: result.event_id,
        status: result.status,
    }
}

fn edit_message_failed(code: &str, description: &'static str) -> EditMessageError {
    EditMessageError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_edit_message_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> EditMessageError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            edit_message_failed(code, EDIT_MESSAGE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-edit-message-")
                || code.starts_with("v-send.r-edit-")
                || code.starts_with("d0.4-send-")
                || code.starts_with("v-send.4-")
                || code.starts_with("p6.1-") =>
        {
            edit_message_failed(code, EDIT_MESSAGE_OWNER_DESCRIPTION)
        }
        _ => edit_message_failed(EDIT_MESSAGE_FAILED_CODE, EDIT_MESSAGE_FAILED_DESCRIPTION),
    }
}

fn edit_message_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, EditMessageError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(edit_message_failed(
            EDIT_MESSAGE_FAILED_CODE,
            EDIT_MESSAGE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditMessageResultWire {
    room_id: String,
    event_id: String,
    local_txn_id: String,
    status: String,
}

fn edit_message_dto(result: EditMessageResultWire) -> EditMessageDto {
    EditMessageDto {
        room_id: result.room_id,
        event_id: result.event_id,
        local_txn_id: result.local_txn_id,
        status: result.status,
    }
}

fn poll_respond_failed(code: &str, description: &'static str) -> PollRespondError {
    PollRespondError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_poll_respond_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> PollRespondError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            poll_respond_failed(code, POLL_RESPOND_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-poll-respond-")
                || code.starts_with("v-send.3-poll-")
                || code.starts_with("d0.4-send-") =>
        {
            poll_respond_failed(code, POLL_RESPOND_OWNER_DESCRIPTION)
        }
        _ => poll_respond_failed(POLL_RESPOND_FAILED_CODE, POLL_RESPOND_FAILED_DESCRIPTION),
    }
}

fn poll_respond_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, PollRespondError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(poll_respond_failed(
            POLL_RESPOND_FAILED_CODE,
            POLL_RESPOND_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollRespondResultWire {
    room_id: String,
    poll_event_id: String,
    event_id: String,
    status: String,
}

fn poll_respond_dto(result: PollRespondResultWire) -> PollRespondDto {
    PollRespondDto {
        room_id: result.room_id,
        poll_event_id: result.poll_event_id,
        event_id: result.event_id,
        status: result.status,
    }
}

fn timeline_mutate_failed(code: &str, description: &'static str) -> TimelineMutateError {
    TimelineMutateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_mutate_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineMutateError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_mutate_failed(code, TIMELINE_MUTATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-edit-text-")
                || code.starts_with("p2-timeline-redact-")
                || code.starts_with("p2-timeline-report-")
                || code.starts_with("v-timeline-edit-")
                || code.starts_with("v-timeline-redact-")
                || code.starts_with("v-timeline-report-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_mutate_failed(code, TIMELINE_MUTATE_OWNER_DESCRIPTION)
        }
        _ => timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        ),
    }
}

fn timeline_mutate_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineMutateError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineMutateResultWire {
    schema_version: u32,
    action: String,
    room_id: String,
    event_id: String,
    status: String,
}

fn closed_timeline_mutate_action(value: &str) -> Option<&'static str> {
    match value {
        "edit_text" => Some("edit_text"),
        "redact" => Some("redact"),
        "report" => Some("report"),
        _ => None,
    }
}

fn closed_timeline_mutate_status(value: &str) -> Option<&'static str> {
    match value {
        "sent" => Some("sent"),
        "redacted" => Some("redacted"),
        "reported" => Some("reported"),
        _ => None,
    }
}

fn timeline_mutate_dto(
    result: TimelineMutateResultWire,
) -> Result<TimelineMutateDto, TimelineMutateError> {
    let action = closed_timeline_mutate_action(&result.action).ok_or_else(|| {
        timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_mutate_status(&result.status).ok_or_else(|| {
        timeline_mutate_failed(
            TIMELINE_MUTATE_FAILED_CODE,
            TIMELINE_MUTATE_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineMutateDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

fn timeline_pin_failed(code: &str, description: &'static str) -> TimelinePinError {
    TimelinePinError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_pin_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelinePinError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_pin_failed(code, TIMELINE_PIN_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-pin-")
                || code.starts_with("p2-timeline-unpin-")
                || code.starts_with("v-timeline-pin-")
                || code.starts_with("v-timeline-unpin-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_pin_failed(code, TIMELINE_PIN_OWNER_DESCRIPTION)
        }
        _ => timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION),
    }
}

fn timeline_pin_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelinePinError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_pin_failed(
            TIMELINE_PIN_FAILED_CODE,
            TIMELINE_PIN_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelinePinResultWire {
    schema_version: u32,
    action: String,
    room_id: String,
    event_id: String,
    status: String,
}

fn closed_timeline_pin_action(value: &str) -> Option<&'static str> {
    match value {
        "pin" => Some("pin"),
        "unpin" => Some("unpin"),
        _ => None,
    }
}

fn closed_timeline_pin_status(value: &str) -> Option<&'static str> {
    match value {
        "pinned" => Some("pinned"),
        "unpinned" => Some("unpinned"),
        "already_pinned" => Some("already_pinned"),
        "already_unpinned" => Some("already_unpinned"),
        _ => None,
    }
}

fn timeline_pin_dto(result: TimelinePinResultWire) -> Result<TimelinePinDto, TimelinePinError> {
    let action = closed_timeline_pin_action(&result.action).ok_or_else(|| {
        timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
    })?;
    let status = closed_timeline_pin_status(&result.status).ok_or_else(|| {
        timeline_pin_failed(TIMELINE_PIN_FAILED_CODE, TIMELINE_PIN_FAILED_DESCRIPTION)
    })?;
    Ok(TimelinePinDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

fn timeline_vote_decline_failed(code: &str, description: &'static str) -> TimelineVoteDeclineError {
    TimelineVoteDeclineError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_vote_decline_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineVoteDeclineError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_vote_decline_failed(code, TIMELINE_VOTE_DECLINE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-poll-vote-")
                || code.starts_with("p2-timeline-call-decline-")
                || code.starts_with("v-timeline-poll-vote-")
                || code.starts_with("v-timeline-call-decline-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_vote_decline_failed(code, TIMELINE_VOTE_DECLINE_OWNER_DESCRIPTION)
        }
        _ => timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        ),
    }
}

fn timeline_vote_decline_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineVoteDeclineError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineVoteDeclineResultWire {
    schema_version: u32,
    action: String,
    room_id: String,
    event_id: String,
    status: String,
}

fn closed_timeline_vote_decline_action(value: &str) -> Option<&'static str> {
    match value {
        "poll_vote" => Some("poll_vote"),
        "call_decline" => Some("call_decline"),
        _ => None,
    }
}

fn closed_timeline_vote_decline_status(value: &str) -> Option<&'static str> {
    match value {
        "voted" => Some("voted"),
        "declined" => Some("declined"),
        _ => None,
    }
}

fn timeline_vote_decline_dto(
    result: TimelineVoteDeclineResultWire,
) -> Result<TimelineVoteDeclineDto, TimelineVoteDeclineError> {
    let action = closed_timeline_vote_decline_action(&result.action).ok_or_else(|| {
        timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_vote_decline_status(&result.status).ok_or_else(|| {
        timeline_vote_decline_failed(
            TIMELINE_VOTE_DECLINE_FAILED_CODE,
            TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineVoteDeclineDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

fn timeline_forward_failed(code: &str, description: &'static str) -> TimelineForwardError {
    TimelineForwardError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_timeline_forward_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> TimelineForwardError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            timeline_forward_failed(code, TIMELINE_FORWARD_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-timeline-forward-text-")
                || code.starts_with("p2-timeline-forward-media-")
                || code.starts_with("v-timeline-forward-")
                || code.starts_with("d0.4-send-") =>
        {
            timeline_forward_failed(code, TIMELINE_FORWARD_OWNER_DESCRIPTION)
        }
        _ => timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        ),
    }
}

fn timeline_forward_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, TimelineForwardError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineForwardResultWire {
    schema_version: u32,
    action: String,
    room_id: String,
    event_id: String,
    status: String,
}

fn closed_timeline_forward_action(value: &str) -> Option<&'static str> {
    match value {
        "forward_text" => Some("forward_text"),
        "forward_media" => Some("forward_media"),
        _ => None,
    }
}

fn closed_timeline_forward_status(value: &str) -> Option<&'static str> {
    match value {
        "sent" => Some("sent"),
        _ => None,
    }
}

fn timeline_forward_dto(
    result: TimelineForwardResultWire,
) -> Result<TimelineForwardDto, TimelineForwardError> {
    let action = closed_timeline_forward_action(&result.action).ok_or_else(|| {
        timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_timeline_forward_status(&result.status).ok_or_else(|| {
        timeline_forward_failed(
            TIMELINE_FORWARD_FAILED_CODE,
            TIMELINE_FORWARD_FAILED_DESCRIPTION,
        )
    })?;
    Ok(TimelineForwardDto {
        schema_version: result.schema_version,
        action: action.to_owned(),
        room_id: result.room_id,
        event_id: result.event_id,
        status: status.to_owned(),
    })
}

fn session_status_failed(code: &str, description: &'static str) -> SessionStatusError {
    SessionStatusError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

fn map_session_status_core_error(error: MatrixIpcError) -> SessionStatusError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code.starts_with("p2-session-snapshot-")
                || code.starts_with("p2-sync-status-")
                || code.starts_with("p2-media-config-")
                || code.starts_with("p2-secret-storage-status-")
                || code.starts_with("v-crypto.4-") =>
        {
            session_status_failed(code, SESSION_STATUS_OWNER_DESCRIPTION)
        }
        _ => session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ),
    }
}

fn session_status_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, SessionStatusError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

#[derive(Debug, Deserialize)]
struct SessionSnapshotResultWire {
    status: String,
    user_id: Option<String>,
    device_id: Option<String>,
    homeserver_url: Option<String>,
    #[serde(rename = "sessionGeneration")]
    session_generation: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncStatusResultWire {
    readiness: String,
    session_generation: u64,
    offline_mode_enabled: bool,
    failure_diagnostic_id: Option<String>,
    sliding_sync_capable: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct MediaConfigResultWire {
    #[serde(rename = "m.upload.size")]
    upload_size: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretStorageStatusResultWire {
    session_generation: u64,
    state: String,
    exists: bool,
    unlocked: bool,
    default_key_set: bool,
    passphrase_configured: bool,
    bootstrap_ready: bool,
    missing_secrets: Vec<String>,
    action: String,
}

fn closed_session_snapshot_status(value: &str) -> Option<&'static str> {
    match value {
        "logged_out" => Some("logged_out"),
        "logged_in" => Some("logged_in"),
        _ => None,
    }
}

fn closed_sync_readiness(value: &str) -> Option<&'static str> {
    match value {
        "unconfigured" => Some("unconfigured"),
        "idle" => Some("idle"),
        "running" => Some("running"),
        "offline" => Some("offline"),
        "terminated" => Some("terminated"),
        "failed" => Some("failed"),
        _ => None,
    }
}

fn closed_sync_failure_diagnostic(value: Option<&str>) -> Option<Option<&'static str>> {
    match value {
        None => Some(None),
        Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID) => Some(Some(SYNC_SERVICE_FAILURE_DIAGNOSTIC_ID)),
        Some(_) => None,
    }
}

fn closed_secret_storage_state(value: &str) -> Option<&'static str> {
    match value {
        "unavailable" => Some("unavailable"),
        "not_set_up" => Some("not_set_up"),
        "locked" => Some("locked"),
        "ready" => Some("ready"),
        _ => None,
    }
}

fn closed_secret_storage_action(value: &str) -> Option<&'static str> {
    match value {
        "bootstrap_required" => Some("bootstrap_required"),
        "unlock_required" => Some("unlock_required"),
        "none" => Some("none"),
        _ => None,
    }
}

fn closed_missing_secret(value: &str) -> Option<&'static str> {
    match value {
        "cross_signing_master" => Some("cross_signing_master"),
        "cross_signing_self_signing" => Some("cross_signing_self_signing"),
        "cross_signing_user_signing" => Some("cross_signing_user_signing"),
        "encryption_backup" => Some("encryption_backup"),
        _ => None,
    }
}

fn session_snapshot_dto(
    payload: serde_json::Value,
) -> Result<SessionSnapshotDto, SessionStatusError> {
    let result: SessionSnapshotResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let status = closed_session_snapshot_status(&result.status).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    match status {
        "logged_out" => Ok(SessionSnapshotDto {
            status: status.to_owned(),
            user_id: None,
            device_id: None,
            homeserver_url: None,
            session_generation: None,
        }),
        "logged_in" => {
            let user_id = result
                .user_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let device_id = result
                .device_id
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let homeserver_url = result
                .homeserver_url
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })?;
            let session_generation = result.session_generation.ok_or_else(|| {
                session_status_failed(
                    SESSION_STATUS_FAILED_CODE,
                    SESSION_STATUS_FAILED_DESCRIPTION,
                )
            })?;
            Ok(SessionSnapshotDto {
                status: status.to_owned(),
                user_id: Some(user_id),
                device_id: Some(device_id),
                homeserver_url: Some(homeserver_url),
                session_generation: Some(session_generation),
            })
        }
        _ => Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )),
    }
}

const START_OBSERVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const START_OBSERVE_POLL: std::time::Duration = std::time::Duration::from_millis(50);

fn product_live_readiness(readiness: SyncReadiness) -> bool {
    matches!(readiness, SyncReadiness::Running | SyncReadiness::Offline)
}

fn sync_start_dto_from_snapshot(snapshot: SyncReadinessSnapshot) -> SyncStartDto {
    SyncStartDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        started: product_live_readiness(snapshot.readiness),
        offline_mode_enabled: snapshot.offline_mode_enabled,
    }
}

fn sync_stop_dto_from_snapshot(snapshot: SyncReadinessSnapshot) -> SyncStopDto {
    SyncStopDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        stopped: matches!(
            snapshot.readiness,
            SyncReadiness::Idle | SyncReadiness::Terminated
        ),
        offline_mode_enabled: snapshot.offline_mode_enabled,
    }
}

async fn wait_for_started_readiness(
    owner: &SyncServiceOwner,
    mut snapshot: SyncReadinessSnapshot,
) -> SyncReadinessSnapshot {
    let deadline = tokio::time::Instant::now() + START_OBSERVE_TIMEOUT;
    while snapshot.readiness == SyncReadiness::Idle && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(START_OBSERVE_POLL).await;
        snapshot = owner.observe();
    }
    snapshot
}

fn sync_status_from_owner_snapshot(
    snapshot: SyncReadinessSnapshot,
) -> Result<SyncStatusDto, SessionStatusError> {
    if !snapshot.is_valid_public_sync_status() {
        return Err(session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        ));
    }
    Ok(SyncStatusDto {
        readiness: snapshot.readiness.as_str().to_owned(),
        session_generation: snapshot.session_generation,
        offline_mode_enabled: snapshot.offline_mode_enabled,
        failure_diagnostic_id: snapshot.failure_diagnostic_id.map(str::to_owned),
        sliding_sync_capable: snapshot.sliding_sync_capable,
    })
}

fn sync_status_dto(payload: serde_json::Value) -> Result<SyncStatusDto, SessionStatusError> {
    let result: SyncStatusResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let readiness = closed_sync_readiness(&result.readiness).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let failure_diagnostic_id =
        closed_sync_failure_diagnostic(result.failure_diagnostic_id.as_deref())
            .ok_or_else(|| {
                session_status_failed(
                    SESSION_STATUS_FAILED_CODE,
                    SESSION_STATUS_FAILED_DESCRIPTION,
                )
            })?
            .map(str::to_owned);
    Ok(SyncStatusDto {
        readiness: readiness.to_owned(),
        session_generation: result.session_generation,
        offline_mode_enabled: result.offline_mode_enabled,
        failure_diagnostic_id,
        sliding_sync_capable: result.sliding_sync_capable,
    })
}

fn media_config_dto(payload: serde_json::Value) -> Result<MediaConfigDto, SessionStatusError> {
    let result: MediaConfigResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    Ok(MediaConfigDto {
        upload_size: result.upload_size,
    })
}

fn secret_storage_status_dto(
    payload: serde_json::Value,
) -> Result<SecretStorageStatusDto, SessionStatusError> {
    let result: SecretStorageStatusResultWire = serde_json::from_value(payload).map_err(|_| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let state = closed_secret_storage_state(&result.state).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let action = closed_secret_storage_action(&result.action).ok_or_else(|| {
        session_status_failed(
            SESSION_STATUS_FAILED_CODE,
            SESSION_STATUS_FAILED_DESCRIPTION,
        )
    })?;
    let missing_secrets = result
        .missing_secrets
        .iter()
        .map(|value| {
            closed_missing_secret(value)
                .map(str::to_owned)
                .ok_or_else(|| {
                    session_status_failed(
                        SESSION_STATUS_FAILED_CODE,
                        SESSION_STATUS_FAILED_DESCRIPTION,
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SecretStorageStatusDto {
        session_generation: result.session_generation,
        state: state.to_owned(),
        exists: result.exists,
        unlocked: result.unlocked,
        default_key_set: result.default_key_set,
        passphrase_configured: result.passphrase_configured,
        bootstrap_ready: result.bootstrap_ready,
        missing_secrets,
        action: action.to_owned(),
    })
}

fn timeline_event_item_dto(item: NativeTimelineItem) -> TimelineEventItemDto {
    TimelineEventItemDto {
        item_id: item.item_id,
        event_id: item.event_id,
        sender: item.sender,
        event_type: item.event_type,
        body: item.body,
        origin_server_ts: item.origin_server_ts,
        decryption_state: item.decryption_state.map(|state| match state {
            NativeDecryptionState::Pending => "pending".to_owned(),
            NativeDecryptionState::Unavailable => "unavailable".to_owned(),
        }),
    }
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
        own_verification: match snapshot.own_verification {
            crate::app::devices::NativeOwnDeviceVerification::Unknown => "unknown",
            crate::app::devices::NativeOwnDeviceVerification::Unverified => "unverified",
            crate::app::devices::NativeOwnDeviceVerification::Verified => "verified",
        }
        .to_owned(),
        has_devices_to_verify_against: snapshot.has_devices_to_verify_against,
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
            RestoredClientSlot::Ready(_) => Err(restore_failed(
                ALREADY_RESTORED_CODE,
                ALREADY_RESTORED_DESCRIPTION,
            )),
            RestoredClientSlot::InFlight => Err(restore_failed(
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

fn leftover_reject_oversize(size: usize) -> Result<(), LeftoverCommandError> {
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(leftover_failed(
            LEFTOVER_OVERSIZE_CODE,
            LEFTOVER_OVERSIZE_DESCRIPTION,
        ));
    }
    Ok(())
}

fn leftover_status_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, LeftoverCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    leftover_reject_oversize(size)?;
    Ok(payload)
}

fn map_leftover_status_core_error(error: MatrixIpcError) -> LeftoverCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code)
            if code.ends_with("-no-session")
                || code.contains("requires-session")
                || code.contains("-session-missing") =>
        {
            leftover_failed(code, LEFTOVER_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("p2-backup-status-")
                || code.starts_with("p2-crypto-status-")
                || code.starts_with("p2-cross-signing-status-")
                || code.starts_with("p2-room-key-transfer-status-")
                || code.starts_with("v-crypto.2-")
                || code.starts_with("v-crypto.3-") =>
        {
            leftover_failed(code, LEFTOVER_UNAVAILABLE_DESCRIPTION)
        }
        _ => leftover_failed(LEFTOVER_FAILED_CODE, LEFTOVER_FAILED_DESCRIPTION),
    }
}

fn leftover_backup_status_dto(
    payload: serde_json::Value,
) -> Result<BackupStatusDto, LeftoverCommandError> {
    Ok(BackupStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        availability: payload
            .get("availability")
            .and_then(|value| value.as_str())
            .unwrap_or("missing")
            .to_owned(),
        enabled: payload
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        device_state: payload
            .get("deviceState")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
        recovery_state: payload
            .get("recoveryState")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_owned(),
        action: payload
            .get("action")
            .and_then(|value| value.as_str())
            .unwrap_or("none")
            .to_owned(),
    })
}

fn leftover_crypto_status_dto(
    payload: serde_json::Value,
) -> Result<CryptoStatusDto, LeftoverCommandError> {
    Ok(CryptoStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        encryption_enabled: payload
            .get("encryptionEnabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        cross_signing_state: payload
            .get("crossSigningState")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
    })
}

fn leftover_cross_signing_status_dto(
    payload: serde_json::Value,
) -> Result<CrossSigningStatusDto, LeftoverCommandError> {
    Ok(CrossSigningStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        readiness: payload
            .get("readiness")
            .and_then(|value| value.as_str())
            .unwrap_or("unavailable")
            .to_owned(),
    })
}

fn leftover_room_key_transfer_status_dto(
    payload: serde_json::Value,
) -> Result<RoomKeyTransferStatusDto, LeftoverCommandError> {
    Ok(RoomKeyTransferStatusDto {
        session_generation: payload
            .get("sessionGeneration")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        phase: payload
            .get("phase")
            .and_then(|value| value.as_str())
            .unwrap_or("idle")
            .to_owned(),
        keys_processed: payload
            .get("keysProcessed")
            .and_then(|value| value.as_u64())
            .unwrap_or(0) as u32,
    })
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

/// NSE access must never create or migrate a key. The containing app owns
/// Keychain mutations and publishes readiness only after the current key and
/// shared store are both available.
fn store_key_for_read_only(
    store: &Arc<dyn SecretVault + Send + Sync>,
    identity: &AccountIdentity,
) -> Result<StoreKeyMaterial, SessionRestoreError> {
    let vault = SecretStoreKeyVault {
        store: Arc::clone(store),
    };
    vault
        .get(&StoreKeyId::from_identity(identity))
        .map_err(|error| match error {
            StoreKeyVaultError::BackendUnavailable { .. } => {
                restore_failed(VAULT_UNAVAILABLE_CODE, VAULT_UNAVAILABLE_DESCRIPTION)
            }
            _ => restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION),
        })?
        .ok_or_else(|| restore_failed(RESTORE_FAILED_CODE, RESTORE_FAILED_DESCRIPTION))
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

#[derive(Clone)]
struct SecretStoreSessionVault {
    store: Arc<dyn SecretVault + Send + Sync>,
}

#[derive(Debug)]
struct SessionRotationCallbackError(&'static str);

impl std::fmt::Display for SessionRotationCallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SessionRotationCallbackError {}

/// Keep the host vault in lockstep with SDK access/refresh-token rotation.
///
/// `handle_refresh_tokens` updates the live SDK session, but persistence is an
/// application responsibility. Without these callbacks, a later relaunch can
/// restore a consumed refresh token even though the preceding run worked.
fn install_session_rotation_callbacks(
    client: &matrix_sdk::Client,
    identity: AccountIdentity,
    store: Arc<dyn SecretVault + Send + Sync>,
) -> Result<(), SessionRotationCallbackError> {
    let reload_identity = identity.clone();
    let reload_store = Arc::clone(&store);
    let save_identity = identity;
    client
        .set_session_callbacks(
            Box::new(move |_| {
                let vault = SecretStoreSessionVault {
                    store: Arc::clone(&reload_store),
                };
                let material = load_session_material(&vault, &reload_identity)
                    .map_err(|_| SessionRotationCallbackError("session-reload-read-failed"))?
                    .ok_or(SessionRotationCallbackError(
                        "session-reload-material-missing",
                    ))?;
                let secrets = material
                    .decode_host_secrets()
                    .map_err(|_| SessionRotationCallbackError("session-reload-decode-failed"))?;
                let session = matrix_session_from_host_secrets(&reload_identity, &secrets)
                    .map_err(|_| SessionRotationCallbackError("session-reload-invalid"))?;
                Ok(session.tokens)
            }),
            Box::new(move |client| {
                let vault = SecretStoreSessionVault {
                    store: Arc::clone(&store),
                };
                persist_session_after_login(&client, &save_identity, &vault)
                    .map_err(|_| SessionRotationCallbackError("session-rotation-persist-failed"))?;
                Ok(())
            }),
        )
        .map_err(|_| SessionRotationCallbackError("session-callback-install-failed"))
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

    #[tokio::test]
    async fn session_logout_forgets_credentials_when_restore_is_unavailable() {
        let entries = Arc::new(Mutex::new(HashMap::new()));
        let shared =
            SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
        let identity = alice();
        let credential_key = SessionMaterialId::from_identity(&identity)
            .account()
            .to_owned();
        let history_key = StoreKeyId::from_identity(&identity).account().to_owned();
        {
            let mut vault = entries.lock().unwrap();
            vault.insert(credential_key.clone(), b"unrestorable-session".to_vec());
            vault.insert(history_key.clone(), vec![7; 32]);
            vault.insert("other-account".into(), b"other-credential".to_vec());
        }
        assert!(!shared
            .revoke_server_session(
                identity.user_id().into(),
                "DEVICE".into(),
                identity.homeserver_url().into()
            )
            .await
            .unwrap());
        let result = shared
            .forget_session(identity.user_id().into(), identity.homeserver_url().into())
            .await
            .unwrap();
        assert_eq!(result.status, "forgotten");
        let vault = entries.lock().unwrap();
        assert!(!vault.contains_key(&credential_key));
        assert_eq!(vault.get(&history_key), Some(&vec![7; 32]));
        assert!(vault.contains_key("other-account"));
    }

    #[tokio::test]
    async fn session_logout_revokes_exact_device_and_cannot_restore_after_forget() {
        use matrix_sdk::test_utils::mocks::MatrixMockServer;
        let server = MatrixMockServer::new().await;
        server.mock_versions().ok().mount().await;
        server
            .mock_logout()
            .expect_access_token("fixture-token")
            .ok()
            .expect(1)
            .mount()
            .await;
        let entries = Arc::new(Mutex::new(HashMap::new()));
        let shared =
            SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries.clone())));
        let root = temp_root("logout");
        let root_path = root.to_string_lossy().into_owned();
        shared
            .persist_planted_session_for_test(
                "@alice:example.org".into(),
                server.server().uri(),
                root_path.clone(),
                "DEVICE".into(),
                "fixture-token".into(),
                None,
            )
            .await
            .unwrap();
        assert!(shared
            .revoke_server_session(
                "@alice:example.org".into(),
                "OTHER".into(),
                server.server().uri()
            )
            .await
            .is_err());
        assert!(shared
            .forget_session("@bob:example.org".into(), server.server().uri())
            .await
            .is_err());
        assert!(shared
            .revoke_server_session(
                "@alice:example.org".into(),
                "DEVICE".into(),
                server.server().uri()
            )
            .await
            .unwrap());
        shared
            .forget_session("@alice:example.org".into(), server.server().uri())
            .await
            .unwrap();
        assert!(shared.core.session_snapshot().unwrap().is_none());
        assert!(shared.retained_client().is_err());
        let relaunched = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(entries)));
        let error = relaunched
            .restore_persisted_session(
                "@alice:example.org".into(),
                server.server().uri(),
                root_path,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(error, SessionRestoreError::Failed { code, .. } if code == MATERIAL_MISSING_CODE)
        );
        server.verify_and_reset().await;
        fs::remove_dir_all(root).unwrap();
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
    fn session_status_oversize_payload_fails_closed_without_truncate_or_echo() {
        let marker = "s931OversizeMarker";
        let payload = serde_json::json!({
            "pad": format!("{marker}{}", "x".repeat(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8))
        });
        let error = session_status_envelope_payload(payload)
            .expect_err("oversize session/status payload must fail closed");
        let text = format!("{error:?}{error}");
        assert!(text.contains(SESSION_STATUS_FAILED_CODE));
        assert!(!text.contains(marker));
        assert!(!text.contains("syt_"));
        assert!(!text.contains("@alice"));
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
    fn verification_list_projection_preserves_display_only_sas() {
        let dto = verification_request_dto_with_sas(NativeVerificationRequest {
            flow_id: "flow".to_owned(),
            other_user_id: "@alice:example.org".to_owned(),
            other_device_id: Some("DEVICE".to_owned()),
            direction: NativeVerificationDirection::Incoming,
            phase: NativeVerificationPhase::SasReady,
            started_ts: Some(1),
            sas: Some(NativeVerificationSas {
                emoji: Some(vec![NativeVerificationEmoji {
                    symbol: "🐶".to_owned(),
                    description: "Dog".to_owned(),
                }]),
                decimals: Some([1234, 5678, 9012]),
            }),
        });

        let sas = dto.sas.expect("sas_ready list row must carry display SAS");
        assert_eq!(sas.emoji.expect("emoji")[0].symbol, "🐶");
        assert_eq!(sas.decimals, Some(vec![1234, 5678, 9012]));
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
    fn sync_stop_closes_retained_client_stores_and_start_reopens_them() {
        let identity = alice();
        let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let shared = SharedCore::new_with_secret_store(Box::new(MemoryCallbackVault(
            std::sync::Arc::clone(&values),
        )));
        let root = temp_root("sync-store-quiescence");
        let rt = test_runtime();
        let _enter = rt.enter();

        rt.block_on(shared.persist_planted_session_for_test(
            identity.user_id().to_owned(),
            identity.homeserver_url().to_owned(),
            root.to_string_lossy().into_owned(),
            "DEVICEABC".to_owned(),
            "syt_sync_store_quiescence_access".to_owned(),
            None,
        ))
        .expect("planted persist retains a SQLite-backed Client");
        rt.block_on(shared.attach_session_owners())
            .expect("attach retained session owners");
        let client = shared.retained_client().expect("retained Client");

        let stopped = rt
            .block_on(shared.stop_sync())
            .expect("stop must complete the full store quiescence boundary");
        assert!(stopped.stopped);
        let paused_store_access = rt.block_on(client.event_cache_store().lock());
        assert!(
            paused_store_access.is_err(),
            "stop_sync returned before the retained event-cache store was closed"
        );

        rt.block_on(async {
            tokio::time::timeout(std::time::Duration::from_secs(15), shared.start_sync())
                .await
                .expect("start_sync timed out")
        })
        .expect("start must resume stores before restarting SyncService");
        let resumed_store_access = rt.block_on(client.event_cache_store().lock());
        assert!(
            resumed_store_access.is_ok(),
            "start_sync did not reopen the retained event-cache store"
        );
        drop(resumed_store_access);

        rt.block_on(shared.stop_sync())
            .expect("final stop releases store resources before teardown");
        drop(client);
        drop(shared);
        drop(_enter);
        drop(rt);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nse_store_key_lookup_never_mints_a_missing_key() {
        let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let store: Arc<dyn SecretVault + Send + Sync> = Arc::new(CallbackSecretVault {
            inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&values))),
        });

        let error = store_key_for_read_only(&store, &alice()).expect_err("missing key");

        assert!(matches!(
            error,
            SessionRestoreError::Failed { ref code, .. } if code == RESTORE_FAILED_CODE
        ));
        assert!(values.lock().expect("vault").is_empty());
    }

    #[test]
    fn nse_store_key_lookup_returns_the_existing_current_key() {
        let values = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let store: Arc<dyn SecretVault + Send + Sync> = Arc::new(CallbackSecretVault {
            inner: Box::new(MemoryCallbackVault(std::sync::Arc::clone(&values))),
        });
        let identity = alice();
        let expected = StoreKeyMaterial::from_bytes([7; STORE_KEY_LEN]);
        values.lock().expect("vault").insert(
            StoreKeyId::from_identity(&identity).account().to_owned(),
            expected.as_bytes().to_vec(),
        );

        let actual = store_key_for_read_only(&store, &identity).expect("existing key");

        assert_eq!(actual.as_bytes(), expected.as_bytes());
        assert_eq!(values.lock().expect("vault").len(), 1);
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
        assert!(format!("{second:?}").contains(ALREADY_RESTORED_CODE));
        assert!(!format!("{second:?}").contains(RESTORE_FAILED_CODE));
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

    #[test]
    fn leftover_oversize_fails_closed_without_truncate_or_echo() {
        let marker = "s10OversizeMarker";
        let error = leftover_reject_oversize(MAX_ENVELOPE_PAYLOAD_JSON_BYTES + 8)
            .expect_err("oversize leftover payload must fail closed");
        let text = format!("{error:?}{error}");
        assert!(text.contains(LEFTOVER_OVERSIZE_CODE));
        assert!(!text.contains(marker));
        assert!(!text.contains("syt_"));
    }

    #[test]
    fn leftover_commands_without_session_fail_closed_without_echo() {
        let shared = SharedCore::new();
        let rt = test_runtime();
        let recovery_key = "s10-secret-recovery-key";
        let room_id = "!s10SecretRoom:example.org";
        let action_title = "s10-secret-action-title";
        let mxc = "mxc://example.org/s10SecretMedia";

        let recover = rt
            .block_on(shared.recover(recovery_key.to_owned()))
            .expect_err("recover must fail closed");
        let recover_text = format!("{recover:?}{recover}");
        assert!(recover_text.contains(LEFTOVER_UNAVAILABLE_CODE));
        assert!(!recover_text.contains(recovery_key));

        let approval = rt
            .block_on(shared.send_agent_approval(
                room_id.to_owned(),
                "approve-s10".to_owned(),
                action_title.to_owned(),
                "approve".to_owned(),
                Some("$source:example.org".to_owned()),
                1,
            ))
            .expect_err("agent approval must fail closed");
        let approval_text = format!("{approval:?}{approval}");
        assert!(approval_text.contains(AGENT_APPROVAL_NO_SESSION_CODE));
        assert!(!approval_text.contains(room_id));
        assert!(!approval_text.contains(action_title));

        let media = rt
            .block_on(shared.media_download(mxc.to_owned()))
            .expect_err("media download must fail closed");
        let media_text = format!("{media:?}{media}");
        assert!(media_text.contains(LEFTOVER_NO_SESSION_CODE));
        assert!(!media_text.contains(mxc));

        let crypto = rt
            .block_on(shared.crypto_status())
            .expect_err("crypto status must fail closed without a platform session");
        let crypto_text = format!("{crypto:?}{crypto}");
        assert!(
            crypto_text.contains("p2-crypto-status-platform-unavailable")
                || crypto_text.contains(LEFTOVER_FAILED_CODE)
        );
        assert!(!crypto_text.contains(room_id));
    }

    #[test]
    fn leftover_wipe_removes_only_the_validated_store_root() {
        let shared = SharedCore::new();
        let rt = test_runtime();
        let root = temp_root("s10-wipe");
        fs::create_dir_all(root.join("data")).unwrap();
        let ack = rt
            .block_on(shared.wipe_persisted_stores(root.to_string_lossy().into_owned()))
            .expect("validated leftover wipe");
        assert_eq!(ack.status, "wiped");
        assert!(!root.exists());
    }

    #[test]
    fn timeline_view_row_dto_maps_message_without_token_echo() {
        use crate::app::timeline::{
            TimelineEventRowBase, TimelineForwardTransport, TimelineMessageRow, TimelineReaction,
            TimelineReplyPreview, TimelineRowCapabilities, TimelineThreadSummary,
        };
        let row = TimelineViewRow::Message(Box::new(TimelineMessageRow {
            event: TimelineEventRowBase {
                item_id: "item-1".to_owned(),
                event_id: Some("$evt:example.org".to_owned()),
                sender_id: "@alice:example.org".to_owned(),
                sender_name: "Alice Example".to_owned(),
                sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
                origin_server_ts: 1_700_000_000_000,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: true,
                    edit: false,
                    redact: true,
                    report: true,
                    pin: true,
                    forward: true,
                    vote: false,
                    decline_call: false,
                },
            },
            body: "hello".to_owned(),
            formatted_body: None,
            agent_card_json: Some(r#"{"title":"Approval"}"#.to_owned()),
            is_agent_approval: true,
            message_type: Some("m.text".to_owned()),
            forward_transport: Some(TimelineForwardTransport::Text),
            media_filename: None,
            media_caption: None,
            edited: false,
            reply: Some(TimelineReplyPreview {
                event_id: "$reply:example.org".to_owned(),
                sender_id: Some("@bob:example.org".to_owned()),
                sender_name: "Bob".to_owned(),
                body: "earlier body".to_owned(),
            }),
            thread_root: Some("$root:example.org".to_owned()),
            thread: Some(TimelineThreadSummary {
                root_event_id: "$evt:example.org".to_owned(),
                reply_count: 3,
                latest_event_id: Some("$latest:example.org".to_owned()),
            }),
            reactions: vec![
                TimelineReaction {
                    key: "👍".to_owned(),
                    count: 2,
                    own: Some(true),
                },
                TimelineReaction {
                    key: "🎉".to_owned(),
                    count: 1,
                    own: None,
                },
            ],
            media: None,
        }));
        let dto = timeline_view_row_dto(row);
        assert_eq!(dto.kind, "message");
        assert_eq!(dto.item_id, "item-1");
        assert_eq!(dto.event_id, "$evt:example.org");
        assert_eq!(dto.sender_name, "Alice Example");
        assert_eq!(
            dto.sender_avatar_url.as_deref(),
            Some("mxc://example.org/alice")
        );
        assert_eq!(dto.body, "hello");
        assert_eq!(dto.message_type.as_deref(), Some("m.text"));
        assert_eq!(dto.forward_transport.as_deref(), Some("text"));
        assert_eq!(
            dto.agent_card_json.as_deref(),
            Some(r#"{"title":"Approval"}"#)
        );
        assert!(dto.is_agent_approval);
        assert_eq!(dto.reply_to_event_id.as_deref(), Some("$reply:example.org"));
        assert_eq!(
            dto.thread_root_event_id.as_deref(),
            Some("$root:example.org")
        );
        assert_eq!(
            dto.reply_preview.as_ref().map(|reply| (
                reply.sender_id.as_deref(),
                reply.sender_name.as_str(),
                reply.body.as_str()
            )),
            Some((Some("@bob:example.org"), "Bob", "earlier body"))
        );
        assert_eq!(
            dto.thread_summary.as_ref().map(|thread| (
                thread.root_event_id.as_str(),
                thread.reply_count,
                thread.latest_event_id.as_deref()
            )),
            Some(("$evt:example.org", 3, Some("$latest:example.org")))
        );
        assert_eq!(dto.reactions[0].own, Some(true));
        assert_eq!(dto.reactions[1].own, None);
        let capabilities = dto.capabilities.as_ref().expect("event capabilities");
        assert!(capabilities.reply);
        assert!(capabilities.react);
        assert!(!capabilities.vote);
        assert!(dto.poll.is_none());
        assert!(dto.media_handle_id.is_none());
        let text = format!("{dto:?}");
        assert!(!text.contains("syt_"));
        assert!(!text.contains("password"));
        assert!(text.contains("mxc://example.org/alice"));
    }

    #[test]
    fn timeline_view_row_dto_preserves_open_and_closed_poll_semantics() {
        use crate::app::timeline::{
            TimelineEventRowBase, TimelinePollAnswer, TimelinePollRow, TimelineRowCapabilities,
        };

        let make_poll = |closed: bool| {
            TimelineViewRow::Poll(TimelinePollRow {
                event: TimelineEventRowBase {
                    item_id: if closed { "closed-poll" } else { "open-poll" }.to_owned(),
                    event_id: Some("$poll:example.org".to_owned()),
                    sender_id: "@alice:example.org".to_owned(),
                    sender_name: "Alice".to_owned(),
                    sender_avatar_url: None,
                    origin_server_ts: 1_700_000_000_002,
                    capabilities: TimelineRowCapabilities {
                        react: true,
                        reply: false,
                        edit: false,
                        redact: true,
                        report: true,
                        pin: true,
                        forward: false,
                        vote: !closed,
                        decline_call: false,
                    },
                },
                question: "Choose two".to_owned(),
                closed,
                max_selections: 2,
                answers: vec![
                    TimelinePollAnswer {
                        id: "a".to_owned(),
                        text: "Alpha".to_owned(),
                        vote_count: 4,
                        own: true,
                    },
                    TimelinePollAnswer {
                        id: "b".to_owned(),
                        text: "Beta".to_owned(),
                        vote_count: 1,
                        own: false,
                    },
                ],
                reply: Some(TimelineReplyPreview {
                    event_id: "$poll-reply:example.org".to_owned(),
                    sender_id: None,
                    sender_name: "Message".to_owned(),
                    body: "Jump to original".to_owned(),
                }),
                thread_root: Some("$poll-root:example.org".to_owned()),
                thread: None,
                reactions: vec![TimelineReaction {
                    key: "👍".to_owned(),
                    count: 2,
                    own: Some(true),
                }],
            })
        };

        let open = timeline_view_row_dto(make_poll(false));
        let open_poll = open.poll.expect("open poll presentation");
        assert_eq!(open.body, "Choose two");
        assert!(!open_poll.closed);
        assert_eq!(open_poll.max_selections, 2);
        assert_eq!(open_poll.answers.len(), 2);
        assert!(open_poll.answers[0].own);
        assert_eq!(open_poll.answers[0].vote_count, 4);
        assert!(open.capabilities.expect("open capabilities").vote);
        assert_eq!(
            open.reply_to_event_id.as_deref(),
            Some("$poll-reply:example.org")
        );
        assert_eq!(
            open.thread_root_event_id.as_deref(),
            Some("$poll-root:example.org")
        );
        assert_eq!(open.reactions[0].own, Some(true));
        assert!(open.thread_summary.is_none());

        let closed = timeline_view_row_dto(make_poll(true));
        assert!(closed.poll.expect("closed poll presentation").closed);
        assert!(!closed.capabilities.expect("closed capabilities").vote);
    }

    #[test]
    fn timeline_view_row_dto_preserves_incoming_sticker_media() {
        use crate::app::timeline::{
            TimelineEventRowBase, TimelineForwardTransport, TimelineMediaHandle,
            TimelineRowCapabilities,
        };

        let row = TimelineViewRow::Sticker {
            event: TimelineEventRowBase {
                item_id: "sticker-item".to_owned(),
                event_id: Some("$sticker:example.org".to_owned()),
                sender_id: "@alice:example.org".to_owned(),
                sender_name: "Alice".to_owned(),
                sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
                origin_server_ts: 1_700_000_000_001,
                capabilities: TimelineRowCapabilities {
                    react: true,
                    reply: true,
                    edit: false,
                    redact: true,
                    report: true,
                    pin: true,
                    forward: true,
                    vote: false,
                    decline_call: false,
                },
            },
            media: TimelineMediaHandle {
                handle_id: "incoming-sticker-handle".to_owned(),
                mime_type: Some("image/webp".to_owned()),
                width: Some(256),
                height: Some(128),
                duration_ms: None,
            },
            forward_transport: TimelineForwardTransport::Media,
            reply: None,
            thread_root: Some("$sticker-root:example.org".to_owned()),
            thread: None,
            reactions: vec![TimelineReaction {
                key: "🎉".to_owned(),
                count: 3,
                own: Some(false),
            }],
        };

        let dto = timeline_view_row_dto(row);
        assert_eq!(dto.kind, "sticker");
        assert_eq!(dto.event_id, "$sticker:example.org");
        assert_eq!(dto.message_type.as_deref(), Some("m.sticker"));
        assert_eq!(dto.forward_transport.as_deref(), Some("media"));
        assert_eq!(
            dto.thread_root_event_id.as_deref(),
            Some("$sticker-root:example.org")
        );
        assert_eq!(dto.reactions[0].key, "🎉");
        assert_eq!(dto.reactions[0].own, Some(false));
        assert_eq!(
            dto.media_handle_id.as_deref(),
            Some("incoming-sticker-handle")
        );
        assert_eq!(dto.media_mime_type.as_deref(), Some("image/webp"));
        assert_eq!(dto.media_width, Some(256));
        assert_eq!(dto.media_height, Some(128));
        assert_eq!(dto.sender, "@alice:example.org");
        assert_eq!(
            dto.sender_avatar_url.as_deref(),
            Some("mxc://example.org/alice")
        );
    }

    #[test]
    fn timeline_view_row_dto_preserves_base_metadata_for_non_message_events() {
        use crate::app::timeline::{
            TimelineEncryptedUnavailableRow, TimelineEventRowBase, TimelineOtherRow,
            TimelineRedactedRow, TimelineRowCapabilities,
        };

        let base = |item_id: &str, event_id: &str| TimelineEventRowBase {
            item_id: item_id.to_owned(),
            event_id: Some(event_id.to_owned()),
            sender_id: "@alice:example.org".to_owned(),
            sender_name: "Alice".to_owned(),
            sender_avatar_url: Some("mxc://example.org/alice".to_owned()),
            origin_server_ts: 1_700_000_000_003,
            capabilities: TimelineRowCapabilities {
                react: false,
                reply: false,
                edit: false,
                redact: true,
                report: true,
                pin: true,
                forward: false,
                vote: false,
                decline_call: false,
            },
        };
        let assert_base = |dto: &TimelineViewRowDto, event_id: &str| {
            assert_eq!(dto.event_id, event_id);
            assert_eq!(dto.sender, "@alice:example.org");
            assert_eq!(dto.sender_name, "Alice");
            assert_eq!(
                dto.sender_avatar_url.as_deref(),
                Some("mxc://example.org/alice")
            );
            assert_eq!(dto.origin_server_ts, 1_700_000_000_003);
            let capabilities = dto.capabilities.as_ref().expect("event capabilities");
            assert!(capabilities.redact);
            assert!(capabilities.report);
        };

        let redacted = timeline_view_row_dto(TimelineViewRow::Redacted(TimelineRedactedRow {
            event: base("redacted-item", "$redacted:example.org"),
            summary: "Message removed".to_owned(),
        }));
        assert_eq!(redacted.kind, "redacted");
        assert_base(&redacted, "$redacted:example.org");

        let encrypted = timeline_view_row_dto(TimelineViewRow::EncryptedUnavailable(
            TimelineEncryptedUnavailableRow {
                event: base("encrypted-item", "$encrypted:example.org"),
                reason_code: "unable_to_decrypt".to_owned(),
            },
        ));
        assert_eq!(encrypted.kind, "encrypted");
        assert_base(&encrypted, "$encrypted:example.org");

        let other_base = base("other-item", "$other:example.org");
        let other = timeline_view_row_dto(TimelineViewRow::Other(TimelineOtherRow {
            item_id: other_base.item_id.clone(),
            event_id: other_base.event_id.clone(),
            event: Some(other_base),
            event_type: Some("org.example.unknown".to_owned()),
            forward_transport: None,
            summary: "Unsupported timeline event".to_owned(),
        }));
        assert_eq!(other.kind, "other");
        assert_base(&other, "$other:example.org");
        assert_eq!(other.message_type.as_deref(), Some("org.example.unknown"));
    }
}
