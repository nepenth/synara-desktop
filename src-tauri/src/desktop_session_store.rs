use keyring::{Entry, Error as KeyringError};

use crate::desktop_secret_store::{
    platform_secret_store_status, secret_store_operation_error_code, DesktopSecretStoreStatus,
    DESKTOP_SESSION_CREDENTIAL_ACCOUNT, DESKTOP_SESSION_CREDENTIAL_SERVICE,
    DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE,
};
use crate::desktop_session::{
    current_timestamp_ms, sanitize_session_envelope, session_envelope_is_expired,
    DesktopSessionEnvelope, DESKTOP_STORED_SESSION_INVALID,
};

pub(crate) trait DesktopSessionSecretStore {
    fn status(&self) -> DesktopSecretStoreStatus;
    fn get_secret(&self) -> Result<Option<String>, String>;
    fn set_secret(&self, secret: &str) -> Result<bool, String>;
    fn remove_secret(&self) -> Result<bool, String>;
}

pub(crate) struct KeyringDesktopSessionSecretStore;

impl KeyringDesktopSessionSecretStore {
    fn session_entry(&self) -> Result<Entry, String> {
        self.session_entry_for_service(DESKTOP_SESSION_CREDENTIAL_SERVICE)
    }

    fn legacy_session_entry(&self) -> Result<Entry, String> {
        self.session_entry_for_service(DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE)
    }

    fn session_entry_for_service(&self, service: &str) -> Result<Entry, String> {
        Entry::new(service, DESKTOP_SESSION_CREDENTIAL_ACCOUNT)
            .map_err(|error| map_keyring_error("create-entry", error))
    }
}

impl DesktopSessionSecretStore for KeyringDesktopSessionSecretStore {
    fn status(&self) -> DesktopSecretStoreStatus {
        platform_secret_store_status()
    }

    fn get_secret(&self) -> Result<Option<String>, String> {
        if !self.status().can_persist_session {
            return Ok(None);
        }

        match self.session_entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => match self.legacy_session_entry()?.get_password() {
                Ok(secret) => {
                    self.session_entry()?
                        .set_password(&secret)
                        .map_err(|error| map_keyring_error("migrate-session", error))?;
                    let _ = self.legacy_session_entry()?.delete_credential();
                    Ok(Some(secret))
                }
                Err(KeyringError::NoEntry) => Ok(None),
                Err(error) => Err(map_keyring_error("read-legacy-session", error)),
            },
            Err(error) => Err(map_keyring_error("read-session", error)),
        }
    }

    fn set_secret(&self, secret: &str) -> Result<bool, String> {
        if !self.status().can_persist_session {
            return Ok(false);
        }

        self.session_entry()?
            .set_password(secret)
            .map_err(|error| map_keyring_error("write-session", error))?;
        Ok(true)
    }

    fn remove_secret(&self) -> Result<bool, String> {
        if !self.status().can_persist_session {
            return Ok(false);
        }

        let legacy_removed = match self.legacy_session_entry()?.delete_credential() {
            Ok(()) => true,
            Err(KeyringError::NoEntry) => false,
            Err(error) => return Err(map_keyring_error("remove-legacy-session", error)),
        };

        match self.session_entry()?.delete_credential() {
            Ok(()) => Ok(true),
            Err(KeyringError::NoEntry) => Ok(legacy_removed),
            Err(error) => Err(map_keyring_error("remove-session", error)),
        }
    }
}

fn map_keyring_error(operation: &'static str, error: KeyringError) -> String {
    let code = secret_store_operation_error_code(&error);
    eprintln!("desktop secret store {operation} failed: code={code}");
    code.to_owned()
}

pub(crate) fn desktop_get_session_from_store(
    store: &impl DesktopSessionSecretStore,
) -> Result<Option<DesktopSessionEnvelope>, String> {
    desktop_get_session_from_store_at(store, current_timestamp_ms())
}

fn desktop_get_session_from_store_at(
    store: &impl DesktopSessionSecretStore,
    now_ms: u64,
) -> Result<Option<DesktopSessionEnvelope>, String> {
    if !store.status().can_persist_session {
        return Ok(None);
    }

    let Some(secret) = store.get_secret()? else {
        return Ok(None);
    };
    let session = serde_json::from_str::<DesktopSessionEnvelope>(&secret)
        .map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    let session = sanitize_session_envelope(session)
        .map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    if session_envelope_is_expired(&session, now_ms) {
        let _ = desktop_remove_session_from_store(store);
        return Ok(None);
    }

    Ok(Some(session))
}

pub(crate) fn desktop_set_session_in_store(
    store: &impl DesktopSessionSecretStore,
    session: DesktopSessionEnvelope,
) -> Result<bool, String> {
    let mut session = sanitize_session_envelope(session)?;
    session.stored_at_ms = Some(current_timestamp_ms());
    if !store.status().can_persist_session {
        return Ok(false);
    }

    let secret =
        serde_json::to_string(&session).map_err(|_| DESKTOP_STORED_SESSION_INVALID.to_owned())?;
    store.set_secret(&secret)
}

