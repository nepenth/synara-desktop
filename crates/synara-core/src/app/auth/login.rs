//! Password login against an unauthenticated Matrix Rust SDK client (P3.2 / V-AUTH.2).
//!
//! Uses SDK APIs only under this module:
//! `client.matrix_auth().login_username(...).initial_device_display_name(...).await`.
//!
//! The desktop shell composes password login into the production Tauri command.
//! **V-AUTH.2** closed desktop `m.login.token` product login as
//! not retained (SSO token-completion UI was removed with V-AUTH.1; no standalone
//! token-login product surface remains). There is no dual-backend and no one-time
//! token product login path.
//!
//! Access/refresh tokens remain on the SDK `Client` after success. [`LoginResult`]
//! never carries access/refresh tokens or password — only privacy-safe identity
//! fields for harness/status. Optional host-side persistence after login is
//! [`crate::app::lifecycle::persist_session_after_login`] (P3.5).

use matrix_sdk::encryption::{CryptoStoreError, OlmError};
use matrix_sdk::{Client, Error as MatrixSdkError, HttpError};

use super::error::AuthError;
use super::{platform_device_display_name, DevicePlatform};

/// How the caller authenticated (privacy-safe discriminator; no secrets).
///
/// Desktop product login is password-only after V-AUTH.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoginMethodKind {
    Password,
}

impl LoginMethodKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Password => "password",
        }
    }
}

/// Privacy-safe outcome of a successful password login.
///
/// **Never** includes access_token, refresh_token, password, or one-time login token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginResult {
    pub user_id: String,
    pub device_id: String,
    pub homeserver_url: String,
    /// Device display name sent on the login request (D-NEW-DEVICE).
    pub device_display_name: String,
    pub method: LoginMethodKind,
}

/// Options for password login (P3.2).
#[derive(Debug, Clone, Default)]
pub struct LoginOptions {
    /// Initial device display name. Defaults to [`platform_device_display_name`].
    pub device_display_name: Option<String>,
    /// When true, request a refresh token from the homeserver (SDK `request_refresh_token`).
    /// Persistence of that token uses
    /// [`crate::app::lifecycle::persist_session_after_login`] (P3.5) after success.
    pub request_refresh_token: bool,
    /// Existing device id from the leftover local crypto store.
    ///
    /// Password login without this id asks the homeserver for a *new* device.
    /// Logout does not wipe the SQLite crypto account, so a new device collides
    /// with the leftover Olm account (`CryptoStoreError::MismatchedAccount`)
    /// and fails closed. Reusing the stored device id is the SDK-supported
    /// re-login path when the client still holds the matching keys.
    pub device_id: Option<String>,
}

impl LoginOptions {
    pub fn with_platform_device_name(platform: DevicePlatform) -> Self {
        Self {
            device_display_name: Some(platform.device_display_name().to_owned()),
            request_refresh_token: false,
            device_id: None,
        }
    }

    fn resolved_device_display_name(&self) -> String {
        self.device_display_name
            .clone()
            .unwrap_or_else(|| platform_device_display_name().to_owned())
    }
}

/// Log in with Matrix user id (or localpart) + password.
///
/// `client` must be an **unauthenticated** client from P2.3
/// [`crate::app::client_builder::build_unauthenticated_client`].
///
/// On success the SDK client holds the session (including tokens). The returned
/// [`LoginResult`] is privacy-safe for harness/status use.
pub async fn login_with_password(
    client: &Client,
    user_id_or_localpart: &str,
    password: &str,
    options: &LoginOptions,
) -> Result<LoginResult, AuthError> {
    let user = validate_user_id_or_localpart(user_id_or_localpart)?;
    validate_password_present(password)?;
    let device_display_name = options.resolved_device_display_name();
    validate_device_display_name(&device_display_name)?;

    let mut builder = client
        .matrix_auth()
        .login_username(user.as_str(), password)
        .initial_device_display_name(&device_display_name);

    if let Some(device_id) = options.device_id.as_deref() {
        let device_id = validate_device_id(device_id)?;
        builder = builder.device_id(&device_id);
    }

    if options.request_refresh_token {
        builder = builder.request_refresh_token();
    }

    let response = builder.send().await.map_err(map_login_sdk_error)?;

    Ok(LoginResult {
        user_id: response.user_id.to_string(),
        device_id: response.device_id.to_string(),
        homeserver_url: client.homeserver().to_string(),
        device_display_name,
        method: LoginMethodKind::Password,
    })
}

