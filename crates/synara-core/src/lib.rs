//! # synara-core
//!
//! Transport-agnostic shared native core for Synara (desktop via Tauri, iOS via
//! uniffi). Domain modules move here by `git mv` + path updates only, keeping
//! behavior identical (P1 slices: dto, transport/ipc, task, then the app/
//! domain chunks).

// Linux release `tauri build` overflows rustc's default query-depth limit
// while laying out Core command futures (`core.rs` timeline-open and peers).
#![recursion_limit = "256"]

// UniFFI proc-macro scaffolding. Every exported item carries its own
// `#[uniffi::...]` attribute; the Swift surface is pinned by
// synara-ios/SynaraCore/api/synara_core.swift-api.txt.
#[cfg(feature = "full-uniffi")]
uniffi::setup_scaffolding!("synara_core");

/// Identifies the project-owned UniFFI surface without exposing a product
/// command, credential, Matrix SDK type, or platform callback prematurely.
/// P4 migration slices grow the UDL only alongside their corresponding core API.
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn binding_scaffold_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Converts supported Markdown to Matrix-compatible HTML using Ruma's parser.
/// Plain text deliberately returns `None` so clients omit a redundant
/// `formatted_body` field.
#[cfg(feature = "full-app")]
#[cfg_attr(feature = "full-uniffi", uniffi::export)]
pub fn markdown_to_html(body: String) -> Option<String> {
    use matrix_sdk::ruma::{
        events::room::message::FormattedBody,
        html::{HtmlSanitizerMode, RemoveReplyFallback},
    };

    let mut formatted = FormattedBody::markdown(body)?;
    formatted.sanitize_html(HtmlSanitizerMode::Strict, RemoveReplyFallback::No);
    Some(formatted.body)
}

/// Opens a composer placeholder: `PLACEHOLDER_OPEN`, a decimal fragment index,
/// `PLACEHOLDER_CLOSE`. Private-use code points the composer strips from typed
/// text, so a user cannot forge one.
pub const COMPOSER_PLACEHOLDER_OPEN: char = '\u{E000}';
pub const COMPOSER_PLACEHOLDER_CLOSE: char = '\u{E001}';
/// Stand-ins that keep typed `<` and `&` literal through markdown, in prose
/// and in code alike. Also stripped from typed text by the composer.
const COMPOSER_LITERAL_LT: char = '\u{E002}';
const COMPOSER_LITERAL_AMP: char = '\u{E003}';

/// Render composer markdown the way [`markdown_to_html`] does, with inline
/// constructs markdown cannot express (mentions, custom emoji, links, spoilers,
/// underline) passed as HTML fragments behind placeholders.
///
/// Typed text is literal: `<b>` stays text rather than raw HTML, as it always
/// has in the desktop composer. Every fragment is reduced to the Matrix
/// allowlist before substitution, and placeholders with an unknown index are
/// dropped. Returns `None` when the message is plain text with no fragments,
/// so callers omit `formatted_body`.
#[cfg(feature = "full-app")]
pub fn render_composer_markdown(source: &str, fragments: &[String]) -> Option<String> {
    use matrix_sdk::ruma::html::{sanitize_html, HtmlSanitizerMode, RemoveReplyFallback};

    let used_fragment = source.contains(COMPOSER_PLACEHOLDER_OPEN) && !fragments.is_empty();
    let shielded: String = source
        .chars()
        .map(|ch| match ch {
            '<' => COMPOSER_LITERAL_LT,
            '&' => COMPOSER_LITERAL_AMP,
            other => other,
        })
        .collect();
    let html = match markdown_to_html(shielded.clone()) {
        Some(html) => html,
        None if used_fragment => escape_plain_html(&shielded),
        None => return None,
    };
    let html = html
        .replace(COMPOSER_LITERAL_LT, "&lt;")
        .replace(COMPOSER_LITERAL_AMP, "&amp;");
    let safe: Vec<String> = fragments
        .iter()
        .map(|fragment| {
            sanitize_html(
                fragment,
                HtmlSanitizerMode::Strict,
                RemoveReplyFallback::Yes,
            )
        })
        .collect();
    Some(substitute_placeholders(&html, &safe))
}

