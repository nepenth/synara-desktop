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

#[cfg(test)]
mod tests {
    use super::*;

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
}
