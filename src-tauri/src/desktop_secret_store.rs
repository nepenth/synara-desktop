#[cfg(any(target_os = "macos", target_os = "linux"))]
use keyring::Entry;
use keyring::Error as KeyringError;
use serde::Serialize;
#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "linux")]
use std::time::Duration;

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

pub(crate) const DESKTOP_SESSION_CREDENTIAL_SERVICE: &str = "com.whylandcreative.synara.desktop";
const LEGACY_RENDERER_SESSION_CREDENTIALS: [(&str, &str); 2] = [
    (DESKTOP_SESSION_CREDENTIAL_SERVICE, "matrix-session"),
    ("app.synara.desktop", "matrix-session"),
];
#[cfg(target_os = "macos")]
const DESKTOP_SESSION_KEYCHAIN_PROBE_ACCOUNT: &str = "matrix-session-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SERVICE: &str =
    "com.whylandcreative.synara.desktop.secret-service-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_ACCOUNT: &str = "availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET: &str =
    "synara-secret-service-availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SERVICE: &str =
    "com.whylandcreative.synara.desktop.keyutils-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_ACCOUNT: &str = "availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET: &str = "synara-keyutils-availability-probe";
#[cfg(target_os = "linux")]
const DESKTOP_SECRET_STORE_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

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

#[cfg(target_os = "macos")]
static MACOS_SECRET_STORE_STATUS_CACHE: OnceLock<Mutex<Option<DesktopSecretStoreStatus>>> =
    OnceLock::new();

#[cfg(target_os = "macos")]
fn macos_secret_store_status_cached() -> DesktopSecretStoreStatus {
    let cache = MACOS_SECRET_STORE_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .expect("macos secret store status cache should not be poisoned");
    if let Some(status) = *guard {
        return status;
    }

    let status = macos_secret_store_status_from_probe(macos_keychain_probe());
    *guard = Some(status);
    status
}

#[cfg(target_os = "macos")]
fn macos_keychain_probe_entry() -> Result<Entry, KeyringError> {
    Entry::new(
        DESKTOP_SESSION_CREDENTIAL_SERVICE,
        DESKTOP_SESSION_KEYCHAIN_PROBE_ACCOUNT,
    )
}

#[cfg(all(test, target_os = "macos"))]
static MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE: Mutex<Option<Result<(), KeyringError>>> =
    Mutex::new(None);

#[cfg(all(test, target_os = "macos"))]
fn set_macos_keychain_probe_test_override(result: Option<Result<(), KeyringError>>) {
    if let Ok(mut guard) = MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE.lock() {
        *guard = result;
    }
}