#[cfg(feature = "full-app")]
fn escape_plain_html(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\n' => out.push_str("<br />\n"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(feature = "full-app")]
fn substitute_placeholders(html: &str, fragments: &[String]) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(COMPOSER_PLACEHOLDER_OPEN) {
        out.push_str(&rest[..start]);
        let after = &rest[start + COMPOSER_PLACEHOLDER_OPEN.len_utf8()..];
        match after.find(COMPOSER_PLACEHOLDER_CLOSE) {
            Some(end) => {
                if let Some(fragment) = after[..end]
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| fragments.get(index))
                {
                    out.push_str(fragment);
                }
                rest = &after[end + COMPOSER_PLACEHOLDER_CLOSE.len_utf8()..];
            }
            None => {
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.replace(COMPOSER_PLACEHOLDER_CLOSE, "")
}

#[cfg(all(test, feature = "full-app"))]
mod composer_markdown_tests {
    use super::*;

    fn token(index: usize) -> String {
        format!("{COMPOSER_PLACEHOLDER_OPEN}{index}{COMPOSER_PLACEHOLDER_CLOSE}")
    }

    #[test]
    fn plain_text_without_fragments_has_no_formatted_body() {
        assert_eq!(render_composer_markdown("hello there", &[]), None);
    }

    #[test]
    fn markdown_renders_and_fragments_land_in_place() {
        let mention = r#"<a href="https://matrix.to/#/@alice:example.org">Alice</a>"#.to_owned();
        let source = format!("**hi** {}\n- one\n- two", token(0));
        let html = render_composer_markdown(&source, &[mention]).expect("html");
        assert!(html.contains("<strong>hi</strong>"), "{html}");
        assert!(
            html.contains(r#"<a href="https://matrix.to/#/@alice:example.org">Alice</a>"#),
            "{html}"
        );
        assert!(html.contains("<ul>"), "{html}");
        assert!(!html.contains(COMPOSER_PLACEHOLDER_OPEN));
    }

    #[test]
    fn a_fragment_alone_still_yields_html_with_escaped_text() {
        let spoiler = "<span data-mx-spoiler>secret</span>".to_owned();
        let source = format!("a <b> & {}", token(0));
        let html = render_composer_markdown(&source, &[spoiler]).expect("html");
        assert!(html.contains("data-mx-spoiler"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
    }

    #[test]
    fn fragments_are_sanitized_and_unknown_indexes_dropped() {
        let hostile = r#"<a href="javascript:alert(1)" onclick="x()">x</a><script>bad()</script>"#;
        let source = format!("**x** {} {}", token(0), token(7));
        let html = render_composer_markdown(&source, &[hostile.to_owned()]).expect("html");
        for banned in ["javascript:", "onclick", "<script"] {
            assert!(!html.contains(banned), "{banned}: {html}");
        }
        assert!(!html.contains('7'), "{html}");
    }

    #[test]
    fn typed_angle_brackets_and_ampersands_stay_literal_in_text_and_code() {
        let html = render_composer_markdown("use **<b>** & `a<b && c`", &[]).expect("html");
        assert!(html.contains("<strong>&lt;b&gt;</strong>"), "{html}");
        assert!(html.contains("&amp; "), "{html}");
        assert!(html.contains("<code>a&lt;b &amp;&amp; c</code>"), "{html}");
        assert!(!html.contains("<b>"), "{html}");
    }

    #[test]
    fn markdown_files_and_messages_render_the_common_blocks() {
        let html = render_composer_markdown(
            "# Agent notes\n\nUse **bold** and ~~old~~ plus `code`.\n\n- dash\n\n* star\n\n10. ten\n11. eleven\n\n| Name | Role |\n| --- | --- |\n| Ada | Lead |\n",
            &[],
        )
        .expect("html");
        for expected in [
            "<h1>Agent notes</h1>",
            "<strong>bold</strong>",
            "<del>old</del>",
            "<code>code</code>",
            "<ul>",
            "<ol start=\"10\">",
            "<table>",
            "<th>Name</th>",
        ] {
            assert!(html.contains(expected), "{expected}: {html}");
        }
    }

    #[test]
    fn newlines_are_hard_breaks_like_the_composer() {
        let html = render_composer_markdown("line one\nline **two**", &[]).expect("html");
        assert!(html.contains("<br"), "{html}");
    }
}

#[cfg(all(test, feature = "full-app"))]
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
    AgentApprovalDecisionDto, AgentApprovalHistoryCommandError, AgentApprovalHistoryItemDto,
    AgentApprovalHistorySnapshotDto, AgentApprovalInboxDto, AgentApprovalInboxError,
    AgentApprovalInboxItemDto, AgentApprovalSendDto, AgentApprovalSendError,
    AgentNotificationPreferencesDto, AgentNotificationPreferencesError, BackupActionDto,
    BackupAvailabilityDto, BackupDeviceStateDto, BackupRecoveryStateDto, BackupStatusDto,
    CommandGateDto, ComposerReplyDraftDto, ComposerReplyDraftError, ComposerReplyDraftPreviewDto,
    CrossSigningReadinessDto, CrossSigningStateDto, CrossSigningStatusDto, CryptoStatusDto,
    DeviceCommandError, DeviceDeleteAuthenticationDto, DeviceDeleteChallengeDto, DeviceDeleteDto,
    DeviceDeleteOutcomeDto, DeviceSnapshotDto, DeviceSummaryDto, DeviceTrustDto,
    DirectorySearchCommandError, DirectoryVisibilityCommandError, EditMessageDto, EditMessageError,
    GlobalImagePacksSnapshotDto, HttpPusherOwner, IgnoredUsersCommandError,
    IgnoredUsersSnapshotDto, IgnoredUsersWriteDto, ImagePackCommandError, ImagePackDto,
    ImagePackWriteDto, InboxNotificationDto, InboxNotificationsError, InboxNotificationsPageDto,
    InviteActionError, InviteDto, InviteSnapshotDto, InviteSnapshotError, IosSecretVault,
    IosSecretVaultError, JoinRuleCommandError, LaterCommandError, LaterItemDto, LaterSnapshotDto,
    LeftoverAckDto, LeftoverAckStatusDto, LeftoverBytesDto, LeftoverCommandError,
    MDirectCommandError, MDirectMutationDto, MDirectSnapshotDto, MediaBytesDto, MediaConfigDto,
    MediaUploadDto, MediaUploadError, MessageSearchDto, MessageSearchError, MessageSearchGroupDto,
    MessageSearchItemDto, MissingSecretDto, OwnDeviceVerificationDto, OwnProfileCommandError,
    OwnProfileDto, OwnProfileUploadDto, OwnProfileWriteDto, OwnerUpdateDto, OwnerUpdateError,
    PlainMediaError, PollRespondDto, PollRespondError, PresenceCommandError, PresenceSnapshotDto,
    PresenceSubscriptionDto, PresenceWriteDto, PushRuleMentionsDto, PushRulesCommandError,
    PushRulesSnapshotDto, PushRulesWriteDto, PusherCommandError, PusherWriteDto, RestoreBackupDto,
    RestoreBackupError, RestrictedJoinReparentDto, RoomCreateCommandError, RoomCreateDto,
    RoomCreateRequestDto, RoomCreatorsSnapshotDto, RoomDirectoryHitDto, RoomDirectoryPageDto,
    RoomDirectoryProtocolInstanceDto, RoomDirectoryProtocolsDto, RoomDirectorySearchDto,
    RoomDirectoryVisibilityDto, RoomDirectoryVisibilityWriteDto, RoomImagePacksSnapshotDto,
    RoomJoinRuleSnapshotDto, RoomJoinRuleWriteDto, RoomKeyTransferPhaseDto,
    RoomKeyTransferStatusDto, RoomListRoomDto, RoomListSnapshotDto, RoomListSnapshotError,
    RoomListUpdateDto, RoomListUpdateError, RoomMemberDto, RoomMembersSnapshotDto,
    RoomMembersSnapshotError, RoomMembershipCommandError, RoomMembershipWriteDto,
    RoomModerationCommandError, RoomModerationWriteDto, RoomNoteItemDto, RoomNotesCommandError,
    RoomNotesSnapshotDto, RoomNotificationCommandError, RoomNotificationSnapshotDto,
    RoomNotificationWriteDto, RoomNotificationsSnapshotDto, RoomPowerLevelCommandError,
    RoomPowerLevelTagsSnapshotDto, RoomPowerLevelWriteDto, RoomPowerLevelsSnapshotDto,
    RoomProfileCommandError, RoomProfileWriteDto, RtcTransportDto, RtcTransportsCommandError,
    RtcTransportsSnapshotDto, SecretStorageActionDto, SecretStorageSetupDto, SecretStorageStateDto,
    SecretStorageStatusDto, SendPollDto, SendPollError, SendRoomAttachmentDto,
    SendRoomAttachmentError, SendTextDto, SendTextError, SessionAttachDto, SessionAttachError,
    SessionLoginDto, SessionLoginError, SessionRestoreDto, SessionRestoreError, SessionSnapshotDto,
    SessionStatusDto, SessionStatusError, SharedCore, SpaceChildEdgeDto, SpaceChildMutationDto,
    SpaceChildrenSnapshotDto, SpaceCommandError, SpaceHierarchyRoomDto, SpaceHierarchySnapshotDto,
    SpaceParentEntryDto, SpaceParentsSnapshotDto, SyncReadinessDto, SyncStartDto, SyncStartError,
    SyncStatusDto, SyncStopDto, SyncStopError, ThreepidAddDto, ThreepidCommandError,
    ThreepidEmailDto, ThreepidEmailTokenDto, ThreepidSnapshotDto, ThreepidWriteDto,
    TimelineActionKindDto, TimelineActionStatusDto, TimelineDirectionDto, TimelineError,
    TimelineEventItemDto, TimelineEventReadbackDto, TimelineForwardDto, TimelineForwardError,
    TimelineMediaError, TimelineMutateDto, TimelineMutateError, TimelineOpenDto,
    TimelineOpenKindDto, TimelineOpenPositionDto, TimelinePageStateDto, TimelinePinDto,
    TimelinePinError, TimelinePositionKindDto, TimelineReactionDto, TimelineReactionError,
    TimelineReactionMutationDto, TimelineReactionSenderDto, TimelineReadActionDto,
    TimelineReadIntentDto, TimelineReadStateDto, TimelineReadStateError, TimelineRowKindDto,
    TimelineSnapshotDto, TimelineViewPollAnswerDto, TimelineViewPollDto, TimelineViewPositionDto,
    TimelineViewReactionDto, TimelineViewReplyPreviewDto, TimelineViewRowCapabilitiesDto,
    TimelineViewRowDto, TimelineViewThreadSummaryDto, TimelineViewUpdateDto,
    TimelineViewUpdateError, TimelineVoteDeclineDto, TimelineVoteDeclineError, TypingCommandError,
    TypingRoomDto, TypingSnapshotDto, UserDirectoryHitDto, UserDirectorySearchDto,
    UserDirectorySearchError, UserImagePackSnapshotDto, UserInCallDto, UserStatusCommandError,
    UserStatusFieldDto, UserStatusSnapshotDto, UserStatusWriteDto, VerificationDirectionDto,
    VerificationEmojiDto, VerificationInboxDto, VerificationListError, VerificationPhaseDto,
    VerificationQrDto, VerificationRequestDto, VerificationSasDto, VerificationSasError,
};

#[cfg(feature = "full-app")]
mod core;
#[cfg(feature = "full-app")]
pub use core::{
    api as core_api, Core, MatrixCrossSigningState, MatrixCryptoStatus, MatrixSessionSnapshot,
};

pub mod app;
#[cfg(feature = "full-app")]
pub use app::room_list::{
    room_activity_recovery_required, room_unread_presentation, RoomActivityPreviousState,
    RoomEncryptionStatus, RoomUnreadMembership, RoomUnreadPresentationDto,
};

#[cfg(feature = "full-app")]
pub mod dto;
#[cfg(feature = "full-app")]
pub mod platform;

pub mod transport;