fn validate_user_id_or_localpart(raw: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-empty-user-id",
            reason: "user id is empty",
        });
    }
    if trimmed.len() > 255 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-user-id-too-long",
            reason: "user id exceeds length limit",
        });
    }
    // Reject obvious whitespace / control characters; full MXID shape is enforced
    // by the homeserver / SDK. Localparts are allowed.
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-user-id-invalid-chars",
            reason: "user id contains whitespace or control characters",
        });
    }
    Ok(trimmed.to_owned())
}

fn validate_password_present(password: &str) -> Result<(), AuthError> {
    if password.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-empty-password",
            reason: "password is empty",
        });
    }
    Ok(())
}

fn validate_device_id(raw: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-empty-device-id",
            reason: "device id is empty",
        });
    }
    if trimmed.len() > 255 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-device-id-too-long",
            reason: "device id exceeds length limit",
        });
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-device-id-invalid-chars",
            reason: "device id contains whitespace or control characters",
        });
    }
    Ok(trimmed.to_owned())
}

fn validate_device_display_name(name: &str) -> Result<(), AuthError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-empty-device-display-name",
            reason: "device display name is empty",
        });
    }
    if trimmed.len() > 128 {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.2-device-display-name-too-long",
            reason: "device display name exceeds length limit",
        });
    }
    Ok(())
}

/// Map SDK login errors to privacy-safe [`AuthError`] variants.
///
/// Uses structured UIA detection when available, then classifies from a
/// redacted structural scan of the error Display text. Never embeds password,
/// token, or raw homeserver error bodies into Display/diagnostic fields.
pub(crate) fn map_login_sdk_error(err: MatrixSdkError) -> AuthError {
    if err.as_uiaa_response().is_some() {
        return AuthError::InteractiveAuthRequired {
            diagnostic_id: "p3.2-login-uiaa-required",
        };
    }

    // Prefer structured Matrix error kinds. These are the only server-response
    // details we map; raw response text is never returned or logged.
    if let Some(kind) = err.client_api_error_kind() {
        let kind_debug = format!("{kind:?}");
        return classify_error_kind_debug(&kind_debug);
    }

    // A standard Matrix error kind was not available. Preserve only a static,
    // privacy-safe shape identifier so a support report can distinguish a
    // transport failure, non-Matrix response, store failure, or decoder issue
    // without exposing a URL, response body, password, or token.
    if let Some(error) = classify_login_sdk_error_shape(&err) {
        return error;
    }

    // Retain the existing redacted structural fallback for SDK variants that
    // are not representable by the public error enum on every feature set.
    let raw = format!("{err}");
    classify_login_message_fallback(&raw)
}

