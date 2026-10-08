//! SharedCore room identity-change warnings for iOS.

use super::*;
use crate::app::verification::{
    NativeIdentityWarningAction, NativeIdentityWarningResolveRequest, NativeRoomIdentityWarnings,
    NativeRoomIdentityWarningsRequest,
};

super::wire_enum::wire_enum! {
    pub enum IdentityWarningKindDto {
        VerificationViolation => "verification_violation",
        PinViolation => "pin_violation",
    }
}
super::wire_enum::wire_enum_from!(crate::app::verification::NativeIdentityWarningKind => IdentityWarningKindDto {
    VerificationViolation, PinViolation
});

super::wire_enum::wire_enum! {
    /// How to resolve an identity warning.
    pub enum IdentityWarningActionDto {
        Dismiss => "dismiss",
        WithdrawVerification => "withdraw_verification",
    }
}

impl From<IdentityWarningActionDto> for NativeIdentityWarningAction {
    fn from(action: IdentityWarningActionDto) -> Self {
        match action {
            IdentityWarningActionDto::Dismiss => Self::Dismiss,
            IdentityWarningActionDto::WithdrawVerification => Self::WithdrawVerification,
        }
    }
}

/// A room member whose identity changed. No key material.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomIdentityWarningDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub kind: IdentityWarningKindDto,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomIdentityWarningsDto {
    pub room_id: String,
    pub warnings: Vec<RoomIdentityWarningDto>,
}

/// Static fail-closed identity-warning error. `code` is a closed diagnostic id.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomIdentityWarningError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomIdentityWarningError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomIdentityWarningError {}

const IDENTITY_WARNING_IDS: [&str; 7] = [
    "v-crypto.identity-warning-requires-session",
    "v-crypto.identity-warning-invalid-room",
    "v-crypto.identity-warning-invalid-user",
    "v-crypto.identity-warning-room-not-found",
    "v-crypto.identity-warning-identity-unavailable",
    "v-crypto.identity-warning-action-not-applicable",
    "v-crypto.identity-warning-store-failed",
];

pub(super) fn map_identity_warning_core_error(error: MatrixIpcError) -> RoomIdentityWarningError {
    let code = error
        .diagnostic_id
        .as_deref()
        .and_then(|id| IDENTITY_WARNING_IDS.iter().find(|known| **known == id))
        .copied()
        .unwrap_or("v-crypto.identity-warning-failed");
    RoomIdentityWarningError::Failed {
        code: code.to_owned(),
        description: "The identity change could not be updated.".to_owned(),
    }
}

pub(super) fn room_identity_warnings_dto(
    warnings: NativeRoomIdentityWarnings,
) -> RoomIdentityWarningsDto {
    RoomIdentityWarningsDto {
        room_id: warnings.room_id,
        warnings: warnings
            .warnings
            .into_iter()
            .map(|warning| RoomIdentityWarningDto {
                user_id: warning.user_id,
                display_name: warning.display_name,
                kind: warning.kind.into(),
            })
            .collect(),
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Members of `room_id` whose cryptographic identity changed.
    pub async fn room_identity_warnings(
        &self,
        room_id: String,
    ) -> Result<RoomIdentityWarningsDto, RoomIdentityWarningError> {
        self.core
            .room_identity_warnings(NativeRoomIdentityWarningsRequest { room_id })
            .await
            .map(room_identity_warnings_dto)
            .map_err(map_identity_warning_core_error)
    }

    /// `dismiss` accepts the new identity; `withdraw_verification` drops an old
    /// verification. Returns the room's remaining warnings.
    pub async fn resolve_room_identity_warning(
        &self,
        room_id: String,
        user_id: String,
        action: IdentityWarningActionDto,
    ) -> Result<RoomIdentityWarningsDto, RoomIdentityWarningError> {
        let action = action.into();
        self.core
            .room_identity_warning_resolve(NativeIdentityWarningResolveRequest {
                room_id,
                user_id,
                action,
            })
            .await
            .map(room_identity_warnings_dto)
            .map_err(map_identity_warning_core_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::verification::{NativeIdentityWarningKind, NativeRoomIdentityWarning};

    #[test]
    fn warnings_project_closed_kinds() {
        let dto = room_identity_warnings_dto(NativeRoomIdentityWarnings {
            schema_version: 1,
            room_id: "!room:example.org".to_owned(),
            warnings: vec![NativeRoomIdentityWarning {
                user_id: "@bob:example.org".to_owned(),
                display_name: Some("Bob".to_owned()),
                kind: NativeIdentityWarningKind::PinViolation,
            }],
        });
        assert_eq!(dto.warnings[0].kind, "pin_violation");
        assert_eq!(dto.warnings[0].display_name.as_deref(), Some("Bob"));
    }

    #[test]
    fn unknown_errors_and_actions_fail_closed() {
        let RoomIdentityWarningError::Failed { code, .. } = map_identity_warning_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("@bob:example.org"),
        );
        assert_eq!(code, "v-crypto.identity-warning-failed");
        assert_eq!(IdentityWarningActionDto::from_wire("verify"), None);
        assert_eq!(
            NativeIdentityWarningAction::from(IdentityWarningActionDto::WithdrawVerification),
            NativeIdentityWarningAction::WithdrawVerification
        );
    }
}
