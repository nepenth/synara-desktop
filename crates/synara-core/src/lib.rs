//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

// Linux release `tauri build` overflows rustc's default query-depth limit
// while laying out Core command futures (`core.rs` timeline-open and peers).
#![recursion_limit = "256"]

// Generated from `src/synara_core.udl` by build.rs. Keep this at crate root:
// P4-3 adds only a safe Core session-projection mirror to the credential-free
// P4-2 login-flow surface.
#[cfg(feature = "full-uniffi")]
uniffi::include_scaffolding!("synara_core");

/// Identifies the project-owned UniFFI surface without exposing a product
/// command, credential, Matrix SDK type, or platform callback prematurely.
/// P4 migration slices grow the UDL only alongside their corresponding core API.
pub fn binding_scaffold_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Converts supported Markdown to Matrix-compatible HTML using Ruma's parser.
/// Plain text deliberately returns `None` so clients omit a redundant
/// `formatted_body` field.
pub fn markdown_to_html(body: String) -> Option<String> {
    use matrix_sdk::ruma::{
        events::room::message::FormattedBody,
        html::{HtmlSanitizerMode, RemoveReplyFallback},
    };

    let mut formatted = FormattedBody::markdown(body)?;
    formatted.sanitize_html(HtmlSanitizerMode::Strict, RemoveReplyFallback::No);
    Some(formatted.body)
}

#[cfg(test)]
mod markdown_tests {
    use super::markdown_to_html;

    #[test]
    fn plain_text_does_not_emit_redundant_html() {
        assert_eq!(markdown_to_html("hello world".into()), None);
    }

    #[test]
    fn rich_markdown_emits_matrix_html() {
        let html = markdown_to_html("- **Ship it**\n- `verify`".into()).expect("formatted body");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<strong>Ship it</strong>"));
        assert!(html.contains("<code>verify</code>"));
    }

    #[test]
    fn markdown_html_is_sanitized_to_the_matrix_allowlist() {
        let html =
            markdown_to_html("**safe** <script>alert(1)</script>".into()).expect("formatted body");
        assert!(html.contains("<strong>safe</strong>"));
        assert!(!html.contains("<script>"));
    }
}

#[cfg(all(test, feature = "full-uniffi"))]
mod generated_binding_tests {
    #[test]
    fn every_async_udl_export_uses_the_tokio_bridge() {
        let udl = include_str!("synara_core.udl");
        let scaffolding = include_str!(concat!(env!("OUT_DIR"), "/synara_core.uniffi.rs"));
        let async_declarations = udl.matches("[Async").count();
        let bridged_exports = scaffolding
            .matches("#[::uniffi::export_for_udl(async_runtime = \"tokio\")]")
            .count();

        assert!(async_declarations > 0);
        assert_eq!(bridged_exports, async_declarations);
    }
}

#[cfg(feature = "full-uniffi")]
mod ffi;
#[cfg(feature = "full-uniffi")]
pub use ffi::{
    login_flows, register_flows, LoginFlowDto, LoginFlowsError, RegisterFlowsDto,
    RegisterFlowsError, RegisterFlowsStatus, RegisterUiaFlowDto,
};

#[cfg(feature = "full-uniffi")]
mod session_projection_ffi;
#[cfg(feature = "full-uniffi")]
pub use session_projection_ffi::{
    SessionProjection, SessionProjectionCore, SessionProjectionError, SessionProjectionLifecycle,
};