fn classify_login_sdk_error_shape(err: &MatrixSdkError) -> Option<AuthError> {
    let diagnostic_id = match err {
        MatrixSdkError::Http(error) => match error.as_ref() {
            HttpError::Reqwest(error) if error.is_timeout() => "p3.2-login-http-timeout",
            HttpError::Reqwest(error) if error.is_connect() => "p3.2-login-http-connect",
            HttpError::Reqwest(_) => "p3.2-login-http-request",
            // Matrix API errors with a recognized errcode return above. This
            // branch therefore means an unexpected/non-Matrix API response or
            // response decoder failure, not a server message we may disclose.
            HttpError::Api(_) => "p3.2-login-http-api-response",
            HttpError::IntoHttp(_) => "p3.2-login-http-request-build",
            HttpError::RefreshToken(_) => "p3.2-login-refresh-token",
            HttpError::Cached(_) => "p3.2-login-http-cached",
            #[cfg(target_os = "android")]
            HttpError::VerifierBuilder(_) => "p3.2-login-http-verifier",
        },
        MatrixSdkError::SerdeJson(_) => "p3.2-login-response-decode",
        MatrixSdkError::Io(_) => "p3.2-login-local-io",
        MatrixSdkError::Url(_) => "p3.2-login-url-parse",
        MatrixSdkError::Timeout => "p3.2-login-sdk-timeout",
        // Sub-classified store failures so a privacy-safe support report can
        // distinguish the distinct local failure classes (and their
        // remediation) without exposing any raw SDK text:
        // - CrossProcessLockError: another running instance holds the store
        //   lock (macOS users often see this after launching while a prior
        //   instance is still open, or after an unclean exit left a stale lock).
        // - CryptoStoreError/StateStore/BadCryptoStoreState: local store open,
        //   migration, or schema/cipher failure (e.g. leftover store created by
        //   a different app build or a changed store key).
        // - OlmError wrapping a leftover-store identity/pickle failure: the
        //   SDK opened the crypto store, then activate() found an Olm account
        //   for a different device (typical after logout, which does not wipe
        //   the store) or could not unpickle it. This is a reset candidate.
        // - NoOlmMachine / other Olm/Megolm errors: encryption engine could
        //   not be initialized; fails closed.
        MatrixSdkError::CrossProcessLockError(_) => "p3.2-login-store-locked",
        MatrixSdkError::CryptoStoreError(error)
            if leftover_crypto_store_requires_reset(error.as_ref()) =>
        {
            "p3.2-login-store-reset-required"
        }
        MatrixSdkError::CryptoStoreError(_)
        | MatrixSdkError::StateStore(_)
        | MatrixSdkError::BadCryptoStoreState => "p3.2-login-store-open-failed",
        MatrixSdkError::OlmError(error) if leftover_olm_store_requires_reset(error.as_ref()) => {
            "p3.2-login-store-reset-required"
        }
        MatrixSdkError::NoOlmMachine
        | MatrixSdkError::OlmError(_)
        | MatrixSdkError::MegolmError(_) => "p3.2-login-olm-unavailable",
        _ => return None,
    };

    let error = match diagnostic_id {
        "p3.2-login-http-timeout"
        | "p3.2-login-http-connect"
        | "p3.2-login-http-request"
        | "p3.2-login-sdk-timeout" => AuthError::Connectivity { diagnostic_id },
        "p3.2-login-http-request-build" | "p3.2-login-url-parse" => AuthError::InvalidInput {
            diagnostic_id,
            reason: "SDK could not construct the login request",
        },
        "p3.2-login-store-locked"
        | "p3.2-login-store-open-failed"
        | "p3.2-login-store-reset-required"
        | "p3.2-login-olm-unavailable"
        | "p3.2-login-local-io" => AuthError::SdkInvariant { diagnostic_id },
        _ => AuthError::Unknown { diagnostic_id },
    };
    Some(error)
}

fn classify_error_kind_debug(kind_debug: &str) -> AuthError {
    // `Debug` of ErrorKind is variant-shaped (e.g. "Forbidden", "LimitExceeded(...)").
    // Never include the original SDK error body.
    let k = kind_debug.to_ascii_lowercase();
    if k.starts_with("forbidden") || k.starts_with("unauthorized") {
        return AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected",
        };
    }
    if k.starts_with("userdeactivated") {
        return AuthError::UserDeactivated {
            diagnostic_id: "p3.2-login-user-deactivated",
        };
    }
    if k.starts_with("limitexceeded") {
        return AuthError::RateLimited {
            diagnostic_id: "p3.2-login-rate-limited",
            retry_after_ms: None,
        };
    }
    if k.starts_with("unknowntoken") || k.starts_with("missingtoken") {
        return AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-unknown-token",
        };
    }
    if k.starts_with("notfound") {
        return AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.2-login-endpoint-not-found",
        };
    }
    if k.starts_with("unrecognized") {
        return AuthError::UnsupportedCapability {
            diagnostic_id: "p3.2-login-unrecognized",
        };
    }
    if k.starts_with("weakpassword") || k.starts_with("invalidusername") {
        return AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected",
        };
    }
    if k.starts_with("unknown") {
        return AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-unknown-rejected",
        };
    }
    AuthError::Unknown {
        diagnostic_id: "p3.2-login-unknown",
    }
}

fn classify_login_message_fallback(message: &str) -> AuthError {
    let lower = message.to_ascii_lowercase();
    if looks_like_connectivity(&lower) {
        return AuthError::Connectivity {
            diagnostic_id: "p3.2-login-connectivity",
        };
    }
    if lower.contains("deactivated") {
        return AuthError::UserDeactivated {
            diagnostic_id: "p3.2-login-user-deactivated",
        };
    }
    if lower.contains("uiaa") || (lower.contains("unauthorized") && lower.contains("interaction")) {
        return AuthError::InteractiveAuthRequired {
            diagnostic_id: "p3.2-login-uiaa-required",
        };
    }
    if lower.contains("403")
        || lower.contains("forbidden")
        || lower.contains("401")
        || lower.contains("unauthorized")
        || lower.contains("invalid username")
        || lower.contains("invalid password")
        || lower.contains("m_forbidden")
    {
        return AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected",
        };
    }
    if lower.contains("429") || lower.contains("rate") || lower.contains("limit exceeded") {
        return AuthError::RateLimited {
            diagnostic_id: "p3.2-login-rate-limited",
            retry_after_ms: None,
        };
    }
    if lower.contains("502")
        || lower.contains("503")
        || lower.contains("504")
        || lower.contains("homeserver")
    {
        return AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.2-login-homeserver-unavailable",
        };
    }
    AuthError::Unknown {
        diagnostic_id: "p3.2-login-unknown",
    }
}

