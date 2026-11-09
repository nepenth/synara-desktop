//! Typed commands for the shared composer core: command suggestions,
//! running a command, and typed-mention resolution.
//!
//! Commands run through the same Core operations the shells already call
//! (`room_kick`, `set_room_topic`, `room_upgrade`, …), so a slash command and
//! the matching button behave the same.

use super::*;
use crate::app::composer::{
    self, CommandContext, CommandOutcome, CommandSuggestion, ComposerCommandError,
    ComposerErrorKind, ComposerMember, ComposerOps, MentionCandidate, OpResult, RedactSummary,
    TypedMention,
};
use crate::app::room_ops::{MatrixRoomCreatePreset, MatrixRoomCreateVisibility};

/// Exact React/Tauri envelope payload for `matrix_composer_command_suggestions`.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixComposerCommandSuggestionsRequest {
    pub room_id: String,
    pub prefix: String,
}

/// Exact React/Tauri envelope payload for `matrix_composer_run_command`.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixComposerRunCommandRequest {
    pub room_id: String,
    pub text: String,
    #[serde(default)]
    pub confirmed: bool,
}

/// Exact React/Tauri envelope payload for `matrix_composer_resolve_mentions`.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixComposerResolveMentionsRequest {
    pub text: String,
    pub members: Vec<MentionCandidate>,
    pub own_user_id: String,
}

/// `run_command` over Core's own operations.
struct CoreComposerOps<'a> {
    state: &'a Arc<CoreState>,
}

/// The diagnostic id of a failed Core operation.
fn diagnostic(error: MatrixIpcError) -> String {
    error
        .diagnostic_id
        .unwrap_or_else(|| "v-composer-failed".to_owned())
}

impl CoreComposerOps<'_> {
    fn room_owner(&self) -> OpResult<Arc<NativeRoomJoinRuleOwner>> {
        self.state
            .join_rule_owner()
            .map_err(diagnostic)?
            .ok_or_else(|| "v-composer-requires-session".to_owned())
    }
}

