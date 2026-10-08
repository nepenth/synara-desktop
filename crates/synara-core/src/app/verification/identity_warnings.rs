//! Room-scoped warnings for members whose cryptographic identity changed.
//!
//! The SDK's `RoomIdentityState` decides which members are in pin violation
//! (an unverified identity changed) or verification violation (a previously
//! verified identity changed). Core projects only the closed kind, the member
//! id and an optional display name, and runs the two acknowledgements the SDK
//! offers: pin the new identity, or withdraw the old verification.

use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId, UserId};
use matrix_sdk::Client;
use matrix_sdk_crypto::{IdentityState, IdentityStatusChange, RoomIdentityState};
use serde::{Deserialize, Serialize};

pub const NATIVE_ROOM_IDENTITY_WARNINGS_SCHEMA_VERSION: u32 = 1;

/// Closed reason a member needs attention.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeIdentityWarningKind {
    /// A previously verified identity changed. Serious: verify again or
    /// withdraw the verification.
    VerificationViolation,
    /// An unverified identity changed since it was first seen.
    PinViolation,
}

impl NativeIdentityWarningKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VerificationViolation => "verification_violation",
            Self::PinViolation => "pin_violation",
        }
    }
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomIdentityWarning {
    pub user_id: String,
    /// Room display name when the member is known; presenters fall back to the id.
    #[cfg_attr(feature = "ts-export", ts(optional))]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub kind: NativeIdentityWarningKind,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomIdentityWarnings {
    pub schema_version: u32,
    pub room_id: String,
    /// Verification violations first, then pin violations, each by user id.
    pub warnings: Vec<NativeRoomIdentityWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRoomIdentityWarningsRequest {
    pub room_id: String,
}

/// How the user acknowledges a changed identity.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeIdentityWarningAction {
    /// Accept the new identity (`UserIdentity::pin`).
    Dismiss,
    /// Drop the old verification requirement (`UserIdentity::withdraw_verification`).
    WithdrawVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeIdentityWarningResolveRequest {
    pub room_id: String,
    pub user_id: String,
    pub action: NativeIdentityWarningAction,
}

/// Closed failure ids. No SDK error text crosses this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityWarningError {
    InvalidRoom,
    InvalidUser,
    RoomNotFound,
    IdentityUnavailable,
    ActionNotApplicable,
    StoreFailed,
}

impl IdentityWarningError {
    pub fn diagnostic_id(self) -> &'static str {
        match self {
            Self::InvalidRoom => "v-crypto.identity-warning-invalid-room",
            Self::InvalidUser => "v-crypto.identity-warning-invalid-user",
            Self::RoomNotFound => "v-crypto.identity-warning-room-not-found",
            Self::IdentityUnavailable => "v-crypto.identity-warning-identity-unavailable",
            Self::ActionNotApplicable => "v-crypto.identity-warning-action-not-applicable",
            Self::StoreFailed => "v-crypto.identity-warning-store-failed",
        }
    }
}

