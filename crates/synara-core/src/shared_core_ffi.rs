//! SharedCore UniFFI facade and domain module wiring.
//!
//! Domain modules own typed operations and closed DTO/error projections.
//! Credentials use dedicated zeroizing arguments; generic command envelopes
//! never carry recovery secrets. Narrow NSE product builds use synara-nse-core.

mod agent_notification_preferences;
pub use agent_notification_preferences::*;
mod approval_inbox;
pub use approval_inbox::{
    AgentApprovalInboxDto, AgentApprovalInboxError, AgentApprovalInboxItemDto,
};
mod agent_approval_history;
pub use agent_approval_history::{
    AgentApprovalHistoryCommandError, AgentApprovalHistoryItemDto, AgentApprovalHistorySnapshotDto,
};
mod inbox_notifications;
pub use inbox_notifications::{
    InboxNotificationDto, InboxNotificationsError, InboxNotificationsPageDto,
};

use std::path::{Component, Path};
use std::sync::{Arc, Mutex};

use matrix_sdk::store::RoomLoadSettings;
use matrix_sdk::Client;
use zeroize::Zeroizing;

use crate::app::account_data::{
    NativeAccountDataWakeupKind, NativeGlobalImagePacksSnapshot, NativeImagePack,
    NativeImagePackOwner, NativeImagePackUpdateSignal, NativeLaterSnapshot, NativeMDirectSnapshot,
    NativeRoomImagePacksSnapshot, NativeRoomNotesSnapshot, NativeUserImagePackSnapshot,
    RoomNoteMoveDirection, SynaraLaterItem, SynaraLaterItemKind, SynaraRoomNoteItem,
    SynaraRoomNoteItemKind,
};
use crate::app::auth::{
    existing_sqlite_crypto_device_id, login_with_password as core_login_with_password,
    DevicePlatform, LoginOptions,
};
use crate::app::client_builder::{build_unauthenticated_client, ClientBuildConfig};
use crate::app::dehydrated_devices::NativeDehydratedDevicesOwner;
use crate::app::devices::{
    NativeDeviceDeleteAuthentication, NativeDeviceDeleteResult, NativeDeviceOwner,
    NativeDeviceSnapshot, NativeDeviceTrust, NativeDeviceUpdateSignal,
};
use crate::app::lifecycle::{
    persist_session_after_login, restore_session_from_vault,
    restore_session_from_vault_with_room_load_settings, restore_session_onto_client,
    SessionMaterial, SessionMaterialId, SessionMaterialVault,
};
use crate::app::media_cache::NativeMediaRetentionOwner;
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
use crate::app::rtc_transports::{
    NativeRtcTransport, NativeRtcTransportsOwner, NativeRtcTransportsSnapshot,
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
use crate::app::user_profile::{NativeOwnProfileOwner, OwnProfileUpdateEmit};
use crate::app::user_status::{
    NativeInCall, NativeUserStatus, NativeUserStatusOwner, NativeUserStatusSnapshot,
    NativeUserStatusWriteResult,
};
use crate::app::verification::{
    NativeVerificationDirection, NativeVerificationEmoji, NativeVerificationInbox,
    NativeVerificationOwner, NativeVerificationPhase, NativeVerificationQr,
    NativeVerificationRequest, NativeVerificationSas, NativeVerificationUpdateSignal,
};
use crate::core::Core;
use crate::dto::{SessionLifecycle, SessionSnapshot};
use crate::platform::{IosFailClosedPlatform, Platform, SecretVault};
use crate::transport::{MatrixIpcError, MatrixIpcErrorCategory, MAX_ENVELOPE_PAYLOAD_JSON_BYTES};

/// Whether `request`'s wire JSON fits the envelope cap that Core enforces on
/// shell payloads; oversized writes fail closed before reaching an owner.
pub(crate) fn within_envelope_cap<T: serde::Serialize>(request: &T) -> bool {
    serde_json::to_vec(request).is_ok_and(|json| json.len() <= MAX_ENVELOPE_PAYLOAD_JSON_BYTES)
}
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
const TIMELINE_VIEW_POLL_FAILED_CODE: &str = "p4-s14-timeline-view-poll-failed";
const TIMELINE_VIEW_POLL_FAILED_DESCRIPTION: &str = "Timeline view updates could not be polled.";
const TIMELINE_VIEW_UPDATE_QUEUE_CAP: usize = 32;
const OWNER_UPDATE_POLL_FAILED_CODE: &str = "p4-s17-owner-update-poll-failed";
const OWNER_UPDATE_POLL_FAILED_DESCRIPTION: &str = "Owner updates could not be polled.";
const OWNER_UPDATE_QUEUE_CAP: usize = 32;
const ROOM_LIST_UPDATE_POLL_FAILED_CODE: &str = "p4-s19-room-list-update-poll-failed";
const ROOM_LIST_UPDATE_POLL_FAILED_DESCRIPTION: &str = "Room list updates could not be polled.";
const ROOM_LIST_UPDATE_QUEUE_CAP: usize = 32;
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
const ATTACHED_OWNER_NAMES: &[&str] = &[
    "typing",
    "presence",
    "rtc_transports",
    "user_status",
    "verification",
    "devices",
    "dehydrated_devices",
    "join_rules",
    "image_packs",
    "http_pusher",
    "timelines",
    "sync",
];
const ROOM_LIST_NO_SESSION_CODE: &str = "p2-room-list-snapshot-no-session";
const ROOM_LIST_NO_SESSION_DESCRIPTION: &str = "No room list session is available.";
const ROOM_LIST_SYNC_NOT_STARTED_CODE: &str = "p4-s4-sync-not-started";
const ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION: &str = "The room list is not live.";
const ROOM_LIST_FAILED_CODE: &str = "p4-s4-snapshot-failed";
const ROOM_LIST_FAILED_DESCRIPTION: &str = "The room list could not be loaded.";
const INVITES_NO_SESSION_CODE: &str = "p2-invites-snapshot-no-session";
const INVITES_NO_SESSION_DESCRIPTION: &str = "No invite session is available.";
const INVITES_FAILED_CODE: &str = "p4-s5-snapshot-failed";
const INVITES_FAILED_DESCRIPTION: &str = "The invite inbox could not be loaded.";
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
const RTC_TRANSPORTS_NO_SESSION_CODE: &str = "p2-rtc-transports-snapshot-no-session";
const RTC_TRANSPORTS_REFRESH_NO_SESSION_CODE: &str = "p2-rtc-transports-refresh-no-session";
const RTC_TRANSPORTS_NO_SESSION_DESCRIPTION: &str = "No MatrixRTC transport session is available.";
const RTC_TRANSPORTS_FAILED_CODE: &str = "p4-rtc-transports-snapshot-failed";
const RTC_TRANSPORTS_FAILED_DESCRIPTION: &str = "MatrixRTC transports could not be loaded.";
const USER_STATUS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-user-status-snapshot-no-session";
const USER_STATUS_SET_NO_SESSION_CODE: &str = "p2-user-status-set-no-session";
const USER_STATUS_CLEAR_NO_SESSION_CODE: &str = "p2-user-status-clear-no-session";
const USER_STATUS_NO_SESSION_DESCRIPTION: &str = "No user status session is available.";
const USER_STATUS_FAILED_CODE: &str = "p4-user-status-failed";
const USER_STATUS_FAILED_DESCRIPTION: &str = "User status could not be updated.";
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
const VERIFICATION_LIST_NO_SESSION_CODE: &str = "p2-verification-list-no-session";
const VERIFICATION_LIST_NO_SESSION_DESCRIPTION: &str = "No verification session is available.";
const VERIFICATION_LIST_FAILED_CODE: &str = "p4-s8-list-failed";
const VERIFICATION_LIST_FAILED_DESCRIPTION: &str = "The verification inbox could not be loaded.";
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
const DEVICE_DELETE_PASSWORD_NO_SESSION_CODE: &str = "p2-device-delete-password-no-session";
const DEVICE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-device-snapshot-no-session";
const DEVICE_RENAME_NO_SESSION_CODE: &str = "p2-device-rename-no-session";
const DEVICE_DELETE_START_NO_SESSION_CODE: &str = "p2-device-delete-start-no-session";
const DEVICE_DELETE_CANCEL_NO_SESSION_CODE: &str = "p2-device-delete-cancel-no-session";
const DEVICE_NO_SESSION_DESCRIPTION: &str = "No device session is available.";
const DEVICE_FAILED_CODE: &str = "p4-s9-2-device-failed";
const DEVICE_FAILED_DESCRIPTION: &str = "The device request could not be completed.";
const DEVICE_OWNER_DESCRIPTION: &str = "The device request is not available.";
const JOIN_RULE_SNAPSHOT_NO_SESSION_CODE: &str = "p2-join-rule-snapshot-no-session";
const JOIN_RULE_SET_NO_SESSION_CODE: &str = "p2-room-set-join-rule-no-session";
const JOIN_RULE_NO_SESSION_DESCRIPTION: &str = "No join-rule session is available.";
const JOIN_RULE_FAILED_CODE: &str = "p4-s9-3-join-rule-failed";
const JOIN_RULE_FAILED_DESCRIPTION: &str = "The join-rule request could not be completed.";
const JOIN_RULE_OWNER_DESCRIPTION: &str = "The join-rule request is not available.";
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
const MDIRECT_SNAPSHOT_NO_SESSION_CODE: &str = "p2-mdirect-snapshot-no-session";
const MDIRECT_ADD_NO_SESSION_CODE: &str = "p2-mdirect-add-no-session";
const MDIRECT_REMOVE_NO_SESSION_CODE: &str = "p2-mdirect-remove-no-session";
const MDIRECT_NO_SESSION_DESCRIPTION: &str = "No m.direct session is available.";
const MDIRECT_FAILED_CODE: &str = "p4-s9-6-mdirect-failed";
const MDIRECT_FAILED_DESCRIPTION: &str = "The m.direct request could not be completed.";
const MDIRECT_OWNER_DESCRIPTION: &str = "The m.direct request is not available.";
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
const SET_OWN_DISPLAY_NAME_NO_SESSION_CODE: &str = "p2-set-own-display-name-no-session";
const SET_OWN_AVATAR_NO_SESSION_CODE: &str = "p2-set-own-avatar-no-session";
const GET_OWN_PROFILE_NO_SESSION_CODE: &str = "p2-get-own-profile-no-session";
const OWN_PROFILE_NO_SESSION_DESCRIPTION: &str = "No own-profile session is available.";
const OWN_PROFILE_FAILED_CODE: &str = "p4-s9-8-own-profile-failed";
const OWN_PROFILE_FAILED_DESCRIPTION: &str = "The own-profile request could not be completed.";
const OWN_PROFILE_OWNER_DESCRIPTION: &str = "The own-profile request is not available.";
const IGNORED_USERS_SNAPSHOT_NO_SESSION_CODE: &str = "p2-ignored-users-snapshot-no-session";
const IGNORED_USERS_IGNORE_NO_SESSION_CODE: &str = "p2-ignored-users-ignore-no-session";
const IGNORED_USERS_UNIGNORE_NO_SESSION_CODE: &str = "p2-ignored-users-unignore-no-session";
const IGNORED_USERS_NO_SESSION_DESCRIPTION: &str = "No ignored-users session is available.";
const IGNORED_USERS_FAILED_CODE: &str = "p4-s9-ignored-users-failed";
const IGNORED_USERS_FAILED_DESCRIPTION: &str = "The ignored-users request could not be completed.";
const IGNORED_USERS_OWNER_DESCRIPTION: &str = "The ignored-users request is not available.";
const USER_DIRECTORY_SEARCH_NO_SESSION_CODE: &str = "p2-user-directory-search-no-session";
const USER_DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION: &str =
    "No user-directory session is available.";
const USER_DIRECTORY_SEARCH_FAILED_CODE: &str = "p4-s9-user-directory-search-failed";
const USER_DIRECTORY_SEARCH_FAILED_DESCRIPTION: &str =
    "The user-directory search request could not be completed.";
const USER_DIRECTORY_SEARCH_OWNER_DESCRIPTION: &str =
    "The user-directory search request is not available.";
const MESSAGE_SEARCH_NO_SESSION_CODE: &str = "p2-message-search-no-session";
const MESSAGE_SEARCH_NO_SESSION_DESCRIPTION: &str = "No message-search session is available.";
const MESSAGE_SEARCH_FAILED_CODE: &str = "p4-s9-message-search-failed";
const MESSAGE_SEARCH_FAILED_DESCRIPTION: &str =
    "The message-search request could not be completed.";
const MESSAGE_SEARCH_OWNER_DESCRIPTION: &str = "The message-search request is not available.";
const PUSH_RULES_SNAPSHOT_NO_SESSION_CODE: &str = "p2-push-rules-snapshot-no-session";
const PUSH_RULES_SET_DEFAULT_NO_SESSION_CODE: &str = "p2-push-rules-set-default-no-session";
const PUSH_RULES_SET_MENTION_NO_SESSION_CODE: &str = "p2-push-rules-set-mention-no-session";
const PUSH_RULES_ADD_KEYWORD_NO_SESSION_CODE: &str = "p2-push-rules-add-keyword-no-session";
const PUSH_RULES_REMOVE_KEYWORD_NO_SESSION_CODE: &str = "p2-push-rules-remove-keyword-no-session";
const PUSH_RULES_NO_SESSION_DESCRIPTION: &str = "No push-rules session is available.";
const PUSH_RULES_FAILED_CODE: &str = "p4-s9-push-rules-failed";
const PUSH_RULES_FAILED_DESCRIPTION: &str = "The push-rules request could not be completed.";
const PUSH_RULES_OWNER_DESCRIPTION: &str = "The push-rules request is not available.";
const ROOM_NOTIFICATION_SNAPSHOT_NO_SESSION_CODE: &str = "p2-room-notification-snapshot-no-session";
const ROOM_NOTIFICATION_SET_NO_SESSION_CODE: &str = "p2-room-notification-set-no-session";
const ROOM_NOTIFICATIONS_SNAPSHOT_NO_SESSION_CODE: &str =
    "p2-room-notifications-snapshot-no-session";
const ROOM_NOTIFICATION_NO_SESSION_DESCRIPTION: &str = "No room-notification session is available.";
const ROOM_NOTIFICATION_FAILED_CODE: &str = "p4-s9-room-notification-failed";
const ROOM_NOTIFICATION_FAILED_DESCRIPTION: &str =
    "The room-notification request could not be completed.";
const ROOM_NOTIFICATION_OWNER_DESCRIPTION: &str = "The room-notification request is not available.";
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
const SET_ROOM_NAME_NO_SESSION_CODE: &str = "p2-set-room-name-no-session";
const SET_ROOM_TOPIC_NO_SESSION_CODE: &str = "p2-set-room-topic-no-session";
const SET_ROOM_AVATAR_NO_SESSION_CODE: &str = "p2-set-room-avatar-no-session";
const ROOM_PROFILE_NO_SESSION_DESCRIPTION: &str = "No room-profile session is available.";
const ROOM_PROFILE_FAILED_CODE: &str = "p4-s9-9-room-profile-failed";
const ROOM_PROFILE_FAILED_DESCRIPTION: &str = "The room-profile request could not be completed.";
const ROOM_PROFILE_OWNER_DESCRIPTION: &str = "The room-profile request is not available.";
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
const ROOM_LEAVE_NO_SESSION_CODE: &str = "p2-room-leave-no-session";
const ROOM_JOIN_NO_SESSION_CODE: &str = "p2-room-join-no-session";
const ROOM_SET_FAVORITE_NO_SESSION_CODE: &str = "p2-room-set-favorite-no-session";
const ROOM_MEMBERSHIP_NO_SESSION_DESCRIPTION: &str = "No room-membership session is available.";
const ROOM_MEMBERSHIP_FAILED_CODE: &str = "p4-s9-12-room-membership-failed";
const ROOM_MEMBERSHIP_FAILED_DESCRIPTION: &str =
    "The room-membership request could not be completed.";
const ROOM_MEMBERSHIP_OWNER_DESCRIPTION: &str = "The room-membership request is not available.";
const ROOM_INVITE_NO_SESSION_CODE: &str = "p2-room-invite-no-session";
const ROOM_KICK_NO_SESSION_CODE: &str = "p2-room-kick-no-session";
const ROOM_BAN_NO_SESSION_CODE: &str = "p2-room-ban-no-session";
const ROOM_UNBAN_NO_SESSION_CODE: &str = "p2-room-unban-no-session";
const ROOM_MODERATION_NO_SESSION_DESCRIPTION: &str = "No room-moderation session is available.";
const ROOM_MODERATION_FAILED_CODE: &str = "p4-s9-13-room-moderation-failed";
const ROOM_MODERATION_FAILED_DESCRIPTION: &str =
    "The room-moderation request could not be completed.";
const ROOM_MODERATION_OWNER_DESCRIPTION: &str = "The room-moderation request is not available.";
const ROOM_SET_POWER_LEVEL_NO_SESSION_CODE: &str = "p2-room-set-power-level-no-session";
const ROOM_SET_POWER_LEVELS_NO_SESSION_CODE: &str = "p2-room-set-power-levels-no-session";
const ROOM_SET_POWER_LEVEL_TAGS_NO_SESSION_CODE: &str = "p2-room-set-power-level-tags-no-session";
const ROOM_POWER_LEVEL_NO_SESSION_DESCRIPTION: &str = "No room-power-level session is available.";
const ROOM_POWER_LEVEL_FAILED_CODE: &str = "p4-s9-14-room-power-levels-failed";
const ROOM_POWER_LEVEL_FAILED_DESCRIPTION: &str =
    "The room-power-level request could not be completed.";
const ROOM_POWER_LEVEL_OWNER_DESCRIPTION: &str = "The room-power-level request is not available.";
const ROOM_CREATE_NO_SESSION_CODE: &str = "p2-room-create-no-session";
const ROOM_CREATE_NO_SESSION_DESCRIPTION: &str = "No room-create session is available.";
const ROOM_CREATE_FAILED_CODE: &str = "p4-s9-15-room-create-failed";
const ROOM_CREATE_FAILED_DESCRIPTION: &str = "The room-create request could not be completed.";
const ROOM_CREATE_OWNER_DESCRIPTION: &str = "The room-create request is not available.";
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
const INVITES_ACCEPT_NO_SESSION_CODE: &str = "p2-invites-accept-no-session";
const INVITES_DECLINE_NO_SESSION_CODE: &str = "p2-invites-decline-no-session";
const INVITES_REPORT_SPAM_NO_SESSION_CODE: &str = "p2-invites-report-spam-no-session";
const INVITES_BLOCK_SENDER_NO_SESSION_CODE: &str = "p2-invites-block-sender-no-session";
const INVITE_ACTION_NO_SESSION_DESCRIPTION: &str = "No invite-action session is available.";
const INVITE_ACTION_FAILED_CODE: &str = "p4-s9-18-invite-actions-failed";
const INVITE_ACTION_FAILED_DESCRIPTION: &str = "The invite action could not be completed.";
const INVITE_ACTION_OWNER_DESCRIPTION: &str = "The invite action is not available.";
const TIMELINE_EVENT_READBACK_NO_SESSION_CODE: &str = "p2-timeline-event-readback-no-session";
const TIMELINE_SET_READ_STATE_NO_SESSION_CODE: &str = "p2-timeline-set-read-state-no-session";
const TIMELINE_JUMP_LATEST_NO_SESSION_CODE: &str = "p2-timeline-jump-latest-no-session";
const TIMELINE_READ_STATE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_READ_STATE_FAILED_CODE: &str = "p4-s9-19-timeline-read-state-failed";
const TIMELINE_READ_STATE_FAILED_DESCRIPTION: &str =
    "The timeline read-state request could not be completed.";
const TIMELINE_READ_STATE_OWNER_DESCRIPTION: &str =
    "The timeline read-state request is not available.";
const REACTION_ENSURE_NO_SESSION_CODE: &str = "p2-reaction-ensure-no-session";
const REACTION_REDACT_NO_SESSION_CODE: &str = "p2-reaction-redact-no-session";
const TIMELINE_REACTION_TOGGLE_NO_SESSION_CODE: &str = "p2-timeline-reaction-toggle-no-session";
const TIMELINE_REACTION_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_REACTION_FAILED_CODE: &str = "p4-s9-20-timeline-reactions-failed";
const TIMELINE_REACTION_FAILED_DESCRIPTION: &str =
    "The timeline reaction request could not be completed.";
const TIMELINE_REACTION_OWNER_DESCRIPTION: &str = "The timeline reaction request is not available.";
const COMPOSER_SET_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-set-reply-draft-no-session";
const COMPOSER_GET_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-get-reply-draft-no-session";
const COMPOSER_CLEAR_REPLY_DRAFT_NO_SESSION_CODE: &str = "p2-composer-clear-reply-draft-no-session";
const COMPOSER_REPLY_DRAFT_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const COMPOSER_REPLY_DRAFT_FAILED_CODE: &str = "p4-s9-21-composer-reply-draft-failed";
const COMPOSER_REPLY_DRAFT_FAILED_DESCRIPTION: &str =
    "The composer reply-draft request could not be completed.";
const COMPOSER_REPLY_DRAFT_OWNER_DESCRIPTION: &str =
    "The composer reply-draft request is not available.";
const SEND_TEXT_NO_SESSION_CODE: &str = "p2-send-text-no-session";
const SEND_TEXT_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const SEND_TEXT_FAILED_CODE: &str = "p4-s9-22-send-text-failed";
const SEND_TEXT_FAILED_DESCRIPTION: &str = "The send-text request could not be completed.";
const SEND_TEXT_OWNER_DESCRIPTION: &str = "The send-text request is not available.";
const SEND_POLL_NO_SESSION_CODE: &str = "p2-send-poll-no-session";
const SEND_POLL_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const SEND_POLL_FAILED_CODE: &str = "p4-s9-24-send-poll-failed";
const SEND_POLL_FAILED_DESCRIPTION: &str = "The send-poll request could not be completed.";
const SEND_POLL_OWNER_DESCRIPTION: &str = "The send-poll request is not available.";
const EDIT_MESSAGE_NO_SESSION_CODE: &str = "p2-edit-message-no-session";
const EDIT_MESSAGE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const EDIT_MESSAGE_FAILED_CODE: &str = "p4-s9-25-edit-message-failed";
const EDIT_MESSAGE_FAILED_DESCRIPTION: &str = "The edit-message request could not be completed.";
const EDIT_MESSAGE_OWNER_DESCRIPTION: &str = "The edit-message request is not available.";
const POLL_RESPOND_NO_SESSION_CODE: &str = "p2-poll-respond-no-session";
const POLL_RESPOND_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const POLL_RESPOND_FAILED_CODE: &str = "p4-s9-26-poll-respond-failed";
const POLL_RESPOND_FAILED_DESCRIPTION: &str = "The poll-respond request could not be completed.";
const POLL_RESPOND_OWNER_DESCRIPTION: &str = "The poll-respond request is not available.";
const TIMELINE_EDIT_TEXT_NO_SESSION_CODE: &str = "p2-timeline-edit-text-no-session";
const TIMELINE_REDACT_NO_SESSION_CODE: &str = "p2-timeline-redact-no-session";
const TIMELINE_REPORT_NO_SESSION_CODE: &str = "p2-timeline-report-no-session";
const TIMELINE_MUTATE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_MUTATE_FAILED_CODE: &str = "p4-s9-27-timeline-mutate-failed";
const TIMELINE_MUTATE_FAILED_DESCRIPTION: &str =
    "The timeline mutation request could not be completed.";
const TIMELINE_MUTATE_OWNER_DESCRIPTION: &str = "The timeline mutation request is not available.";
const TIMELINE_PIN_NO_SESSION_CODE: &str = "p2-timeline-pin-no-session";
const TIMELINE_UNPIN_NO_SESSION_CODE: &str = "p2-timeline-unpin-no-session";
const TIMELINE_PIN_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_PIN_FAILED_CODE: &str = "p4-s9-28-timeline-pin-failed";
const TIMELINE_PIN_FAILED_DESCRIPTION: &str = "The timeline pin request could not be completed.";
const TIMELINE_PIN_OWNER_DESCRIPTION: &str = "The timeline pin request is not available.";
const TIMELINE_POLL_VOTE_NO_SESSION_CODE: &str = "p2-timeline-poll-vote-no-session";
const TIMELINE_CALL_DECLINE_NO_SESSION_CODE: &str = "p2-timeline-call-decline-no-session";
const TIMELINE_VOTE_DECLINE_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_VOTE_DECLINE_FAILED_CODE: &str = "p4-s9-29-timeline-vote-decline-failed";
const TIMELINE_VOTE_DECLINE_FAILED_DESCRIPTION: &str =
    "The timeline vote or decline request could not be completed.";
const TIMELINE_VOTE_DECLINE_OWNER_DESCRIPTION: &str =
    "The timeline vote or decline request is not available.";
const TIMELINE_FORWARD_TEXT_NO_SESSION_CODE: &str = "p2-timeline-forward-text-no-session";
const TIMELINE_FORWARD_MEDIA_NO_SESSION_CODE: &str = "p2-timeline-forward-media-no-session";
const TIMELINE_FORWARD_NO_SESSION_DESCRIPTION: &str = "No timeline session is available.";
const TIMELINE_FORWARD_FAILED_CODE: &str = "p4-s9-30-timeline-forward-failed";
const TIMELINE_FORWARD_FAILED_DESCRIPTION: &str =
    "The timeline forward request could not be completed.";
const TIMELINE_FORWARD_OWNER_DESCRIPTION: &str = "The timeline forward request is not available.";
const SESSION_STATUS_FAILED_CODE: &str = "p4-s9-31-session-status-failed";
const SESSION_STATUS_FAILED_DESCRIPTION: &str =
    "The session or status request could not be completed.";
const SESSION_STATUS_OWNER_DESCRIPTION: &str = "The session or status request is not available.";

/// Retained shared Core for the iOS UniFFI boundary.
#[derive(uniffi::Object)]
pub struct SharedCore {
    core: Core,
    secret_store: Arc<dyn SecretVault + Send + Sync>,
    restored_client: Mutex<RestoredClientSlot>,
    owner_attach: Mutex<OwnerAttachSlot>,
    /// Serializes the complete SyncService/Client lifecycle transaction.
    /// Shell-side ordering remains useful, but the persistence boundary must
    /// remain correct for every current and future FFI caller.
    sync_lifecycle: tokio::sync::Mutex<()>,
    timeline_view_updates: Arc<Mutex<Vec<TimelineViewDeltaBatch>>>,
    owner_updates: Arc<Mutex<Vec<OwnerUpdateDto>>>,
    room_list_updates: Arc<Mutex<Vec<RoomListUpdateDto>>>,
    room_list_live: Arc<Mutex<Option<NativeRoomListOwner>>>,
    own_profile_live: Arc<Mutex<Option<NativeOwnProfileOwner>>>,
    media_retention_live: Arc<Mutex<Option<NativeMediaRetentionOwner>>>,
    /// Monotonic per-instance session generation. A re-login in the same
    /// process must not reuse the generation a retired session carried.
    generations: crate::app::lifecycle::session::SessionGenerations,
    /// Backoff for re-saving tokens after a failed rotation save.
    save_retry_backoff: Mutex<crate::app::lifecycle::session::RetryBackoff>,
    /// The generation (and account) whose sync reported a rejected refresh.
    /// Latched when observed so retirement does not depend on the live sync
    /// owner still existing, or still reporting, after the shell stops it.
    rejected_session: Mutex<Option<(u64, AccountIdentity)>>,
}

impl Default for SharedCore {
    fn default() -> Self {
        Self::new()
    }
}

const START_OBSERVE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const START_OBSERVE_POLL: std::time::Duration = std::time::Duration::from_millis(50);

// Typed FFI domains preserve the public facade while keeping each operation,
// projection, and closed error conversion beside its owner boundary.
mod account_data;
pub use account_data::*;
mod media;
pub use media::*;
mod messaging;
pub use messaging::*;
mod store_keys;
use store_keys::*;

/// The serde label of a closed Core enum (`snake_case`), for string DTO fields.
pub(super) fn wire_label<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(label)) => label,
        _ => String::new(),
    }
}
mod profile_search;
pub use profile_search::*;
mod push_preferences;
pub use push_preferences::*;
mod realtime;
pub use realtime::*;
mod recovery;
pub use recovery::*;
mod room_directory;
pub use room_directory::*;
mod rooms;
pub use rooms::*;
mod session_lifecycle;
pub use session_lifecycle::*;
mod spaces;
pub use spaces::*;
mod timeline_view;
pub use timeline_view::*;
mod verification_devices;
pub use verification_devices::*;

#[cfg(test)]
mod tests;
