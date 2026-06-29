use keyring::Error as KeyringError;
use serde::Serialize;

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DesktopSecretStoreStatus {
    pub available: bool,
    pub backend: &'static str,
    pub can_persist_session: bool,
    pub reason: Option<&'static str>,
}

pub(crate) const DESKTOP_SECRET_STORE_BACKEND_NONE: &str = "none";
#[cfg(any(target_os = "macos", test))]
pub(crate) const DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN: &str = "macos-keychain";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE: &str = "linux-secret-service";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS: &str = "linux-keyutils";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_NOT_CONFIGURED: &str = "secure-secret-store-not-configured";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_UNSUPPORTED_PLATFORM: &str =
    "secure-secret-store-unsupported-platform";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED: &str =
    "windows-native-session-store-unsupported";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_SESSION_SCOPED: &str = "linux-keyutils-session-scoped";
#[allow(dead_code)]
pub(crate) const DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE: &str = "linux-secret-store-unavailable";
#[cfg(any(target_os = "macos", test))]
pub(crate) const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED: &str = "macos-keychain-locked";
#[cfg(any(target_os = "macos", test))]
pub(crate) const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED: &str =
    "macos-keychain-access-denied";
#[cfg(any(target_os = "macos", test))]
pub(crate) const DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE: &str =
    "macos-keychain-unavailable";

pub(crate) const DESKTOP_SECRET_STORE_OPERATION_LOCKED: &str = "desktop-secret-store-locked";
pub(crate) const DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE: &str =
    "desktop-secret-store-unavailable";
pub(crate) const DESKTOP_SECRET_STORE_OPERATION_DENIED: &str = "desktop-secret-store-denied";

#[allow(dead_code)]
pub(crate) fn unavailable_secret_store_status(reason: &'static str) -> DesktopSecretStoreStatus {
    DesktopSecretStoreStatus {
        available: false,
        backend: DESKTOP_SECRET_STORE_BACKEND_NONE,
        can_persist_session: false,
        reason: Some(reason),
    }
}

pub(crate) fn bridge_supports_secure_secret_store(status: &DesktopSecretStoreStatus) -> bool {
    status.available && status.can_persist_session
}

