//! Typed SharedCore operations for the shared composer core: slash-command
//! suggestions, running a command, and typed-mention resolution.
//!
//! iOS had no slash commands. It now asks Core what to suggest and what a
//! command did, exactly as desktop does, and renders the outcome.

use super::*;
use crate::app::composer::{
    CommandOutcome, CommandSuggestion, ComposerErrorKind, MentionCandidate, TypedMention,
};
use crate::core::api::{
    MatrixComposerCommandSuggestionsRequest, MatrixComposerResolveMentionsRequest,
    MatrixComposerRunCommandRequest,
};

const COMPOSER_NO_SESSION_CODE: &str = "v-composer-requires-session";

/// A command that did not run. `description` is user copy to show as is;
/// `reason` says why, for the shell's tone.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum ComposerCommandRunError {
    Failed {
        kind: CoreErrorKind,
        reason: ComposerErrorKind,
        code: String,
        description: String,
    },
}

impl TypedFfiError for ComposerCommandRunError {
    fn kind(&self) -> CoreErrorKind {
        match self {
            Self::Failed { kind, .. } => *kind,
        }
    }

    fn code(&self) -> &str {
        match self {
            Self::Failed { code, .. } => code,
        }
    }
}

impl std::fmt::Display for ComposerCommandRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for ComposerCommandRunError {}

fn from_command_error(
    error: crate::app::composer::ComposerCommandError,
) -> ComposerCommandRunError {
    let kind = match error.kind {
        ComposerErrorKind::NotSignedIn => CoreErrorKind::NotReady,
        ComposerErrorKind::Failed | ComposerErrorKind::Partial => CoreErrorKind::Retryable,
        _ => CoreErrorKind::Terminal,
    };
    ComposerCommandRunError::Failed {
        kind,
        reason: error.kind,
        code: error
            .diagnostic_id
            .unwrap_or_else(|| format!("v-composer-{}", reason_wire(error.kind))),
        description: error.message,
    }
}

fn reason_wire(reason: ComposerErrorKind) -> &'static str {
    match reason {
        ComposerErrorKind::Usage => "usage",
        ComposerErrorKind::UnknownCommand => "unknown-command",
        ComposerErrorKind::Permission => "permission",
        ComposerErrorKind::NotFound => "not-found",
        ComposerErrorKind::Partial => "partial",
        ComposerErrorKind::Failed => "failed",
        ComposerErrorKind::NotSignedIn => "not-signed-in",
    }
}

fn from_core_error(error: MatrixIpcError) -> ComposerCommandRunError {
    let not_ready = error.category == MatrixIpcErrorCategory::Forbidden;
    ComposerCommandRunError::Failed {
        kind: if not_ready {
            CoreErrorKind::NotReady
        } else {
            CoreErrorKind::from_category(error.category)
        },
        reason: if not_ready {
            ComposerErrorKind::NotSignedIn
        } else {
            ComposerErrorKind::Failed
        },
        code: error
            .diagnostic_id
            .unwrap_or_else(|| COMPOSER_NO_SESSION_CODE.to_owned()),
        description: if not_ready {
            "Synara is not signed in to your Matrix account. Sign in again, then send your message."
        } else {
            "Synara could not run the command. Try again."
        }
        .to_owned(),
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Slash commands the user may use in `room_id` whose name or alias
    /// starts with `prefix` (with or without the slash). Commands the user's
    /// power level does not allow, or that do not fit a direct message or a
    /// room, are left out.
    pub async fn composer_command_suggestions(
        &self,
        room_id: String,
        prefix: String,
    ) -> Result<Vec<CommandSuggestion>, ComposerCommandRunError> {
        self.core
            .composer_command_suggestions(MatrixComposerCommandSuggestionsRequest {
                room_id,
                prefix,
            })
            .await
            .map_err(from_core_error)
    }

    /// Run what the composer holds in `room_id`. Message commands come back
    /// as `SendMessage` for the composer to send with its reply and thread;
    /// destructive ones come back as `NeedsConfirmation` until `confirmed`.
    pub async fn composer_execute(
        &self,
        room_id: String,
        text: String,
        confirmed: bool,
    ) -> Result<CommandOutcome, ComposerCommandRunError> {
        self.core
            .composer_run_command(MatrixComposerRunCommandRequest {
                room_id,
                text,
                confirmed,
            })
            .await
            .map_err(from_core_error)?
            .map_err(from_command_error)
    }

    /// Mentions typed out as "@Name" in `text` that name exactly one of
    /// `members`. Ranges are UTF-16 offsets. Needs no session.
    pub fn composer_resolve_mentions(
        &self,
        text: String,
        members: Vec<MentionCandidate>,
        own_user_id: String,
    ) -> Vec<TypedMention> {
        self.core
            .composer_resolve_mentions(MatrixComposerResolveMentionsRequest {
                text,
                members,
                own_user_id,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn commands_without_a_session_say_so() {
        let core = SharedCore::new();
        let error = core
            .composer_execute("!r:example.org".into(), "/kick @a:b".into(), false)
            .await
            .expect_err("no session");
        assert_eq!(error.kind(), CoreErrorKind::NotReady);
        let ComposerCommandRunError::Failed {
            reason,
            description,
            ..
        } = error;
        assert_eq!(reason, ComposerErrorKind::NotSignedIn);
        assert!(description.starts_with("Synara is not signed in"));
    }

    #[tokio::test]
    async fn suggestions_and_mentions_work_before_sign_in() {
        let core = SharedCore::new();
        let suggestions = core
            .composer_command_suggestions("!r:example.org".into(), "/upgr".into())
            .await
            .expect("suggestions");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].usage, "/upgraderoom <version>");
        let mentions = core.composer_resolve_mentions(
            "hi @Puck".into(),
            vec![MentionCandidate {
                user_id: "@puck:example.org".into(),
                display_name: Some("Puck".into()),
            }],
            "@me:example.org".into(),
        );
        assert_eq!(mentions.len(), 1);
        assert_eq!((mentions[0].start, mentions[0].end), (3, 8));
    }

    #[test]
    fn usage_errors_are_terminal_and_keep_their_copy() {
        let error = from_command_error(crate::app::composer::ComposerCommandError {
            kind: ComposerErrorKind::Usage,
            message: "Add a room address, like /join #room:server.".into(),
            diagnostic_id: None,
        });
        assert_eq!(error.kind(), CoreErrorKind::Terminal);
        assert_eq!(error.code(), "v-composer-usage");
        assert_eq!(
            error.to_string(),
            "Add a room address, like /join #room:server."
        );
    }
}
