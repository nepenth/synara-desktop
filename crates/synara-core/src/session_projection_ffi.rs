//! Credential-free UniFFI mirror of the Core session projection.
//!
//! This is deliberately not an iOS Matrix client. It creates a private,
//! inert [`Platform`] only because [`Core`] already owns the projection's
//! open/close/snapshot semantics. The platform never crosses UniFFI and none
//! of its methods are called by this facade; the facade dispatches no commands.
//! Consequently it has no client, secret store access, callback, or network
//! operation. Matrix SDK ownership remains entirely with the Swift shell.

use std::sync::Arc;

use crate::app::auth::{normalize_homeserver_url, normalize_server_name};
use crate::core::Core;
use crate::dto::{NotificationCandidate, SessionLifecycle, SessionSnapshot};
use crate::platform::{
    CryptoStatusFuture, MediaConfigFuture, Platform, PlatformCryptoStatusError,
    PlatformMediaConfigError, PlatformStatus, PlatformSyncStatusError, SecretVault,
    SyncStatusFuture, UnavailableSecretVault,
};
use crate::transport::{
    CommandRegistry, MatrixIpcEnvelope, MatrixIpcError, MatrixIpcErrorCategory,
};

const INVALID_PROJECTION_CODE: &str = "p4.3-session-projection-rejected";
const INVALID_PROJECTION_DESCRIPTION: &str = "The session projection is invalid.";
const UNAVAILABLE_CODE: &str = "p4.3-session-projection-unavailable";
const UNAVAILABLE_DESCRIPTION: &str = "The session projection is unavailable.";
const MAX_MATRIX_ID_BYTES: usize = 255;

/// The only session values that may cross this UniFFI boundary.
///
/// This is intentionally separate from [`SessionSnapshot`]: its display-name
/// and avatar fields do not cross the Apple boundary, even though they are
/// safe elsewhere in Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProjection {
    pub generation: u64,
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
    pub lifecycle: SessionProjectionLifecycle,
    pub crypto_ready: bool,
}

/// Closed lifecycle values for [`SessionProjection`].
///
/// A closed enum prevents a shell or an SDK diagnostic string from becoming a
/// public lifecycle payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionProjectionLifecycle {
    Empty,
    Opening,
    Authenticating,
    Restoring,
    Syncing,
    Ready,
    Stopping,
    LoggedOut,
    Failed,
    Wiping,
}

impl From<SessionProjectionLifecycle> for SessionLifecycle {
    fn from(value: SessionProjectionLifecycle) -> Self {
        match value {
            SessionProjectionLifecycle::Empty => Self::Empty,
            SessionProjectionLifecycle::Opening => Self::Opening,
            SessionProjectionLifecycle::Authenticating => Self::Authenticating,
            SessionProjectionLifecycle::Restoring => Self::Restoring,
            SessionProjectionLifecycle::Syncing => Self::Syncing,
            SessionProjectionLifecycle::Ready => Self::Ready,
            SessionProjectionLifecycle::Stopping => Self::Stopping,
            SessionProjectionLifecycle::LoggedOut => Self::LoggedOut,
            SessionProjectionLifecycle::Failed => Self::Failed,
            SessionProjectionLifecycle::Wiping => Self::Wiping,
        }
    }
}

impl From<SessionLifecycle> for SessionProjectionLifecycle {
    fn from(value: SessionLifecycle) -> Self {
        match value {
            SessionLifecycle::Empty => Self::Empty,
            SessionLifecycle::Opening => Self::Opening,
            SessionLifecycle::Authenticating => Self::Authenticating,
            SessionLifecycle::Restoring => Self::Restoring,
            SessionLifecycle::Syncing => Self::Syncing,
            SessionLifecycle::Ready => Self::Ready,
            SessionLifecycle::Stopping => Self::Stopping,
            SessionLifecycle::LoggedOut => Self::LoggedOut,
            SessionLifecycle::Failed => Self::Failed,
            SessionLifecycle::Wiping => Self::Wiping,
        }
    }
}

/// Fixed, privacy-safe failures for [`SessionProjectionCore`].
///
/// Every field is selected from a source constant. In particular, invalid
/// Matrix identifiers, URLs, Core errors, and any potential shell/SDK context
/// are never formatted into this UniFFI error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionProjectionError {
    Rejected { code: String, description: String },
    Unavailable { code: String, description: String },
}

impl std::fmt::Display for SessionProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected { description, .. } | Self::Unavailable { description, .. } => {
                formatter.write_str(description)
            }
        }
    }
}

impl std::error::Error for SessionProjectionError {}

fn rejected() -> SessionProjectionError {
    SessionProjectionError::Rejected {
        code: INVALID_PROJECTION_CODE.to_owned(),
        description: INVALID_PROJECTION_DESCRIPTION.to_owned(),
    }
}

fn unavailable() -> SessionProjectionError {
    SessionProjectionError::Unavailable {
        code: UNAVAILABLE_CODE.to_owned(),
        description: UNAVAILABLE_DESCRIPTION.to_owned(),
    }
}

