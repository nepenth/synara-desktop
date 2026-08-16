//! Product deep-link / route resolution (P4.8 harness foundation).
//!
//! Pure parse/build of Synara app routes. No SDK, no dual-backend, no tokens.

use super::error::RouteError;

/// Resolved navigation target for the product shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteTarget {
    Home,
    Room {
        room_id: String,
        event_id: Option<String>,
        thread_root_id: Option<String>,
    },
    Space {
        space_id: String,
    },
    User {
        user_id: String,
    },
    Settings {
        section: Option<String>,
    },
    /// Unknown but well-formed path reserved for forward compatibility.
    Unknown {
        path: String,
    },
}

/// Parse a product path (no scheme/host) into a [`RouteTarget`].
///
/// Accepted examples:
/// - `/home`
/// - `/home/room/!id:server`
/// - `/home/room/!id:server/event/$eid`
/// - `/home/room/!id:server/thread/$root`
/// - `/home/space/!id:server`
/// - `/home/user/@alice:server`
/// - `/settings` or `/settings/security`
pub fn resolve_path(path: &str) -> Result<RouteTarget, RouteError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(RouteError::Invalid {
            diagnostic_id: "p4.8-empty-path",
        });
    }
    if path.contains("://") || path.contains('?') || path.contains('#') {
        // Strip query/fragment if present; reject absolute URLs with scheme
        // (host-side should pass path only).
        if path.contains("://") {
            return Err(RouteError::Invalid {
                diagnostic_id: "p4.8-absolute-url-not-supported",
            });
        }
    }
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let path = if path.len() > 1 && path.ends_with('/') {
        &path[..path.len() - 1]
    } else {
        path
    };

    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return Ok(RouteTarget::Home);
    }

    match segments.as_slice() {
        ["home"] => Ok(RouteTarget::Home),
        ["home", "room", room_id] => {
            validate_room_id(room_id)?;
            Ok(RouteTarget::Room {
                room_id: (*room_id).to_owned(),
                event_id: None,
                thread_root_id: None,
            })
        }
        ["home", "room", room_id, "event", event_id] => {
            validate_room_id(room_id)?;
            validate_event_id(event_id)?;
            Ok(RouteTarget::Room {
                room_id: (*room_id).to_owned(),
                event_id: Some((*event_id).to_owned()),
                thread_root_id: None,
            })
        }
        ["home", "room", room_id, "thread", thread_root_id] => {
            validate_room_id(room_id)?;
            validate_event_id(thread_root_id)?;
            Ok(RouteTarget::Room {
                room_id: (*room_id).to_owned(),
                event_id: None,
                thread_root_id: Some((*thread_root_id).to_owned()),
            })
        }
        ["home", "space", space_id] => {
            validate_room_id(space_id)?;
            Ok(RouteTarget::Space {
                space_id: (*space_id).to_owned(),
            })
        }
        ["home", "user", user_id] => {
            validate_user_id(user_id)?;
            Ok(RouteTarget::User {
                user_id: (*user_id).to_owned(),
            })
        }
        ["settings"] => Ok(RouteTarget::Settings { section: None }),
        ["settings", section] => {
            if section.is_empty()
                || !section
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return Err(RouteError::Invalid {
                    diagnostic_id: "p4.8-invalid-settings-section",
                });
            }
            Ok(RouteTarget::Settings {
                section: Some((*section).to_owned()),
            })
        }
        _ => Ok(RouteTarget::Unknown {
            path: path.to_owned(),
        }),
    }
}

/// Build a canonical product path from a resolved target.
pub fn build_path(target: &RouteTarget) -> Result<String, RouteError> {
    match target {
        RouteTarget::Home => Ok("/home".into()),
        RouteTarget::Room {
            room_id,
            event_id,
            thread_root_id,
        } => {
            validate_room_id(room_id)?;
            if let Some(ev) = event_id {
                validate_event_id(ev)?;
                Ok(format!("/home/room/{room_id}/event/{ev}"))
            } else if let Some(root) = thread_root_id {
                validate_event_id(root)?;
                Ok(format!("/home/room/{room_id}/thread/{root}"))
            } else {
                Ok(format!("/home/room/{room_id}"))
            }
        }
        RouteTarget::Space { space_id } => {
            validate_room_id(space_id)?;
            Ok(format!("/home/space/{space_id}"))
        }
        RouteTarget::User { user_id } => {
            validate_user_id(user_id)?;
            Ok(format!("/home/user/{user_id}"))
        }
        RouteTarget::Settings { section: None } => Ok("/settings".into()),
        RouteTarget::Settings {
            section: Some(section),
        } => {
            if section.is_empty() {
                return Err(RouteError::Invalid {
                    diagnostic_id: "p4.8-invalid-settings-section",
                });
            }
            Ok(format!("/settings/{section}"))
        }
        RouteTarget::Unknown { path } => {
            if path.is_empty() {
                return Err(RouteError::Invalid {
                    diagnostic_id: "p4.8-empty-path",
                });
            }
            Ok(path.clone())
        }
    }
}

fn validate_room_id(id: &str) -> Result<(), RouteError> {
    if id.is_empty() || !id.starts_with('!') {
        return Err(RouteError::Invalid {
            diagnostic_id: "p4.8-invalid-room-id",
        });
    }
    Ok(())
}

fn validate_event_id(id: &str) -> Result<(), RouteError> {
    if id.is_empty() || !id.starts_with('$') {
        return Err(RouteError::Invalid {
            diagnostic_id: "p4.8-invalid-event-id",
        });
    }
    Ok(())
}

fn validate_user_id(id: &str) -> Result<(), RouteError> {
    if id.is_empty() || !id.starts_with('@') {
        return Err(RouteError::Invalid {
            diagnostic_id: "p4.8-invalid-user-id",
        });
    }
    Ok(())
}
