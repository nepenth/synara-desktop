//! Security / crypto status projection — no keys, secrets, or recovery material.

use serde::{Deserialize, Serialize};

/// Key-backup enablement projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupStatus {
    Unknown,
    Disabled,
    Enabled,
    Outdated,
}

/// Secret-storage / recovery setup projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStatus {
    Unknown,
    NotSetup,
    Ready,
    Incomplete,
}

/// Cross-signing / device verification projection for the local user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    Unverified,
    Verified,
    Unavailable,
}

/// Aggregate crypto / security status for settings and banners.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityStatus {
    pub cross_signing_active: bool,
    pub backup_status: BackupStatus,
    pub recovery_status: RecoveryStatus,
    pub verification_state: VerificationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_count: Option<u32>,
    pub has_pending_verification_requests: bool,
}