fn leftover_crypto_store_requires_reset(error: &CryptoStoreError) -> bool {
    matches!(
        error,
        CryptoStoreError::MismatchedAccount { .. }
            | CryptoStoreError::UnpicklingError
            | CryptoStoreError::Pickle(_)
            | CryptoStoreError::UnsupportedDatabaseVersion(_, _)
    )
}

fn leftover_olm_store_requires_reset(error: &OlmError) -> bool {
    match error {
        OlmError::Store(store_error) => leftover_crypto_store_requires_reset(store_error),
        _ => false,
    }
}

fn looks_like_connectivity(lower: &str) -> bool {
    lower.contains("dns")
        || lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("network")
        || lower.contains("offline")
        || lower.contains("tls")
        || lower.contains("certificate")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_user_and_password() {
        assert_eq!(
            validate_user_id_or_localpart("")
                .unwrap_err()
                .diagnostic_id(),
            "p3.2-empty-user-id"
        );
        assert_eq!(
            validate_password_present("").unwrap_err().diagnostic_id(),
            "p3.2-empty-password"
        );
    }

    #[test]
    fn desktop_login_method_is_password_only() {
        // V-AUTH.2: product does not retain m.login.token; only password remains.
        assert_eq!(LoginMethodKind::Password.as_str(), "password");
        let debug = format!("{:?}", LoginMethodKind::Password);
        assert!(!debug.to_ascii_lowercase().contains("token"));
    }

    #[test]
    fn accepts_mxid_and_localpart() {
        assert_eq!(
            validate_user_id_or_localpart("  @alice:example.org  ").unwrap(),
            "@alice:example.org"
        );
        assert_eq!(validate_user_id_or_localpart("alice").unwrap(), "alice");
    }

    #[test]
    fn login_result_has_no_token_fields() {
        // Structural privacy: LoginResult fields are only identity metadata.
        let result = LoginResult {
            user_id: "@alice:example.org".into(),
            device_id: "DEVICE".into(),
            homeserver_url: "https://matrix.example.org".into(),
            device_display_name: "Synara macOS".into(),
            method: LoginMethodKind::Password,
        };
        let debug = format!("{result:?}");
        assert!(!debug.contains("access_token"));
        assert!(!debug.contains("refresh_token"));
        assert!(!debug.contains("password"));
        assert_eq!(result.method.as_str(), "password");
    }

    #[test]
    fn fallback_classifier_never_echoes_secret_fragments() {
        let secret = "syt_super_secret_access_token_xyz";
        let err = classify_login_message_fallback(&format!(
            "M_FORBIDDEN: invalid password for {secret} password=hunter2"
        ));
        assert_eq!(err.diagnostic_id(), "p3.2-login-rejected");
        assert!(err.display_is_privacy_safe(&[secret, "hunter2", "password="]));
    }

    #[test]
    fn kind_debug_classifier_maps_common_variants() {
        assert_eq!(
            classify_error_kind_debug("Forbidden").diagnostic_id(),
            "p3.2-login-rejected"
        );
        assert_eq!(
            classify_error_kind_debug("UserDeactivated").diagnostic_id(),
            "p3.2-login-user-deactivated"
        );
        assert_eq!(
            classify_error_kind_debug("LimitExceeded(...)").diagnostic_id(),
            "p3.2-login-rate-limited"
        );
        assert!(classify_error_kind_debug("Forbidden")
            .display_is_privacy_safe(&["syt_token", "password=x"]));
    }

    #[test]
    fn options_default_uses_platform_name_resolution() {
        let opts = LoginOptions::default();
        let name = opts.resolved_device_display_name();
        assert!(name.starts_with("Synara "));
        let macos = LoginOptions::with_platform_device_name(DevicePlatform::MacOs);
        assert_eq!(macos.resolved_device_display_name(), "Synara macOS");
    }

    #[test]
    fn sdk_timeout_gets_a_static_privacy_safe_diagnostic() {
        let error = map_login_sdk_error(MatrixSdkError::Timeout);
        assert_eq!(error.diagnostic_id(), "p3.2-login-sdk-timeout");
        assert!(error.display_is_privacy_safe(&[
            "https://private.example",
            "password=hunter2",
            "access_token=secret",
        ]));
    }

    #[test]
    fn store_lock_error_maps_to_precise_static_diagnostic() {
        // Cross-process lock held by another instance -> distinct static id
        // (distinguishes "another Synara instance is running" from an invalid
        // password or a local store corruption; never leaks lock internals).
        let unlock_busy = matrix_sdk::cross_process_lock::CrossProcessLockError::Unobtained(
            matrix_sdk::cross_process_lock::CrossProcessLockUnobtained::Busy,
        );
        let err = map_login_sdk_error(MatrixSdkError::CrossProcessLockError(Box::new(unlock_busy)));
        assert_eq!(err.diagnostic_id(), "p3.2-login-store-locked");
        assert!(err.display_is_privacy_safe(&[
            "syt_token",
            "/Users/alice/Library/Application Support/Synara",
            "password=hunter2"
        ]));
    }

    #[test]
    fn store_open_failure_maps_to_precise_static_diagnostic() {
        let err = map_login_sdk_error(MatrixSdkError::CryptoStoreError(Box::new(
            matrix_sdk::encryption::CryptoStoreError::AccountUnset,
        )));
        assert_eq!(err.diagnostic_id(), "p3.2-login-store-open-failed");

        let state = map_login_sdk_error(MatrixSdkError::BadCryptoStoreState);
        assert_eq!(state.diagnostic_id(), "p3.2-login-store-open-failed");
    }

    #[test]
    fn olm_unavailable_maps_to_precise_static_diagnostic() {
        let err = map_login_sdk_error(MatrixSdkError::NoOlmMachine);
        assert_eq!(err.diagnostic_id(), "p3.2-login-olm-unavailable");
        assert!(err.display_is_privacy_safe(&["syt_token", "olm_session", "refresh_token=rrr"]));
    }

    #[test]
    fn leftover_crypto_account_mismatch_is_a_reset_candidate() {
        // Password login against a leftover Olm account (logout ≠ wipe) wraps
        // MismatchedAccount as OlmError::Store. That must arm archive recovery,
        // not the generic "olm unavailable" fail-closed id.
        let mismatch = OlmError::Store(CryptoStoreError::MismatchedAccount {
            expected: (
                ruma::OwnedUserId::try_from("@alice:example.org").expect("test mxid"),
                ruma::OwnedDeviceId::from("OLDDEV"),
            ),
            got: (
                ruma::OwnedUserId::try_from("@alice:example.org").expect("test mxid"),
                ruma::OwnedDeviceId::from("NEWDEV"),
            ),
        });
        let err = map_login_sdk_error(MatrixSdkError::OlmError(Box::new(mismatch)));
        assert_eq!(err.diagnostic_id(), "p3.2-login-store-reset-required");
        assert!(err.display_is_privacy_safe(&[
            "@alice:example.org",
            "OLDDEV",
            "NEWDEV",
            "syt_token",
            "password=hunter2",
        ]));

        let pickle = map_login_sdk_error(MatrixSdkError::OlmError(Box::new(OlmError::Store(
            CryptoStoreError::UnpicklingError,
        ))));
        assert_eq!(pickle.diagnostic_id(), "p3.2-login-store-reset-required");

        let direct = map_login_sdk_error(MatrixSdkError::CryptoStoreError(Box::new(
            CryptoStoreError::MismatchedAccount {
                expected: (
                    ruma::OwnedUserId::try_from("@alice:example.org").expect("test mxid"),
                    ruma::OwnedDeviceId::from("OLDDEV"),
                ),
                got: (
                    ruma::OwnedUserId::try_from("@alice:example.org").expect("test mxid"),
                    ruma::OwnedDeviceId::from("NEWDEV"),
                ),
            },
        )));
        assert_eq!(direct.diagnostic_id(), "p3.2-login-store-reset-required");
    }

    #[test]
    fn rejects_empty_or_oversized_device_id() {
        assert_eq!(
            validate_device_id("").unwrap_err().diagnostic_id(),
            "p3.2-empty-device-id"
        );
        assert_eq!(
            validate_device_id("   ").unwrap_err().diagnostic_id(),
            "p3.2-empty-device-id"
        );
        assert_eq!(
            validate_device_id(&"A".repeat(256))
                .unwrap_err()
                .diagnostic_id(),
            "p3.2-device-id-too-long"
        );
        assert_eq!(validate_device_id("DEVICEID").unwrap(), "DEVICEID");
    }
}
