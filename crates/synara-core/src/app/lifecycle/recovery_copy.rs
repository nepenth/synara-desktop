//! Recovery / transition UX copy catalog (P3.8 harness foundation).
//!
//! Stable **message keys** for UI localization — never embeds tokens, store
//! paths with secrets, recovery keys, or event plaintext. Host maps keys →
//! localized strings.

/// Catalog of privacy-safe recovery / logout UX message keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryCopyKey {
    /// Soft logout completed; stores retained for next login.
    LogoutCompleteStoresRetained,
    /// Local wipe completed; user must re-authenticate.
    WipeCompleteReauthRequired,
    /// Remote logout of this device succeeded.
    RemoteLogoutThisDeviceOk,
    /// Remote logout of all devices succeeded.
    RemoteLogoutAllDevicesOk,
    /// Remote logout skipped (offline); local cleanup still applied.
    RemoteLogoutSkippedOffline,
    /// Remote logout failed; offer retry; legacy/local data retained.
    RemoteLogoutFailedRetry,
    /// Store corruption surfaced; never auto-delete (P0.7).
    StoreCorruptManualRecovery,
    /// Legacy JS session detected; clean-break reauth required (P3.7).
    LegacySessionReauthRequired,
    /// Failed transition preserved local data.
    TransitionFailedDataRetained,
}

impl RecoveryCopyKey {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LogoutCompleteStoresRetained => "logout_complete_stores_retained",
            Self::WipeCompleteReauthRequired => "wipe_complete_reauth_required",
            Self::RemoteLogoutThisDeviceOk => "remote_logout_this_device_ok",
            Self::RemoteLogoutAllDevicesOk => "remote_logout_all_devices_ok",
            Self::RemoteLogoutSkippedOffline => "remote_logout_skipped_offline",
            Self::RemoteLogoutFailedRetry => "remote_logout_failed_retry",
            Self::StoreCorruptManualRecovery => "store_corrupt_manual_recovery",
            Self::LegacySessionReauthRequired => "legacy_session_reauth_required",
            Self::TransitionFailedDataRetained => "transition_failed_data_retained",
        }
    }

    /// English default copy for tests / bootstrap (not a localization system).
    pub fn default_en(self) -> &'static str {
        match self {
            Self::LogoutCompleteStoresRetained => {
                "Signed out. Your encrypted history remains on this device for next sign-in."
            }
            Self::WipeCompleteReauthRequired => {
                "Local Matrix data removed. Sign in again to continue."
            }
            Self::RemoteLogoutThisDeviceOk => "This device was signed out on the server.",
            Self::RemoteLogoutAllDevicesOk => {
                "All devices were signed out on the server. Other sessions must sign in again."
            }
            Self::RemoteLogoutSkippedOffline => {
                "Could not reach the server. Local sign-out still completed."
            }
            Self::RemoteLogoutFailedRetry => {
                "Server sign-out failed. Your local data was kept. You can retry."
            }
            Self::StoreCorruptManualRecovery => {
                "Encrypted store looks damaged. Data was not deleted automatically. You can retry restore or wipe after confirming."
            }
            Self::LegacySessionReauthRequired => {
                "An older session was found. Sign in again to use the new Matrix engine. Tokens are not transferred."
            }
            Self::TransitionFailedDataRetained => {
                "Could not finish switching engines. Your previous local data is still available to retry."
            }
        }
    }

    pub const ALL: &'static [RecoveryCopyKey] = &[
        Self::LogoutCompleteStoresRetained,
        Self::WipeCompleteReauthRequired,
        Self::RemoteLogoutThisDeviceOk,
        Self::RemoteLogoutAllDevicesOk,
        Self::RemoteLogoutSkippedOffline,
        Self::RemoteLogoutFailedRetry,
        Self::StoreCorruptManualRecovery,
        Self::LegacySessionReauthRequired,
        Self::TransitionFailedDataRetained,
    ];
}

/// Resolve a recovery copy key to default English (host may override).
pub fn recovery_copy_en(key: RecoveryCopyKey) -> &'static str {
    key.default_en()
}

/// Pick UX copy after a remote-logout outcome (privacy-safe key only).
pub fn copy_for_remote_outcome(
    remote_succeeded: bool,
    remote_skipped: bool,
    scope: super::remote_policy::RemoteLogoutScope,
    local_policy: super::remote_policy::LocalCleanupPolicy,
) -> RecoveryCopyKey {
    use super::remote_policy::{LocalCleanupPolicy, RemoteLogoutScope};

    if remote_skipped {
        return RecoveryCopyKey::RemoteLogoutSkippedOffline;
    }
    if !remote_succeeded {
        return RecoveryCopyKey::RemoteLogoutFailedRetry;
    }
    match (scope, local_policy) {
        (RemoteLogoutScope::AllDevices, _) => RecoveryCopyKey::RemoteLogoutAllDevicesOk,
        (RemoteLogoutScope::ThisDevice, LocalCleanupPolicy::WipeAccountStore) => {
            RecoveryCopyKey::WipeCompleteReauthRequired
        }
        (RemoteLogoutScope::ThisDevice, LocalCleanupPolicy::LogoutRetainStores) => {
            RecoveryCopyKey::RemoteLogoutThisDeviceOk
        }
    }
}

#[cfg(test)]
mod privacy_tests {
    use super::*;

    #[test]
    fn default_en_never_mentions_tokens_or_keys() {
        for key in RecoveryCopyKey::ALL {
            let s = key.default_en().to_ascii_lowercase();
            assert!(!s.contains("access_token"), "{key:?}");
            assert!(!s.contains("refresh_token"), "{key:?}");
            assert!(!s.contains("recovery key"), "{key:?}");
            assert!(!s.contains("private key"), "{key:?}");
            assert!(!s.contains("sydent"), "{key:?}");
        }
    }
}
