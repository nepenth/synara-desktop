//! Typed SharedCore operations and projections for room directory.

use super::*;

/// Static fail-closed directory-visibility-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
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

pub(super) fn directory_visibility_failed(
    code: &str,
    description: &'static str,
) -> DirectoryVisibilityCommandError {
    DirectoryVisibilityCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_directory_visibility_core_error(
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

pub(super) fn directory_visibility_envelope_payload(
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

pub(super) fn closed_directory_visibility(value: &str) -> Option<&'static str> {
    match value {
        "public" => Some("public"),
        "private" => Some("private"),
        _ => None,
    }
}

/// Static fail-closed directory-search-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
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

pub(super) fn directory_search_failed(
    code: &str,
    description: &'static str,
) -> DirectorySearchCommandError {
    DirectorySearchCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_directory_search_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> DirectorySearchCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            directory_search_failed(code, DIRECTORY_SEARCH_NO_SESSION_DESCRIPTION)
        }
        Some("v-rooms.directory-federation-forbidden") => directory_search_failed(
            "v-rooms.directory-federation-forbidden",
            "This server does not allow public room directory queries over federation. The remote homeserver must enable allow_public_rooms_over_federation.",
        ),
        Some("v-rooms.directory-server-not-found") => directory_search_failed(
            "v-rooms.directory-server-not-found",
            "That Matrix server was not found.",
        ),
        Some("v-rooms.directory-network-failed") => directory_search_failed(
            "v-rooms.directory-network-failed",
            "Could not reach the room directory. Check your connection and try again.",
        ),
        Some("v-rooms.directory-invalid-server") => directory_search_failed(
            "v-rooms.directory-invalid-server",
            "That is not a valid Matrix server name.",
        ),
        Some("v-rooms.directory-rate-limited") => directory_search_failed(
            "v-rooms.directory-rate-limited",
            "The room directory is rate-limited. Try again in a moment.",
        ),
        Some(code @ ("v-rooms.directory-invalid-hit" | "v-rooms.directory-hit-cap")) => {
            directory_search_failed(code, "The public room directory could not be loaded.")
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

pub(super) fn directory_search_envelope_payload(
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

pub(super) fn closed_directory_search_status(value: &str) -> Option<&'static str> {
    match value {
        "ready" => Some("ready"),
        "stale" => Some("stale"),
        "cancelled" => Some("cancelled"),
        _ => None,
    }
}

pub(super) fn closed_directory_room_type(value: &str) -> Option<&'static str> {
    match value {
        "room" => Some("room"),
        "space" => Some("space"),
        _ => None,
    }
}