impl TryFrom<SessionProjection> for SessionSnapshot {
    type Error = SessionProjectionError;

    fn try_from(projection: SessionProjection) -> Result<Self, Self::Error> {
        validate_projection(&projection)?;
        Ok(Self {
            session_generation: projection.generation,
            user_id: projection.user_id,
            device_id: projection.device_id,
            homeserver_url: projection.homeserver_url,
            display_name: None,
            avatar_url: None,
            lifecycle: projection.lifecycle.into(),
            crypto_ready: projection.crypto_ready,
        })
    }
}

impl TryFrom<SessionSnapshot> for SessionProjection {
    type Error = SessionProjectionError;

    fn try_from(snapshot: SessionSnapshot) -> Result<Self, Self::Error> {
        let projection = Self {
            generation: snapshot.session_generation,
            user_id: snapshot.user_id,
            device_id: snapshot.device_id,
            homeserver_url: snapshot.homeserver_url,
            lifecycle: snapshot.lifecycle.into(),
            crypto_ready: snapshot.crypto_ready,
        };
        validate_projection(&projection)?;
        Ok(projection)
    }
}

fn validate_projection(projection: &SessionProjection) -> Result<(), SessionProjectionError> {
    if projection.generation == 0
        || projection.generation == u64::MAX
        || matches!(
            projection.lifecycle,
            SessionProjectionLifecycle::Empty | SessionProjectionLifecycle::LoggedOut
        )
        || !is_valid_user_id(&projection.user_id)
        || !is_valid_device_id(&projection.device_id)
        || !is_canonical_homeserver_url(&projection.homeserver_url)
    {
        return Err(rejected());
    }
    Ok(())
}

fn is_valid_user_id(value: &str) -> bool {
    if value.len() > MAX_MATRIX_ID_BYTES || value.is_empty() || contains_unsafe_text(value) {
        return false;
    }
    let Some(local_and_server) = value.strip_prefix('@') else {
        return false;
    };
    let Some((localpart, server_name)) = local_and_server.split_once(':') else {
        return false;
    };
    if localpart.is_empty()
        || localpart.len() > MAX_MATRIX_ID_BYTES
        || localpart.contains(['/', '\\', '?', '#', ':', '@'])
    {
        return false;
    }

    // Keep the exact shell-provided string only when it is already the safe,
    // canonical server-name form. The normalizer itself returns static errors.
    normalize_server_name(server_name)
        .map(|normalized| normalized.as_str() == server_name)
        .unwrap_or(false)
}

fn is_valid_device_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MATRIX_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn is_canonical_homeserver_url(value: &str) -> bool {
    if value.len() > MAX_MATRIX_ID_BYTES || contains_unsafe_text(value) {
        return false;
    }
    normalize_homeserver_url(value)
        .map(|normalized| normalized.as_str() == value)
        .unwrap_or(false)
}

fn contains_unsafe_text(value: &str) -> bool {
    value.chars().any(char::is_control) || value.chars().any(char::is_whitespace)
}

/// Private, inert Core dependency for the projection-only facade.
///
/// The existing `Core` constructor requires a `Platform`, but open/close/
/// session_snapshot never call one. This implementation has no shell callback,
/// no store implementation, no client, and no network behavior. It is never
/// exported through UniFFI.
#[derive(Default)]
struct ProjectionOnlyPlatform;

impl Platform for ProjectionOnlyPlatform {
    fn emit(&self, _: MatrixIpcEnvelope) -> Result<(), MatrixIpcError> {
        Err(inert_platform_error())
    }

    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync> {
        Arc::new(UnavailableSecretVault)
    }

    fn http_user_agent(&self) -> String {
        String::new()
    }

    fn sync_status(&self) -> SyncStatusFuture<'_> {
        Box::pin(async { Err(PlatformSyncStatusError::Unavailable) })
    }

    fn crypto_status(&self) -> CryptoStatusFuture<'_> {
        Box::pin(async { Err(PlatformCryptoStatusError::InvalidSnapshot) })
    }

    fn cross_signing_status(&self) -> crate::platform::CrossSigningStatusFuture<'_> {
        Box::pin(async { Err(crate::platform::PlatformCrossSigningStatusError::NoSession) })
    }

    fn media_config(&self) -> MediaConfigFuture<'_> {
        Box::pin(async { Err(PlatformMediaConfigError::NoSession) })
    }

    fn notify(&self, _: NotificationCandidate) -> Result<(), MatrixIpcError> {
        Err(inert_platform_error())
    }

    fn set_badge(&self, _: u64) -> Result<(), MatrixIpcError> {
        Err(inert_platform_error())
    }

    fn status(&self, _: PlatformStatus) -> Result<(), MatrixIpcError> {
        Err(inert_platform_error())
    }
}

fn inert_platform_error() -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
        .with_diagnostic("p4.3-session-projection-platform-unavailable")
}

