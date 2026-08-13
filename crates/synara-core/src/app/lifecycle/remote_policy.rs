//! Remote-logout policy enums shared by the recovery-copy catalog.
//!
//! Policy enums live beside the recovery-copy catalog. The coordinator
//! (`RemoteLogoutFlow`) is extracted into this same Core module.

/// How local cleanup relates to remote logout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteLogoutScope {
    /// Invalidate only this device session on the homeserver.
    ThisDevice,
    /// Request server-side logout of all devices (destructive; host confirms).
    AllDevices,
}

impl RemoteLogoutScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ThisDevice => "this_device",
            Self::AllDevices => "all_devices",
        }
    }
}

/// Local cleanup policy after remote (or when remote is skipped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalCleanupPolicy {
    /// Drop client + clear session material; retain encrypted stores (soft logout).
    LogoutRetainStores,
    /// Exact-target local wipe of account store (hard reset).
    WipeAccountStore,
}

impl LocalCleanupPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LogoutRetainStores => "logout_retain_stores",
            Self::WipeAccountStore => "wipe_account_store",
        }
    }
}
