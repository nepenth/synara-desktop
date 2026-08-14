//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

// Generated from `src/synara_core.udl` by build.rs. Keep this at crate root:
// P4-3 adds only a safe Core session-projection mirror to the credential-free
// P4-2 login-flow surface.
uniffi::include_scaffolding!("synara_core");

/// Identifies the project-owned UniFFI surface without exposing a product
/// command, credential, Matrix SDK type, or platform callback prematurely.
/// P4 migration slices grow the UDL only alongside their corresponding core API.
pub fn binding_scaffold_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

mod ffi;
pub use ffi::{
    login_flows, register_flows, LoginFlowDto, LoginFlowsError, RegisterFlowsDto,
    RegisterFlowsError, RegisterFlowsStatus, RegisterUiaFlowDto,
};

mod session_projection_ffi;
pub use session_projection_ffi::{
    SessionProjection, SessionProjectionCore, SessionProjectionError, SessionProjectionLifecycle,
};

mod shared_core_ffi;
pub use shared_core_ffi::{
    ComposerReplyDraftDto, ComposerReplyDraftError, ComposerReplyDraftPreviewDto,
    DeviceCommandError, DeviceDeleteChallengeDto, DeviceDeleteDto, DeviceSnapshotDto,
    DeviceSummaryDto, DirectorySearchCommandError, DirectoryVisibilityCommandError, EditMessageDto,
    EditMessageError, GlobalImagePacksSnapshotDto, ImagePackCommandError, ImagePackDto,
    ImagePackWriteDto, InviteActionError, InviteDto, InviteSnapshotDto, InviteSnapshotError,
    IosSecretVault, IosSecretVaultError, JoinRuleCommandError, LaterCommandError, LaterItemDto,
    LaterSnapshotDto, MDirectCommandError, MDirectMutationDto, MDirectSnapshotDto,
    OwnProfileCommandError, OwnProfileWriteDto, PollRespondDto, PollRespondError,
    PresenceCommandError, PresenceSnapshotDto, PresenceSubscriptionDto, RestrictedJoinReparentDto,
    RoomCreateCommandError, RoomCreateDto, RoomCreateRequestDto, RoomCreatorsSnapshotDto,
    RoomDirectoryHitDto, RoomDirectoryPageDto, RoomDirectoryProtocolInstanceDto,
    RoomDirectoryProtocolsDto, RoomDirectorySearchDto, RoomDirectoryVisibilityDto,
    RoomDirectoryVisibilityWriteDto, RoomImagePacksSnapshotDto, RoomJoinRuleSnapshotDto,
    RoomListRoomDto, RoomListSnapshotDto, RoomListSnapshotError, RoomMemberDto,
    RoomMembersSnapshotDto, RoomMembersSnapshotError, RoomMembershipCommandError,
    RoomMembershipWriteDto, RoomModerationCommandError, RoomModerationWriteDto, RoomNoteItemDto,
    RoomNotesCommandError, RoomNotesSnapshotDto, RoomPowerLevelCommandError,
    RoomPowerLevelTagsSnapshotDto, RoomPowerLevelWriteDto, RoomPowerLevelsSnapshotDto,
    RoomProfileCommandError, RoomProfileWriteDto, SendPollDto, SendPollError, SendStickerDto,
    SendStickerError, SendTextDto, SendTextError, SessionAttachDto, SessionAttachError,
    SessionLoginDto, SessionLoginError, SessionRestoreDto, SessionRestoreError, SharedCore,
    SpaceChildEdgeDto, SpaceChildMutationDto, SpaceChildrenSnapshotDto, SpaceCommandError,
    SpaceHierarchyRoomDto, SpaceHierarchySnapshotDto, SpaceParentEntryDto, SpaceParentsSnapshotDto,
    TimelineError, TimelineEventItemDto, TimelineEventReadbackDto, TimelineMutateDto,
    TimelineMutateError, TimelineOpenDto, TimelineOpenPositionDto, TimelinePinDto,
    TimelinePinError, TimelineReactionDto, TimelineReactionError, TimelineReactionMutationDto,
    TimelineReactionSenderDto, TimelineReadStateDto, TimelineReadStateError, TimelineSnapshotDto,
    TimelineViewPositionDto, TypingCommandError, TypingRoomDto, TypingSnapshotDto,
    UserImagePackSnapshotDto, VerificationEmojiDto, VerificationInboxDto, VerificationListError,
    VerificationRequestDto, VerificationSasDto, VerificationSasError,
};

mod core;
pub use core::Core;

pub mod app;
pub use app::room_list::{
    room_activity_recovery_required, room_unread_presentation, RoomActivityPreviousState,
    RoomUnreadMembership, RoomUnreadPresentationDto,
};

pub mod dto;
pub mod platform;

pub mod task;

pub mod transport;