#[cfg(all(test, target_os = "macos"))]
fn take_macos_keychain_probe_test_override() -> Option<Result<(), KeyringError>> {
    MACOS_KEYCHAIN_PROBE_TEST_OVERRIDE
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[cfg(all(test, target_os = "macos"))]
fn reset_macos_secret_store_status_cache_for_tests() {
    if let Some(cache) = MACOS_SECRET_STORE_STATUS_CACHE.get() {
        *cache
            .lock()
            .expect("macos secret store status cache should not be poisoned") = None;
    }
}

#[cfg(target_os = "macos")]
fn macos_keychain_probe() -> Result<(), KeyringError> {
    #[cfg(all(test, target_os = "macos"))]
    if let Some(override_result) = take_macos_keychain_probe_test_override() {
        return override_result;
    }

    let entry = macos_keychain_probe_entry()?;
    match entry.get_password() {
        Ok(_) | Err(KeyringError::NoEntry) => Ok(()),
        Err(error) => Err(error),
    }
}

/// Delete credential envelopes used by the retired renderer session API.
/// The native per-account vault uses a distinct service and remains the only
/// credential owner. Cleanup is retried after each proven login or restore.
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn clear_legacy_renderer_session_credentials() {
    for (service, account) in LEGACY_RENDERER_SESSION_CREDENTIALS {
        let Ok(entry) = Entry::new(service, account) else {
            continue;
        };
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => {}
            Err(_) => {
                // Never expose platform errors or invalidate a native session
                // that has already been persisted successfully.
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(crate) fn clear_legacy_renderer_session_credentials() {}

pub(crate) fn platform_secret_store_status() -> DesktopSecretStoreStatus {
    #[cfg(target_os = "macos")]
    {
        macos_secret_store_status_cached()
    }

    #[cfg(target_os = "linux")]
    {
        linux_secret_store_status_cached()
    }

    #[cfg(target_os = "windows")]
    {
        unavailable_secret_store_status(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        unavailable_secret_store_status(DESKTOP_SECRET_STORE_UNSUPPORTED_PLATFORM)
    }
}

#[cfg(target_os = "linux")]
trait LinuxSecretServiceProbe: Send {
    fn round_trip(&self) -> Result<(), KeyringError>;
}

#[cfg(target_os = "linux")]
struct LiveLinuxSecretServiceProbe;

#[cfg(target_os = "linux")]
impl LinuxSecretServiceProbe for LiveLinuxSecretServiceProbe {
    fn round_trip(&self) -> Result<(), KeyringError> {
        #[cfg(test)]
        if let Some(override_result) = take_linux_secret_service_probe_test_override() {
            return override_result;
        }

        linux_secret_service_probe_round_trip()
    }
}

#[cfg(all(test, target_os = "linux"))]
static LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE: Mutex<Option<Result<(), KeyringError>>> =
    Mutex::new(None);

#[cfg(all(test, target_os = "linux"))]
fn set_linux_secret_service_probe_test_override(result: Option<Result<(), KeyringError>>) {
    if let Ok(mut guard) = LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE.lock() {
        *guard = result;
    }
}

#[cfg(all(test, target_os = "linux"))]
fn take_linux_secret_service_probe_test_override() -> Option<Result<(), KeyringError>> {
    LINUX_SECRET_SERVICE_PROBE_TEST_OVERRIDE
        .lock()
        .ok()
        .and_then(|mut guard| guard.take())
}

#[cfg(all(test, target_os = "linux"))]
fn reset_linux_secret_store_status_cache_for_tests() {
    if let Some(cache) = LINUX_SECRET_STORE_STATUS_CACHE.get() {
        *cache
            .lock()
            .expect("linux secret store status cache should not be poisoned") = None;
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
enum LinuxSecretServiceProbeError {
    Timeout,
    #[allow(dead_code)]
    Keyring(KeyringError),
}

#[cfg(target_os = "linux")]
fn linux_secret_service_probe_with_timeout(
    probe: impl LinuxSecretServiceProbe + 'static,
) -> Result<(), LinuxSecretServiceProbeError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(probe.round_trip());
    });

    match rx.recv_timeout(DESKTOP_SECRET_STORE_PROBE_TIMEOUT) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(LinuxSecretServiceProbeError::Keyring(error)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(LinuxSecretServiceProbeError::Timeout)
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_secret_service_unavailable_reason_from_probe_error(
    error: &LinuxSecretServiceProbeError,
) -> &'static str {
    match error {
        LinuxSecretServiceProbeError::Timeout | LinuxSecretServiceProbeError::Keyring(_) => {
            DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_secret_service_probe_round_trip() -> Result<(), KeyringError> {
    let entry = Entry::new(
        DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SERVICE,
        DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_ACCOUNT,
    )?;

    entry.set_password(DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET)?;
    let stored = entry.get_password()?;
    if stored != DESKTOP_SECRET_STORE_SECRET_SERVICE_PROBE_SECRET {
        return Err(KeyringError::PlatformFailure(
            std::io::Error::other("linux secret service probe round-trip mismatch").into(),
        ));
    }

    let _ = entry.delete_credential();
    Ok(())
}

#[cfg(target_os = "linux")]
fn has_linux_dbus_session_bus() -> bool {
    env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some()
        || env::var_os("XDG_RUNTIME_DIR")
            .map(|runtime_dir| Path::new(&runtime_dir).join("bus").exists())
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn linux_secret_store_status_from_live_probes(
    secret_service_probe: impl LinuxSecretServiceProbe + 'static,
) -> DesktopSecretStoreStatus {
    let mut unavailable_reason = None;
    let has_secret_service = if !has_linux_dbus_session_bus() {
        false
    } else {
        match linux_secret_service_probe_with_timeout(secret_service_probe) {
            Ok(()) => true,
            Err(error) => {
                unavailable_reason = Some(
                    linux_secret_service_unavailable_reason_from_probe_error(&error),
                );
                false
            }
        }
    };

    linux_secret_store_status_from_signals_with_reason(
        has_secret_service,
        has_linux_keyutils_backend(),
        unavailable_reason,
    )
}

#[cfg(target_os = "linux")]
static LINUX_SECRET_STORE_STATUS_CACHE: OnceLock<Mutex<Option<DesktopSecretStoreStatus>>> =
    OnceLock::new();

#[cfg(target_os = "linux")]
fn linux_secret_store_status_cached() -> DesktopSecretStoreStatus {
    let cache = LINUX_SECRET_STORE_STATUS_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .expect("linux secret store status cache should not be poisoned");
    if let Some(status) = *guard {
        return status;
    }

    let status = linux_secret_store_status_from_live_probes(LiveLinuxSecretServiceProbe);
    *guard = Some(status);
    status
}

#[cfg(all(target_os = "linux", test))]
fn dir_has_fragment(path: &str, fragment: &str) -> bool {
    let mut entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    let fragment = fragment.to_ascii_lowercase();
    entries.any(|entry| {
        let Ok(entry) = entry else {
            return false;
        };
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        name.contains(&fragment)
    })
}

#[cfg(all(target_os = "linux", test))]
fn has_secret_service_backend_service_files() -> bool {
    dir_has_fragment("/usr/share/dbus-1/services", "org.freedesktop.secrets")
        || dir_has_fragment("/usr/share/dbus-1/services", "gnome-keyring")
        || dir_has_fragment("/usr/share/dbus-1/services", "kwallet")
        || dir_has_fragment(
            "/usr/local/share/dbus-1/services",
            "org.freedesktop.secrets",
        )
        || dir_has_fragment("/usr/local/share/dbus-1/services", "gnome-keyring")
        || dir_has_fragment("/usr/local/share/dbus-1/services", "kwallet")
}

#[cfg(target_os = "linux")]
fn has_linux_keyutils_backend() -> bool {
    linux_keyutils_probe_passes()
}

#[cfg(target_os = "linux")]
fn linux_keyutils_probe_passes() -> bool {
    linux_keyutils_probe_round_trip().is_ok()
}

#[cfg(target_os = "linux")]
static LINUX_KEYUTILS_PROBE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(target_os = "linux")]
fn linux_keyutils_probe_round_trip() -> Result<(), KeyringError> {
    use keyring::credential::CredentialApi;
    use keyring::keyutils::KeyutilsCredential;

    let _probe_guard = LINUX_KEYUTILS_PROBE_LOCK.lock().map_err(|_| {
        KeyringError::PlatformFailure(
            std::io::Error::other("linux keyutils probe lock poisoned").into(),
        )
    })?;

    let credential = KeyutilsCredential::new_with_target(
        None,
        DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SERVICE,
        DESKTOP_SECRET_STORE_KEYUTILS_PROBE_ACCOUNT,
    )?;

    credential.set_password(DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET)?;
    let stored = credential.get_password()?;
    if stored != DESKTOP_SECRET_STORE_KEYUTILS_PROBE_SECRET {
        return Err(KeyringError::PlatformFailure(
            std::io::Error::other("linux keyutils probe round-trip mismatch").into(),
        ));
    }

    let _ = credential.delete_credential();
    Ok(())
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

    #[cfg(target_os = "macos")]
    #[test]
    fn platform_secret_store_status_probes_macos_keychain() {
        reset_macos_secret_store_status_cache_for_tests();
        set_macos_keychain_probe_test_override(None);
        let status = platform_secret_store_status();

        assert!(status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN);
        assert!(status.can_persist_session);
        assert_eq!(status.reason, None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn platform_secret_store_status_honors_injected_macos_keychain_probe_failure() {
        reset_macos_secret_store_status_cache_for_tests();
        set_macos_keychain_probe_test_override(Some(Err(KeyringError::NoStorageAccess(Box::new(
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "simulated locked keychain",
            ),
        )))));

        let status = platform_secret_store_status();

        assert!(!status.available);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_MACOS_KEYCHAIN_LOCKED)
        );

        reset_macos_secret_store_status_cache_for_tests();
        set_macos_keychain_probe_test_override(None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn platform_secret_store_status_reports_windows_unsupported() {
        let status = platform_secret_store_status();

        assert_eq!(status.available, false);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(status.can_persist_session, false);
        assert_eq!(
            status.reason,
            Some(DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED)
        );
        assert!(!bridge_supports_secure_secret_store(&status));
    }

    #[test]
    fn linux_secret_store_status_reports_unavailable_reason_from_probe_failure() {
        let status = linux_secret_store_status_from_signals_with_reason(
            false,
            false,
            Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
        );

        assert!(!status.available);
        assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
        assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));
    }

    #[cfg(target_os = "linux")]
    mod linux_secret_service_probe_tests {
        use super::*;
        use std::sync::Mutex;
        use std::time::Instant;

        static ENV_LOCK: Mutex<()> = Mutex::new(());

        struct DbusSessionEnvGuard {
            original_dbus: Option<std::ffi::OsString>,
            original_runtime: Option<std::ffi::OsString>,
        }

        impl DbusSessionEnvGuard {
            fn with_fake_session_bus() -> Self {
                let _env_guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
                let original_dbus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS");
                let original_runtime = std::env::var_os("XDG_RUNTIME_DIR");
                std::env::set_var(
                    "DBUS_SESSION_BUS_ADDRESS",
                    "unix:path=/tmp/synara-secret-service-probe-test-bus",
                );
                std::env::remove_var("XDG_RUNTIME_DIR");
                drop(_env_guard);

                Self {
                    original_dbus,
                    original_runtime,
                }
            }
        }

        impl Drop for DbusSessionEnvGuard {
            fn drop(&mut self) {
                let _env_guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
                if let Some(value) = self.original_dbus.take() {
                    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", value);
                } else {
                    std::env::remove_var("DBUS_SESSION_BUS_ADDRESS");
                }
                if let Some(value) = self.original_runtime.take() {
                    std::env::set_var("XDG_RUNTIME_DIR", value);
                } else {
                    std::env::remove_var("XDG_RUNTIME_DIR");
                }
            }
        }

        #[derive(Clone, Copy)]
        enum MockSecretServiceProbeOutcome {
            Success,
            Unavailable,
            Hangs,
        }

        struct MockLinuxSecretServiceProbe {
            outcome: MockSecretServiceProbeOutcome,
        }

        impl LinuxSecretServiceProbe for MockLinuxSecretServiceProbe {
            fn round_trip(&self) -> Result<(), KeyringError> {
                match self.outcome {
                    MockSecretServiceProbeOutcome::Success => Ok(()),
                    MockSecretServiceProbeOutcome::Unavailable => {
                        Err(KeyringError::PlatformFailure(
                            std::io::Error::other("simulated secret service unavailable").into(),
                        ))
                    }
                    MockSecretServiceProbeOutcome::Hangs => {
                        std::thread::sleep(DESKTOP_SECRET_STORE_PROBE_TIMEOUT * 3);
                        Ok(())
                    }
                }
            }
        }

        #[test]
        fn linux_secret_store_status_from_live_probe_reports_secret_service_when_probe_succeeds() {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Success,
            });

            assert!(status.available);
            assert_eq!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
            assert!(status.can_persist_session);
            assert_eq!(status.reason, None);
            assert!(bridge_supports_secure_secret_store(&status));
        }

        #[test]
        fn linux_secret_store_status_reports_unavailable_when_probe_fails_despite_service_files() {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_signals_with_reason(
                false,
                false,
                Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE),
            );

            assert!(!status.available);
            assert_eq!(status.backend, DESKTOP_SECRET_STORE_BACKEND_NONE);
            assert!(!status.can_persist_session);
            assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));

            let live_status =
                linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                    outcome: MockSecretServiceProbeOutcome::Unavailable,
                });
            assert_ne!(
                live_status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );

            if has_secret_service_backend_service_files() {
                assert!(
                    live_status.backend != DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE,
                    "service-file heuristic alone must not mark Secret Service available"
                );
            }
        }

        #[test]
        fn linux_secret_store_does_not_trust_service_file_heuristic_without_probe_success() {
            if !has_secret_service_backend_service_files() {
                return;
            }

            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            let status = linux_secret_store_status_from_live_probes(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Unavailable,
            });

            assert!(!status.can_persist_session);
            assert_ne!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
        }

        #[test]
        fn linux_secret_service_probe_respects_timeout() {
            let started = Instant::now();
            let result = linux_secret_service_probe_with_timeout(MockLinuxSecretServiceProbe {
                outcome: MockSecretServiceProbeOutcome::Hangs,
            });

            assert!(matches!(result, Err(LinuxSecretServiceProbeError::Timeout)));
            assert!(
                started.elapsed() < DESKTOP_SECRET_STORE_PROBE_TIMEOUT * 2,
                "probe should fail within the configured timeout window"
            );
        }

        #[test]
        fn platform_secret_store_status_honors_injected_secret_service_probe_failure() {
            let _dbus = DbusSessionEnvGuard::with_fake_session_bus();
            reset_linux_secret_store_status_cache_for_tests();
            set_linux_secret_service_probe_test_override(Some(Err(KeyringError::PlatformFailure(
                std::io::Error::other("simulated stopped secret service daemon").into(),
            ))));

            let status = platform_secret_store_status();

            assert!(!status.can_persist_session);
            assert_ne!(
                status.backend,
                DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            );
            if !has_linux_keyutils_backend() {
                assert_eq!(status.reason, Some(DESKTOP_SECRET_STORE_LINUX_UNAVAILABLE));
            }
        }

        #[test]
        fn platform_secret_store_status_probes_live_secret_service_when_accessible() {
            reset_linux_secret_store_status_cache_for_tests();
            set_linux_secret_service_probe_test_override(None);

            let status = platform_secret_store_status();

            if has_linux_dbus_session_bus()
                && status.backend == DESKTOP_SECRET_STORE_BACKEND_LINUX_SECRET_SERVICE
            {
                assert!(status.available);
                assert!(status.can_persist_session);
                assert_eq!(status.reason, None);
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn has_linux_keyutils_backend_matches_probe_not_proc_keys_existence() {
        let probe_passes = linux_keyutils_probe_passes();

        assert_eq!(has_linux_keyutils_backend(), probe_passes);

        if Path::new("/proc/keys").exists() && !probe_passes {
            assert!(
                !has_linux_keyutils_backend(),
                "/proc/keys alone must not mark keyutils as available"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_keyutils_probe_round_trip_is_non_destructive() {
        let first = linux_keyutils_probe_round_trip();
        let second = linux_keyutils_probe_round_trip();

        assert_eq!(first.is_ok(), second.is_ok());
    }
}