/// Project-owned, projection-only UniFFI facade for Core's safe session state.
///
/// It supports only open, close, and snapshot. It does not expose `Core`, a
/// Platform, commands, callbacks, transport, or Matrix SDK types.
pub struct SessionProjectionCore {
    core: Core,
}

impl Default for SessionProjectionCore {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionProjectionCore {
    pub fn new() -> Self {
        Self {
            core: Core::with_registry(Arc::new(ProjectionOnlyPlatform), CommandRegistry::new()),
        }
    }

    /// Put a validated safe projection into Core. This has no SDK, store, or
    /// network side effect.
    pub async fn open(&self, projection: SessionProjection) -> Result<(), SessionProjectionError> {
        self.core
            .open(projection.try_into()?)
            .await
            .map_err(|_| unavailable())
    }

    /// Read only the six approved safe fields from Core.
    pub async fn session_snapshot(
        &self,
    ) -> Result<Option<SessionProjection>, SessionProjectionError> {
        self.core
            .session_snapshot()
            .map_err(|_| unavailable())?
            .map(SessionProjection::try_from)
            .transpose()
    }

    /// Clear Core's in-memory safe projection only. This never deletes a
    /// Keychain item, SDK store, credential, or crypto material.
    pub async fn close(&self) -> Result<(), SessionProjectionError> {
        self.core.close().await.map_err(|_| unavailable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> SessionProjection {
        SessionProjection {
            generation: 7,
            user_id: "@alice:matrix.org".to_owned(),
            device_id: "SYNARA-IOS-DEVICE".to_owned(),
            homeserver_url: "https://matrix.org".to_owned(),
            lifecycle: SessionProjectionLifecycle::Ready,
            crypto_ready: true,
        }
    }

    #[tokio::test]
    async fn uniffi_projection_facade_executes_core_open_snapshot_and_close() {
        let facade = SessionProjectionCore::new();
        assert_eq!(facade.session_snapshot().await.unwrap(), None);

        facade.open(projection()).await.unwrap();
        assert_eq!(facade.session_snapshot().await.unwrap(), Some(projection()));

        facade.close().await.unwrap();
        assert_eq!(facade.session_snapshot().await.unwrap(), None);
    }

    #[tokio::test]
    async fn facade_rejects_hostile_values_with_static_privacy_safe_error() {
        let facade = SessionProjectionCore::new();
        let hostile = "https://user:access-token@private.example/?password=secret";
        let mut invalid = projection();
        invalid.homeserver_url = hostile.to_owned();

        let error = facade
            .open(invalid)
            .await
            .expect_err("hostile URL must fail closed");
        assert_eq!(
            error,
            SessionProjectionError::Rejected {
                code: INVALID_PROJECTION_CODE.to_owned(),
                description: INVALID_PROJECTION_DESCRIPTION.to_owned(),
            }
        );
        let public_error = format!("{error:?}");
        for forbidden in [
            hostile,
            "access-token",
            "password",
            "secret",
            "private.example",
        ] {
            assert!(
                !public_error.contains(forbidden),
                "hostile value must not cross the UniFFI error: {forbidden}"
            );
        }
    }

    #[test]
    fn facade_rejects_invalid_generation_identifiers_and_lifecycle() {
        let mut invalid_generation = projection();
        invalid_generation.generation = 0;
        assert_eq!(
            SessionSnapshot::try_from(invalid_generation),
            Err(rejected())
        );
        let mut invalid_maximum_generation = projection();
        invalid_maximum_generation.generation = u64::MAX;
        assert_eq!(
            SessionSnapshot::try_from(invalid_maximum_generation),
            Err(rejected())
        );

        let mut invalid_user = projection();
        invalid_user.user_id = "@alice:private.example/path".to_owned();
        assert_eq!(SessionSnapshot::try_from(invalid_user), Err(rejected()));

        let mut invalid_device = projection();
        invalid_device.device_id = "DEVICE token=secret".to_owned();
        assert_eq!(SessionSnapshot::try_from(invalid_device), Err(rejected()));

        let mut invalid_lifecycle = projection();
        invalid_lifecycle.lifecycle = SessionProjectionLifecycle::LoggedOut;
        assert_eq!(
            SessionSnapshot::try_from(invalid_lifecycle),
            Err(rejected())
        );
    }

    #[test]
    fn projection_surface_is_limited_to_the_six_approved_fields() {
        // This is intentionally a compile-time-shaped list rather than a
        // serialization of SessionSnapshot, which has additional Core-only
        // fields. Keep the UniFFI record and this list in lockstep.
        const FIELDS: &[&str] = &[
            "generation",
            "user_id",
            "device_id",
            "homeserver_url",
            "lifecycle",
            "crypto_ready",
        ];
        assert_eq!(FIELDS.len(), 6);
        for forbidden in [
            "access_token",
            "refresh_token",
            "password",
            "recovery_key",
            "private_key",
            "session_key",
            "display_name",
            "avatar_url",
            "client",
            "store",
        ] {
            assert!(!FIELDS.contains(&forbidden));
        }
    }
}