impl ComposerOps for CoreComposerOps<'_> {
    fn own_user_id(&self) -> Option<String> {
        self.room_owner()
            .ok()
            .and_then(|owner| owner.own_user_id())
            .or_else(|| {
                self.state
                    .session_snapshot()
                    .ok()
                    .flatten()
                    .map(|session| session.user_id)
            })
    }

    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_millis() as u64)
    }

    async fn context(&self, room_id: &str) -> OpResult<CommandContext> {
        Ok(self.room_owner()?.composer_context(room_id).await?)
    }

    async fn members(&self, room_id: &str) -> OpResult<Vec<ComposerMember>> {
        let snapshot = room_members_snapshot(
            self.state,
            MatrixRoomMembersSnapshotRequest {
                room_id: room_id.to_owned(),
            },
        )
        .await
        .map_err(diagnostic)?;
        Ok(snapshot
            .members
            .into_iter()
            .map(|member| ComposerMember {
                user_id: member.user_id,
                display_name: member.display_name,
                membership: member.membership.as_str().to_owned(),
            })
            .collect())
    }

    async fn invite(&self, room_id: &str, user_id: &str, reason: Option<String>) -> OpResult<()> {
        room_invite(self.state, moderation(room_id, user_id, reason))
            .await
            .map_err(diagnostic)
    }

    async fn kick(&self, room_id: &str, user_id: &str, reason: Option<String>) -> OpResult<()> {
        room_kick(self.state, moderation(room_id, user_id, reason))
            .await
            .map_err(diagnostic)
    }

    async fn ban(&self, room_id: &str, user_id: &str, reason: Option<String>) -> OpResult<()> {
        room_ban(self.state, moderation(room_id, user_id, reason))
            .await
            .map_err(diagnostic)
    }

    async fn unban(&self, room_id: &str, user_id: &str) -> OpResult<()> {
        room_unban(
            self.state,
            MatrixRoomUnbanRequest {
                room_id: room_id.to_owned(),
                user_id: user_id.to_owned(),
            },
        )
        .await
        .map_err(diagnostic)
    }

    async fn join(&self, room_id_or_alias: &str) -> OpResult<()> {
        room_join(
            self.state,
            MatrixRoomJoinRequest {
                room_id_or_alias: room_id_or_alias.to_owned(),
                via_servers: None,
            },
        )
        .await
        .map_err(diagnostic)
    }

    async fn leave(&self, room_id: &str) -> OpResult<()> {
        room_leave(
            self.state,
            MatrixRoomLeaveRequest {
                room_id: room_id.to_owned(),
            },
        )
        .await
        .map_err(diagnostic)
    }

    async fn ignore(&self, user_id: &str) -> OpResult<()> {
        ignored_users_ignore(self.state, ignored(user_id))
            .await
            .map(drop)
            .map_err(diagnostic)
    }

    async fn unignore(&self, user_id: &str) -> OpResult<()> {
        ignored_users_unignore(self.state, ignored(user_id))
            .await
            .map(drop)
            .map_err(diagnostic)
    }

    async fn set_room_name(&self, room_id: &str, name: &str) -> OpResult<()> {
        set_room_name(
            self.state,
            MatrixSetRoomNameRequest {
                room_id: room_id.to_owned(),
                name: name.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn set_room_topic(&self, room_id: &str, topic: &str) -> OpResult<()> {
        set_room_topic(
            self.state,
            MatrixSetRoomTopicRequest {
                room_id: room_id.to_owned(),
                topic: topic.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn set_display_name(&self, name: &str) -> OpResult<()> {
        set_own_display_name(
            self.state,
            MatrixSetOwnDisplayNameRequest {
                display_name: name.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn set_avatar(&self, mxc: &str) -> OpResult<()> {
        set_own_avatar(
            self.state,
            MatrixSetOwnAvatarRequest {
                mxc: mxc.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn state_content(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
    ) -> OpResult<Option<serde_json::Value>> {
        Ok(self
            .room_owner()?
            .state_content(room_id, event_type, state_key)
            .await?)
    }

    async fn send_state(
        &self,
        room_id: &str,
        event_type: &str,
        state_key: &str,
        content: serde_json::Value,
    ) -> OpResult<()> {
        send_state_event(
            self.state,
            MatrixSendStateEventRequest {
                room_id: room_id.to_owned(),
                event_type: event_type.to_owned(),
                state_key: state_key.to_owned(),
                content,
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn set_power_level(&self, room_id: &str, user_id: &str, level: i64) -> OpResult<()> {
        room_set_power_level(
            self.state,
            MatrixRoomSetPowerLevelRequest {
                room_id: room_id.to_owned(),
                user_id: user_id.to_owned(),
                power_level: level,
            },
        )
        .await
        .map_err(diagnostic)
    }

    async fn dm_room(&self, user_id: &str) -> OpResult<Option<String>> {
        Ok(self.room_owner()?.dm_room_for(user_id)?)
    }

    async fn create_dm(&self, user_ids: &[String]) -> OpResult<String> {
        // The same request desktop's /startdm sent.
        room_create(
            self.state,
            MatrixRoomCreateRequest {
                name: None,
                topic: None,
                room_version: None,
                room_alias_name: None,
                is_direct: true,
                invite: user_ids.to_vec(),
                visibility: Some(MatrixRoomCreateVisibility::Private),
                preset: Some(MatrixRoomCreatePreset::TrustedPrivate),
                creation_content: None,
                encryption: true,
                encrypt_state_events: false,
                join_rule: None,
                knock: false,
                parent_room_id: None,
                power_level_content_override: None,
            },
        )
        .await
        .map_err(diagnostic)
    }

    async fn mark_dm(&self, room_id: &str, user_id: &str) -> OpResult<()> {
        mdirect_add(
            self.state,
            MatrixMDirectAddRequest {
                room_id: room_id.to_owned(),
                user_id: user_id.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn unmark_dm(&self, room_id: &str) -> OpResult<()> {
        mdirect_remove(
            self.state,
            MatrixMDirectRemoveRequest {
                room_id: room_id.to_owned(),
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn send_text(&self, room_id: &str, body: &str) -> OpResult<()> {
        send_text(
            self.state,
            MatrixSendTextRequest {
                room_id: room_id.to_owned(),
                body: body.to_owned(),
                msg_type: None,
                formatted_body: None,
                mention_user_ids: None,
                mention_room: None,
                reply_to: None,
                thread_root: None,
                txn_id: None,
            },
        )
        .await
        .map(drop)
        .map_err(diagnostic)
    }

    async fn bulk_redact(
        &self,
        room_id: &str,
        user_ids: &[String],
        since_ts: u64,
        event_types: &[String],
        reason: Option<String>,
    ) -> OpResult<RedactSummary> {
        let result = super::native_extras::room_bulk_redact(
            self.state,
            crate::app::room_profile::NativeBulkRedactRequest {
                room_id: room_id.to_owned(),
                user_ids: user_ids.to_vec(),
                since_ts,
                event_types: event_types.to_vec(),
                reason,
            },
        )
        .await
        .map_err(diagnostic)?;
        Ok(RedactSummary {
            redacted: result.redacted,
            failed: result.failed,
            truncated: result.truncated,
        })
    }

    async fn upgrade_room(&self, room_id: &str, version: &str) -> OpResult<String> {
        super::native_extras::room_upgrade(
            self.state,
            crate::app::room_profile::NativeRoomUpgradeRequest {
                room_id: room_id.to_owned(),
                new_version: version.to_owned(),
                additional_creators: Vec::new(),
            },
        )
        .await
        .map(|result| result.replacement_room_id)
        .map_err(diagnostic)
    }

    async fn event_at(&self, room_id: &str, timestamp_ms: u64) -> OpResult<Option<String>> {
        match timeline_timestamp_to_event(
            self.state,
            MatrixTimelineTimestampToEventRequest {
                room_id: room_id.to_owned(),
                timestamp_ms,
            },
        )
        .await
        {
            Ok(readback) => Ok(Some(readback.event_id)),
            // The homeserver answers 404 when nothing was sent after the date.
            Err(error)
                if error.diagnostic_id.as_deref()
                    == Some("p2-timeline-timestamp-to-event-failed") =>
            {
                Ok(None)
            }
            Err(error) => Err(diagnostic(error)),
        }
    }
}

fn moderation(room_id: &str, user_id: &str, reason: Option<String>) -> MatrixRoomModerationRequest {
    MatrixRoomModerationRequest {
        room_id: room_id.to_owned(),
        user_id: user_id.to_owned(),
        reason,
    }
}

fn ignored(user_id: &str) -> MatrixIgnoredUsersUserRequest {
    MatrixIgnoredUsersUserRequest {
        user_id: user_id.to_owned(),
    }
}

/// Typed `matrix_composer_command_suggestions`.
pub(super) async fn composer_command_suggestions(
    state: &Arc<CoreState>,
    payload: MatrixComposerCommandSuggestionsRequest,
) -> Result<Vec<CommandSuggestion>, MatrixIpcError> {
    let ops = CoreComposerOps { state };
    // Without the room's power levels every command is offered; running one
    // still checks, and the server has the last word.
    let context = ops.context(&payload.room_id).await.unwrap_or_default();
    Ok(composer::command_suggestions(&payload.prefix, &context))
}

/// Typed `matrix_composer_run_command`.
pub(super) async fn composer_run_command(
    state: &Arc<CoreState>,
    payload: MatrixComposerRunCommandRequest,
) -> Result<Result<CommandOutcome, ComposerCommandError>, MatrixIpcError> {
    if state.join_rule_owner()?.is_none() {
        return Ok(Err(ComposerCommandError {
            kind: ComposerErrorKind::NotSignedIn,
            message: "Synara is not signed in to your Matrix account. Sign in again, then send your message.".to_owned(),
            diagnostic_id: Some("v-composer-requires-session".to_owned()),
        }));
    }
    let ops = CoreComposerOps { state };
    Ok(composer::run_command(&ops, &payload.room_id, &payload.text, payload.confirmed).await)
}

/// Typed `matrix_composer_resolve_mentions`. Needs no session.
pub(super) fn composer_resolve_mentions(
    payload: MatrixComposerResolveMentionsRequest,
) -> Vec<TypedMention> {
    composer::resolve_typed_mentions(&payload.text, &payload.members, &payload.own_user_id)
}

impl Core {
    /// Typed `matrix_composer_command_suggestions`.
    pub async fn composer_command_suggestions(
        &self,
        request: MatrixComposerCommandSuggestionsRequest,
    ) -> Result<Vec<CommandSuggestion>, MatrixIpcError> {
        composer_command_suggestions(&self.state, request).await
    }

    /// Typed `matrix_composer_run_command`. The outer error is a Core
    /// failure; the inner one is a command that did not run, with user copy.
    pub async fn composer_run_command(
        &self,
        request: MatrixComposerRunCommandRequest,
    ) -> Result<Result<CommandOutcome, ComposerCommandError>, MatrixIpcError> {
        composer_run_command(&self.state, request).await
    }

    /// Typed `matrix_composer_resolve_mentions`.
    pub fn composer_resolve_mentions(
        &self,
        request: MatrixComposerResolveMentionsRequest,
    ) -> Vec<TypedMention> {
        composer_resolve_mentions(request)
    }
}

#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    fn suggestions(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
        Box::pin(async move {
            let payload: MatrixComposerCommandSuggestionsRequest =
                serde_json::from_value(request.payload)
                    .map_err(|_| core_state_error("composer-invalid-payload"))?;
            let response = composer_command_suggestions(&state, payload).await?;
            serde_json::to_value(response)
                .map_err(|_| core_state_error("composer-serialization-failed"))
        })
    }
    fn run(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
        Box::pin(async move {
            let payload: MatrixComposerRunCommandRequest = serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("composer-invalid-payload"))?;
            let response = composer_run_command(&state, payload).await?;
            serde_json::to_value(response)
                .map_err(|_| core_state_error("composer-serialization-failed"))
        })
    }
    fn mentions(_: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
        Box::pin(async move {
            let payload: MatrixComposerResolveMentionsRequest =
                serde_json::from_value(request.payload)
                    .map_err(|_| core_state_error("composer-invalid-payload"))?;
            serde_json::to_value(composer_resolve_mentions(payload))
                .map_err(|_| core_state_error("composer-serialization-failed"))
        })
    }
    for (name, handler) in [
        (
            "matrix_composer_command_suggestions",
            suggestions as fn(Arc<CoreState>, CommandEnvelope) -> CommandFuture,
        ),
        ("matrix_composer_run_command", run),
        ("matrix_composer_resolve_mentions", mentions),
    ] {
        registry
            .register(name, handler)
            .expect("built-in composer commands must remain in the command census");
    }
}