/// Project the SDK's current room identity states into warnings.
///
/// Only pin and verification violations need attention; verified and pinned
/// members never appear. The signed-in user's own identity is excluded.
pub fn warnings_from_states(
    states: Vec<IdentityStatusChange>,
    own_user_id: Option<&UserId>,
) -> Vec<(OwnedUserId, NativeIdentityWarningKind)> {
    let mut warnings: Vec<(OwnedUserId, NativeIdentityWarningKind)> = states
        .into_iter()
        .filter(|change| Some(change.user_id.as_ref()) != own_user_id)
        .filter_map(|change| {
            let kind = match change.changed_to {
                IdentityState::VerificationViolation => {
                    NativeIdentityWarningKind::VerificationViolation
                }
                IdentityState::PinViolation => NativeIdentityWarningKind::PinViolation,
                IdentityState::Verified | IdentityState::Pinned => return None,
            };
            Some((change.user_id, kind))
        })
        .collect();
    warnings.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    warnings.dedup_by(|a, b| a.0 == b.0);
    warnings
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, IdentityWarningError> {
    OwnedRoomId::try_from(room_id).map_err(|_| IdentityWarningError::InvalidRoom)
}

pub async fn room_identity_warnings(
    client: &Client,
    room_id: &str,
) -> Result<NativeRoomIdentityWarnings, IdentityWarningError> {
    let parsed = parse_room_id(room_id)?;
    let room = client
        .get_room(&parsed)
        .ok_or(IdentityWarningError::RoomNotFound)?;
    let warnings = if room.encryption_state().is_encrypted() {
        let states = RoomIdentityState::new(room.clone()).await.current_state();
        let mut projected = Vec::new();
        for (user_id, kind) in warnings_from_states(states, client.user_id()) {
            let display_name = match room.get_member_no_sync(&user_id).await {
                Ok(Some(member)) => member.display_name().map(str::to_owned),
                _ => None,
            };
            projected.push(NativeRoomIdentityWarning {
                user_id: user_id.to_string(),
                display_name,
                kind,
            });
        }
        projected
    } else {
        Vec::new()
    };
    Ok(NativeRoomIdentityWarnings {
        schema_version: NATIVE_ROOM_IDENTITY_WARNINGS_SCHEMA_VERSION,
        room_id: parsed.to_string(),
        warnings,
    })
}

/// Acknowledge one member's changed identity, then return the room's warnings.
pub async fn resolve_identity_warning(
    client: &Client,
    request: NativeIdentityWarningResolveRequest,
) -> Result<NativeRoomIdentityWarnings, IdentityWarningError> {
    parse_room_id(&request.room_id)?;
    let user_id = OwnedUserId::try_from(request.user_id.as_str())
        .map_err(|_| IdentityWarningError::InvalidUser)?;
    let identity = client
        .encryption()
        .get_user_identity(&user_id)
        .await
        .map_err(|_| IdentityWarningError::StoreFailed)?
        .ok_or(IdentityWarningError::IdentityUnavailable)?;
    match request.action {
        NativeIdentityWarningAction::Dismiss => {
            if identity.has_verification_violation() {
                // Pinning does not clear a verification violation; the user
                // must verify again or withdraw the verification.
                return Err(IdentityWarningError::ActionNotApplicable);
            }
            identity
                .pin()
                .await
                .map_err(|_| IdentityWarningError::StoreFailed)?;
        }
        NativeIdentityWarningAction::WithdrawVerification => {
            if !identity.has_verification_violation() {
                return Err(IdentityWarningError::ActionNotApplicable);
            }
            identity
                .withdraw_verification()
                .await
                .map_err(|_| IdentityWarningError::StoreFailed)?;
        }
    }
    room_identity_warnings(client, &request.room_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::user_id;

    fn change(user: &UserId, state: IdentityState) -> IdentityStatusChange {
        IdentityStatusChange {
            user_id: user.to_owned(),
            changed_to: state,
        }
    }

    #[test]
    fn only_violations_become_warnings_and_own_user_is_excluded() {
        let me = user_id!("@me:example.org");
        let states = vec![
            change(user_id!("@verified:example.org"), IdentityState::Verified),
            change(user_id!("@pinned:example.org"), IdentityState::Pinned),
            change(user_id!("@zed:example.org"), IdentityState::PinViolation),
            change(user_id!("@amy:example.org"), IdentityState::PinViolation),
            change(
                user_id!("@bob:example.org"),
                IdentityState::VerificationViolation,
            ),
            change(me, IdentityState::VerificationViolation),
        ];
        let warnings = warnings_from_states(states, Some(me));
        assert_eq!(
            warnings,
            vec![
                (
                    user_id!("@bob:example.org").to_owned(),
                    NativeIdentityWarningKind::VerificationViolation
                ),
                (
                    user_id!("@amy:example.org").to_owned(),
                    NativeIdentityWarningKind::PinViolation
                ),
                (
                    user_id!("@zed:example.org").to_owned(),
                    NativeIdentityWarningKind::PinViolation
                ),
            ]
        );
    }

    #[test]
    fn warning_dtos_serialize_closed_kinds_and_actions() {
        let json = serde_json::to_value(NativeRoomIdentityWarnings {
            schema_version: NATIVE_ROOM_IDENTITY_WARNINGS_SCHEMA_VERSION,
            room_id: "!room:example.org".to_owned(),
            warnings: vec![NativeRoomIdentityWarning {
                user_id: "@bob:example.org".to_owned(),
                display_name: None,
                kind: NativeIdentityWarningKind::VerificationViolation,
            }],
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schemaVersion": 1,
                "roomId": "!room:example.org",
                "warnings": [{"userId": "@bob:example.org", "kind": "verification_violation"}],
            })
        );
        let request: NativeIdentityWarningResolveRequest =
            serde_json::from_value(serde_json::json!({
                "roomId": "!room:example.org",
                "userId": "@bob:example.org",
                "action": "withdraw_verification",
            }))
            .unwrap();
        assert_eq!(
            request.action,
            NativeIdentityWarningAction::WithdrawVerification
        );
        assert!(
            serde_json::from_value::<NativeIdentityWarningResolveRequest>(serde_json::json!({
                "roomId": "!room:example.org",
                "userId": "@bob:example.org",
                "action": "verify",
            }))
            .is_err()
        );
    }

    #[test]
    fn diagnostic_ids_are_closed_static_ids() {
        for error in [
            IdentityWarningError::InvalidRoom,
            IdentityWarningError::InvalidUser,
            IdentityWarningError::RoomNotFound,
            IdentityWarningError::IdentityUnavailable,
            IdentityWarningError::ActionNotApplicable,
            IdentityWarningError::StoreFailed,
        ] {
            assert!(error
                .diagnostic_id()
                .starts_with("v-crypto.identity-warning-"));
        }
    }
}