#[cfg(feature = "full-uniffi")]
mod shared_core_ffi;
#[cfg(feature = "full-uniffi")]
pub use shared_core_ffi::{
    AgentApprovalSendDto, AgentApprovalSendError, BackupStatusDto, ComposerReplyDraftDto,
    ComposerReplyDraftError, ComposerReplyDraftPreviewDto, CrossSigningStatusDto, CryptoStatusDto,
    DeviceCommandError, DeviceDeleteChallengeDto, DeviceDeleteDto, DeviceSnapshotDto,
    DeviceSummaryDto, DirectorySearchCommandError, DirectoryVisibilityCommandError, EditMessageDto,
    EditMessageError, GlobalImagePacksSnapshotDto, IgnoredUsersCommandError,
    IgnoredUsersSnapshotDto, IgnoredUsersWriteDto, ImagePackCommandError, ImagePackDto,
    ImagePackWriteDto, InviteActionError, InviteDto, InviteSnapshotDto, InviteSnapshotError,
    IosSecretVault, IosSecretVaultError, JoinRuleCommandError, LaterCommandError, LaterItemDto,
    LaterSnapshotDto, LeftoverAckDto, LeftoverBytesDto, LeftoverCommandError, MDirectCommandError,
    MDirectMutationDto, MDirectSnapshotDto, MediaBytesDto, MediaConfigDto, MediaUploadDto,
    MediaUploadError, MessageSearchDto, MessageSearchError, MessageSearchGroupDto,
    MessageSearchItemDto, NseEventPreviewDto, NseStoreDto, NseStoreError, OwnProfileCommandError,
    OwnProfileDto, OwnProfileUploadDto, OwnProfileWriteDto, OwnerUpdateDto, OwnerUpdateError,
    PlainMediaError, PollRespondDto, PollRespondError, PresenceCommandError, PresenceSnapshotDto,
    PresenceSubscriptionDto, PresenceWriteDto, PushRuleMentionsDto, PushRulesCommandError,
    PushRulesSnapshotDto, PushRulesWriteDto, PusherCommandError, PusherWriteDto, RestoreBackupDto,
    RestoreBackupError, RestrictedJoinReparentDto, RoomCreateCommandError, RoomCreateDto,
    RoomCreateRequestDto, RoomCreatorsSnapshotDto, RoomDirectoryHitDto, RoomDirectoryPageDto,
    RoomDirectoryProtocolInstanceDto, RoomDirectoryProtocolsDto, RoomDirectorySearchDto,
    RoomDirectoryVisibilityDto, RoomDirectoryVisibilityWriteDto, RoomImagePacksSnapshotDto,
    RoomJoinRuleSnapshotDto, RoomJoinRuleWriteDto, RoomKeyTransferStatusDto, RoomListRoomDto,
    RoomListSnapshotDto, RoomListSnapshotError, RoomListUpdateDto, RoomListUpdateError,
    RoomMemberDto, RoomMembersSnapshotDto, RoomMembersSnapshotError, RoomMembershipCommandError,
    RoomMembershipWriteDto, RoomModerationCommandError, RoomModerationWriteDto, RoomNoteItemDto,
    RoomNotesCommandError, RoomNotesSnapshotDto, RoomNotificationCommandError,
    RoomNotificationSnapshotDto, RoomNotificationWriteDto, RoomNotificationsSnapshotDto,
    RoomPowerLevelCommandError, RoomPowerLevelTagsSnapshotDto, RoomPowerLevelWriteDto,
    RoomPowerLevelsSnapshotDto, RoomProfileCommandError, RoomProfileWriteDto,
    SecretStorageStatusDto, SendPollDto, SendPollError, SendRoomAttachmentDto,
    SendRoomAttachmentError, SendStickerDto, SendStickerError, SendTextDto, SendTextError,
    SessionAttachDto, SessionAttachError, SessionLoginDto, SessionLoginError, SessionRestoreDto,
    SessionRestoreError, SessionSnapshotDto, SessionStatusError, SharedCore, SpaceChildEdgeDto,
    SpaceChildMutationDto, SpaceChildrenSnapshotDto, SpaceCommandError, SpaceHierarchyRoomDto,
    SpaceHierarchySnapshotDto, SpaceParentEntryDto, SpaceParentsSnapshotDto, SyncStartDto,
    SyncStartError, SyncStatusDto, ThreepidAddDto, ThreepidCommandError, ThreepidEmailDto,
    ThreepidEmailTokenDto, ThreepidSnapshotDto, ThreepidWriteDto, TimelineError,
    TimelineEventItemDto, TimelineEventReadbackDto, TimelineForwardDto, TimelineForwardError,
    TimelineMediaError, TimelineMutateDto, TimelineMutateError, TimelineOpenDto,
    TimelineOpenPositionDto, TimelinePinDto, TimelinePinError, TimelineReactionDto,
    TimelineReactionError, TimelineReactionMutationDto, TimelineReactionSenderDto,
    TimelineReadStateDto, TimelineReadStateError, TimelineSnapshotDto, TimelineViewPositionDto,
    TimelineViewReactionDto, TimelineViewRowDto, TimelineViewUpdateDto, TimelineViewUpdateError,
    TimelineVoteDeclineDto, TimelineVoteDeclineError, TypingCommandError, TypingRoomDto,
    TypingSnapshotDto, UserDirectoryHitDto, UserDirectorySearchDto, UserDirectorySearchError,
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
