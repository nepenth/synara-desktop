//! Desktop bridges for the shared composer core: slash-command suggestions,
//! running a command, and typed-mention resolution.
//!
//! A command that did not run is not a Tauri error: it comes back in
//! `ComposerRunResponse::error` with user copy for the composer. Only a Core
//! failure (no session, poisoned state) is a rejected call.

use synara_core::app::composer::{
    CommandSuggestion, ComposerRunResponse, MentionCandidate, TypedMention,
};
use synara_core::core_api::{
    MatrixComposerCommandSuggestionsRequest, MatrixComposerResolveMentionsRequest,
    MatrixComposerRunCommandRequest,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

fn map_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    if error.category == MatrixIpcErrorCategory::Forbidden {
        return MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-composer-requires-session",
        );
    }
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix request failed.",
        "v-composer-failed",
    )
}

pub(crate) async fn command_suggestions(
    core: &Core,
    room_id: String,
    prefix: String,
) -> Result<Vec<CommandSuggestion>, MatrixAuthCommandError> {
    core.composer_command_suggestions(MatrixComposerCommandSuggestionsRequest { room_id, prefix })
        .await
        .map_err(map_error)
}

pub(crate) async fn run_command(
    core: &Core,
    room_id: String,
    text: String,
    confirmed: bool,
) -> Result<ComposerRunResponse, MatrixAuthCommandError> {
    core.composer_run_command(MatrixComposerRunCommandRequest {
        room_id,
        text,
        confirmed,
    })
    .await
    .map(ComposerRunResponse::from)
    .map_err(map_error)
}

pub(crate) fn resolve_mentions(
    core: &Core,
    text: String,
    members: Vec<MentionCandidate>,
    own_user_id: String,
) -> Vec<TypedMention> {
    core.composer_resolve_mentions(MatrixComposerResolveMentionsRequest {
        text,
        members,
        own_user_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_failures_keep_closed_diagnostics() {
        let mapped = map_error(MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden));
        assert_eq!(mapped.code, "Forbidden");
        assert_eq!(mapped.diagnostic_id, "v-composer-requires-session");
        let mapped = map_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("p2-core-state-poisoned"),
        );
        assert_eq!(mapped.diagnostic_id, "v-composer-failed");
    }
}