pub(crate) fn desktop_remove_session_from_store(
    store: &impl DesktopSessionSecretStore,
) -> Result<bool, String> {
    if !store.status().can_persist_session {
        return Ok(false);
    }

    store.remove_secret()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop_secret_store::{
        DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN, DESKTOP_SECRET_STORE_NOT_CONFIGURED,
        DESKTOP_SECRET_STORE_OPERATION_LOCKED, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE,
    };
    use std::sync::Mutex;

    fn valid_session_envelope() -> DesktopSessionEnvelope {
        DesktopSessionEnvelope {
            base_url: "https://matrix.example.org".to_owned(),
            user_id: "@alice:example.org".to_owned(),
            device_id: "DEVICEID".to_owned(),
            access_token: "access-token".to_owned(),
            session_generation: Some("generation-1".to_owned()),
            refresh_token: None,
            expires_in_ms: Some(3_600_000),
            stored_at_ms: None,
        }
    }

    fn available_test_secret_store_status() -> DesktopSecretStoreStatus {
        DesktopSecretStoreStatus {
            available: true,
            backend: DESKTOP_SECRET_STORE_BACKEND_MACOS_KEYCHAIN,
            can_persist_session: true,
            reason: None,
        }
    }

    struct TestSessionSecretStore {
        status: DesktopSecretStoreStatus,
        secret: Mutex<Option<String>>,
    }

    impl TestSessionSecretStore {
        fn available() -> Self {
            Self {
                status: available_test_secret_store_status(),
                secret: Mutex::new(None),
            }
        }

        fn unavailable() -> Self {
            Self {
                status: DesktopSecretStoreStatus {
                    available: false,
                    backend: crate::desktop_secret_store::DESKTOP_SECRET_STORE_BACKEND_NONE,
                    can_persist_session: false,
                    reason: Some(DESKTOP_SECRET_STORE_NOT_CONFIGURED),
                },
                secret: Mutex::new(None),
            }
        }

        fn with_secret(secret: String) -> Self {
            Self {
                status: available_test_secret_store_status(),
                secret: Mutex::new(Some(secret)),
            }
        }

        fn stored_secret(&self) -> Option<String> {
            self.secret.lock().expect("secret lock").clone()
        }
    }

    impl DesktopSessionSecretStore for TestSessionSecretStore {
        fn status(&self) -> DesktopSecretStoreStatus {
            self.status
        }

        fn get_secret(&self) -> Result<Option<String>, String> {
            Ok(self.stored_secret())
        }

        fn set_secret(&self, secret: &str) -> Result<bool, String> {
            *self.secret.lock().expect("secret lock") = Some(secret.to_owned());
            Ok(true)
        }

        fn remove_secret(&self) -> Result<bool, String> {
            Ok(self.secret.lock().expect("secret lock").take().is_some())
        }
    }

    struct FailingSessionSecretStore {
        status: DesktopSecretStoreStatus,
        set_error: String,
    }

    impl FailingSessionSecretStore {
        fn with_set_error(set_error: String) -> Self {
            Self {
                status: available_test_secret_store_status(),
                set_error,
            }
        }
    }

    impl DesktopSessionSecretStore for FailingSessionSecretStore {
        fn status(&self) -> DesktopSecretStoreStatus {
            self.status
        }

        fn get_secret(&self) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn set_secret(&self, _secret: &str) -> Result<bool, String> {
            Err(self.set_error.clone())
        }

        fn remove_secret(&self) -> Result<bool, String> {
            Ok(false)
        }
    }

    #[test]
    fn credential_store_names_are_stable_and_scoped() {
        assert_eq!(
            DESKTOP_SESSION_CREDENTIAL_SERVICE,
            "com.whylandcreative.synara.desktop"
        );
        assert_eq!(
            DESKTOP_SESSION_LEGACY_CREDENTIAL_SERVICE,
            "app.synara.desktop"
        );
        assert_eq!(DESKTOP_SESSION_CREDENTIAL_ACCOUNT, "matrix-session");
    }

    #[test]
    fn desktop_session_store_persists_and_reads_sanitized_session_envelopes() {
        let store = TestSessionSecretStore::available();
        let stored = desktop_set_session_in_store(
            &store,
            DesktopSessionEnvelope {
                base_url: " https://matrix.example.org ".to_owned(),
                user_id: " @alice:example.org ".to_owned(),
                device_id: " DEVICEID ".to_owned(),
                access_token: " access-token ".to_owned(),
                session_generation: Some(" generation-1 ".to_owned()),
                refresh_token: Some(" refresh-token ".to_owned()),
                expires_in_ms: Some(3_600_000),
                stored_at_ms: None,
            },
        )
        .expect("session should store");

        assert!(stored);
        let raw = store
            .stored_secret()
            .expect("session secret should be stored");
        assert!(!raw.contains("fallbackSdkStores"));
        let stored_json: serde_json::Value =
            serde_json::from_str(&raw).expect("stored session should be json");
        assert_eq!(stored_json["baseUrl"], "https://matrix.example.org");
        assert_eq!(stored_json["accessToken"], "access-token");

        let session = desktop_get_session_from_store(&store)
            .expect("session should read")
            .expect("session should exist");
        assert_eq!(session.base_url, "https://matrix.example.org");
        assert_eq!(session.user_id, "@alice:example.org");
        assert_eq!(session.device_id, "DEVICEID");
        assert_eq!(session.access_token, "access-token");
        assert_eq!(session.refresh_token.as_deref(), Some("refresh-token"));
    }

    #[test]
    fn desktop_session_store_returns_none_when_no_session_exists() {
        let store = TestSessionSecretStore::available();

        assert!(desktop_get_session_from_store(&store).unwrap().is_none());
    }

    #[test]
    fn desktop_session_store_skips_unavailable_backends() {
        let store = TestSessionSecretStore::unavailable();

        assert!(!desktop_set_session_in_store(&store, valid_session_envelope()).unwrap());
        assert!(desktop_get_session_from_store(&store).unwrap().is_none());
        assert!(!desktop_remove_session_from_store(&store).unwrap());
        assert_eq!(store.stored_secret(), None);
    }

    #[test]
    fn desktop_session_store_rejects_invalid_stored_json_without_echoing_secret() {
        let raw_secret = "not-json-access-token";
        let store = TestSessionSecretStore::with_secret(raw_secret.to_owned());

        let error = desktop_get_session_from_store(&store)
            .err()
            .expect("stored session should fail");

        assert_eq!(error, DESKTOP_STORED_SESSION_INVALID);
        assert!(!error.contains(raw_secret));
    }

    #[test]
    fn desktop_session_store_clears_expired_sessions_on_read() {
        let store = TestSessionSecretStore::available();
        let stored_at_ms = 1_000_000;
        let mut session = valid_session_envelope();
        session.stored_at_ms = Some(stored_at_ms);
        session.expires_in_ms = Some(60_000);

        let secret = serde_json::to_string(&session).expect("session should encode");
        store.set_secret(&secret).expect("session should store");
        assert!(
            desktop_get_session_from_store_at(&store, stored_at_ms + 30_000)
                .expect("session should read")
                .is_some()
        );
        assert!(
            desktop_get_session_from_store_at(&store, stored_at_ms + 10_000_000)
                .expect("expired session should read as none")
                .is_none()
        );
        assert_eq!(store.stored_secret(), None);
    }

    #[test]
    fn desktop_session_store_removes_existing_session() {
        let store = TestSessionSecretStore::with_secret(
            serde_json::to_string(&valid_session_envelope()).expect("session should encode"),
        );

        assert!(desktop_remove_session_from_store(&store).unwrap());
        assert_eq!(store.stored_secret(), None);
        assert!(!desktop_remove_session_from_store(&store).unwrap());
    }

    #[test]
    fn desktop_set_session_validates_payload_before_storage() {
        let store = TestSessionSecretStore::available();
        let mut session = valid_session_envelope();
        session.access_token = " ".to_owned();

        assert!(desktop_set_session_in_store(&store, session).is_err());
        assert_eq!(store.stored_secret(), None);
    }

    #[test]
    fn map_keyring_error_never_echoes_sensitive_payloads() {
        let secret_payload = r#"{"accessToken":"super-secret-token","baseUrl":"https://x"}"#;
        let error = map_keyring_error(
            "write-session",
            KeyringError::PlatformFailure(Box::new(std::io::Error::other(secret_payload))),
        );

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE);
        assert!(!error.contains("super-secret-token"));
        assert!(!error.contains("accessToken"));
    }

    #[test]
    fn desktop_set_session_reports_locked_when_keychain_is_locked() {
        let locked_error = map_keyring_error(
            "write-session",
            KeyringError::NoStorageAccess(Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "keychain locked",
            ))),
        );
        let store = FailingSessionSecretStore::with_set_error(locked_error);
        let session = valid_session_envelope();
        let access_token = session.access_token.clone();

        let error = desktop_set_session_in_store(&store, session)
            .err()
            .expect("set session should fail");

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_LOCKED);
        assert!(!error.contains(&access_token));
        assert!(!error.contains("access-token"));
    }

    #[test]
    fn desktop_set_session_error_never_contains_session_json() {
        let unavailable_error = map_keyring_error(
            "write-session",
            KeyringError::PlatformFailure(Box::new(std::io::Error::other(
                "secret service write failed",
            ))),
        );
        let store = FailingSessionSecretStore::with_set_error(unavailable_error);
        let session = valid_session_envelope();
        let session_json = serde_json::to_string(&session).expect("session envelope should encode");

        let error = desktop_set_session_in_store(&store, session)
            .err()
            .expect("set session should fail");

        assert_eq!(error, DESKTOP_SECRET_STORE_OPERATION_UNAVAILABLE);
        assert!(!error.contains(&session_json));
        assert!(!error.contains("access-token"));
        assert!(!error.contains("matrix.example.org"));
    }
}