#[cfg(any(target_os = "macos", test))]
fn macos_keychain_error_indicates_access_denied(
    err: &(dyn std::error::Error + Send + Sync + 'static),
) -> bool {
    #[cfg(test)]
    {
        let message = err.to_string();
        if let Some(code) = message
            .strip_prefix("test keychain error ")
            .and_then(|value| value.parse::<i32>().ok())
        {
            return matches!(code, -25293 | -25308);
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(error) = err.downcast_ref::<security_framework::base::Error>() {
        return matches!(error.code(), -25293 | -25308);
    }

    false
}

#[cfg(any(target_os = "macos", test))]
fn secret_store_error_indicates_access_denied(
    err: &(dyn std::error::Error + Send + Sync + 'static),
) -> bool {
    if macos_keychain_error_indicates_access_denied(err) {
        return true;
    }

    #[cfg(test)]
    {
        let message = err.to_string();
        if message.starts_with("test secret store error denied") {
            return true;
        }
    }

    let message = err.to_string().to_lowercase();
    message.contains("access denied")
        || message.contains("permission denied")
        || message.contains("not authorized")
        || message.contains("auth denied")
}

#[cfg(not(any(target_os = "macos", test)))]
fn secret_store_error_indicates_access_denied(
    err: &(dyn std::error::Error + Send + Sync + 'static),
) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("access denied")
        || message.contains("permission denied")
        || message.contains("not authorized")
        || message.contains("auth denied")
}

pub(crate) fn secret_store_operation_error_code(error: &KeyringError) -> &'static str {
    match error {
        KeyringError::NoStorageAccess(err) => {
            if secret_store_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_OPERATION_DENIED
            } else {
                DESKTOP_SECRET_STORE_OPERATION_LOCKED
            }
        }
        KeyringError::PlatformFailure(err) => {
            if secret_store_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_OPERATION_DENIED
            } else {
                DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE
            }
        }
        _ => DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE,
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_unavailable_secret_store_status(reason: &'static str) -> DesktopSecretStoreStatus {
    DesktopSecretStoreStatus {
        available: false,
        backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
        can_persist_session: false,
        reason: Some(reason),
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_keychain_unavailable_reason(error: &KeyringError) -> &'static str {
    match error {
        KeyringError::NoStorageAccess(err) => {
            if macos_keychain_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED
            } else {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED
            }
        }
        KeyringError::PlatformFailure(err) => {
            if macos_keychain_error_indicates_access_denied(err.as_ref()) {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED
            } else {
                DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE
            }
        }
        _ => DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE,
    }
}

#[cfg(any(target_os = "macos", test))]
pub(crate) fn macos_secret_store_status_from_probe(
    probe: Result<(), KeyringError>,
) -> DesktopSecretStoreStatus {
    match probe {
        Ok(()) => DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
            can_persist_session: true,
            reason: None,
        },
        Err(error) => {
            macos_unavailable_secret_store_status(macos_keychain_unavailable_reason(&error))
        }
    }
}

#[allow(dead_code)]
pub(crate) fn linux_secret_store_status_from_signals(
    has_secret_service: bool,
    has_keyutils: bool,
) -> DesktopSecretStoreStatus {
    linux_secret_store_status_from_signals_with_reason(has_secret_service, has_keyutils, None)
}

#[allow(dead_code)]
pub(crate) fn linux_secret_store_status_from_signals_with_reason(
    has_secret_service: bool,
    has_keyutils: bool,
    secret_service_unavailable_reason: Option<&'static str>,
) -> DesktopSecretStoreStatus {
    if has_secret_service {
        return DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE,
            can_persist_session: true,
            reason: None,
        };
    }

    if has_keyutils {
        return DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS,
            can_persist_session: false,
            reason: Some(DESKTOP_SECRET_STORE_SESSION_SCOPED),
        };
    }

    unavailable_secret_store_status(
        secret_service_unavailable_reason.unwrap_or(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MacosKeychainTestError {
        code: i32,
    }

    impl std::fmt::Display for MacosKeychainTestError {
        fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(formatter, "test keychain error {}", self.code)
        }
    }

    impl std::error::Error for MacosKeychainTestError {}

    #[test]
    fn unavailable_secret_store_status_uses_stable_none_backend() {
        let status = unavailable_secret_store_status(DESKTOP_SECRET_STORE_NOT_CONFIGURED);

        assert!(!status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert!(!status.can_persist_session);
        assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_NOT_CONFIGURED));
    }

    #[test]
    fn bridge_supports_secure_secret_store_only_when_persistence_is_available() {
        let persistent = DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE,
            can_persist_session: true,
            reason: None,
        };
        let session_scoped = DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS,
            can_persist_session: false,
            reason: Some(DESKTOP_SECRET_STORE_SESSION_SCOPED),
        };
        let unavailable = unavailable_secret_store_status(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE);

        assert!(bridge_supports_secure_secret_store(&persistent));
        assert!(!bridge_supports_secure_secret_store(&session_scoped));
        assert!(!bridge_supports_secure_secret_store(&unavailable));
    }

    #[test]
    fn map_secret_store_operation_errors_to_stable_codes_without_payloads() {
        assert_eq!(
            secret_store_operation_error_code(&KeyringError::NoStorageAccess(Box::new(
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "keychain locked"),
            ))),
            DESKTOP_SECRET_STORE_OPERATION_LOCKED
        );
        assert_eq!(
            secret_store_operation_error_code(&KeyringError::NoStorageAccess(Box::new(
                std::io::Error::other("test secret store error denied by ACL"),
            ))),
            DESKTOP_SECRET_STORE_OPERATION_DENIED
        );
        assert_eq!(
            secret_store_operation_error_code(&KeyringError::PlatformFailure(Box::new(
                std::io::Error::other(
                    r#"secret service failed for {"accessToken":"super-secret-token"}"#,
                ),
            ))),
            DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE
        );
    }

    #[test]
    fn macos_secret_store_status_from_probe_maps_keychain_reasons() {
        let available = macos_secret_store_status_from_probe(Ok(()));
        assert!(available.available);
        assert_eq!(
            available.backend,
            DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN
        );
        assert!(available.can_persist_session);
        assert_eq!(available.reason, None);

        let locked =
            macos_secret_store_status_from_probe(Err(KeyringError::NoStorageAccess(Box::new(
                std::io::Error::new(std::io::ErrorKind::PermissionDenied, "keychain locked"),
            ))));
        assert!(!locked.available);
        assert_eq!(locked.backend, DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN);
        assert_eq!(
            locked.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED)
        );

        let denied = macos_secret_store_status_from_probe(Err(KeyringError::PlatformFailure(
            Box::new(MacosKeychainTestError { code: -25293 }),
        )));
        assert!(!denied.available);
        assert_eq!(
            denied.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_ACCESS_DENIED)
        );

        let unavailable = macos_secret_store_status_from_probe(Err(KeyringError::PlatformFailure(
            Box::new(std::io::Error::other("unexpected keychain failure")),
        )));
        assert!(!unavailable.available);
        assert_eq!(
            unavailable.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_UNAVAILABLE)
        );
    }

    #[test]
    fn linux_secret_store_status_classifies_persistent_and_session_scoped_backends() {
        let persistent = linux_secret_store_status_from_signals(true, true);
        assert!(persistent.available);
        assert_eq!(
            persistent.backend,
            DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
        );
        assert!(persistent.can_persist_session);
        assert_eq!(persistent.reason, None);

        let session_scoped = linux_secret_store_status_from_signals(false, true);
        assert!(session_scoped.available);
        assert_eq!(
            session_scoped.backend,
            DESKTOP_SECRET_STORE_BACKEND_LINUX_KEYUTILS
        );
        assert!(!session_scoped.can_persist_session);
        assert_eq!(
            session_scoped.reason,
            Some(DESKTOP_SECRET_STORE_SESSION_SCOPED)
        );

        let unavailable = linux_secret_store_status_from_signals_with_reason(
            false,
            false,
            Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
        );
        assert!(!unavailable.available);
        assert_eq!(unavailable.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(
            unavailable.reason,
            Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE)
        );
    }
}
