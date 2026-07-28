//! Device list / trust projection (P8.2 harness foundation).
//!
//! Pure index of product device summaries. **No device keys, tokens, or
//! recovery material.** No SDK crypto APIs, no dual-backend.

use std::collections::HashMap;

use crate::matrix::dto::DeviceId;

use super::error::DeviceError;

/// Soft cap on tracked devices per account (UI/list safety).
pub const MAX_DEVICES: usize = 256;

/// Product device summary for settings / trust UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummary {
    pub device_id: DeviceId,
    pub display_name: Option<String>,
    /// Optional last activity timestamp (ms), privacy-safe.
    pub last_seen_ts: Option<u64>,
    pub is_verified: bool,
    /// True when this is the local device for the session.
    pub is_own: bool,
    pub is_deleted: bool,
}

/// Session-generation-stamped device index.
#[derive(Debug, Default)]
pub struct DeviceIndex {
    session_generation: u64,
    by_id: HashMap<DeviceId, DeviceSummary>,
    own_device_id: Option<DeviceId>,
}

impl DeviceIndex {
    pub fn new(session_generation: u64) -> Self {
        Self {
            session_generation,
            by_id: HashMap::new(),
            own_device_id: None,
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn len(&self) -> usize {
        self.by_id.values().filter(|d| !d.is_deleted).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn own_device_id(&self) -> Option<&str> {
        self.own_device_id.as_deref()
    }

    fn validate_id(device_id: &str) -> Result<(), DeviceError> {
        if device_id.is_empty()
            || !device_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return Err(DeviceError::Invalid {
                diagnostic_id: "p8.2-invalid-device-id",
            });
        }
        Ok(())
    }

    /// Upsert a device summary (host maps SDK → product shape).
    pub fn upsert(&mut self, mut device: DeviceSummary) -> Result<(), DeviceError> {
        Self::validate_id(&device.device_id)?;
        if !self.by_id.contains_key(&device.device_id)
            && self.by_id.values().filter(|d| !d.is_deleted).count() >= MAX_DEVICES
            && !device.is_deleted
        {
            return Err(DeviceError::Invalid {
                diagnostic_id: "p8.2-device-cap",
            });
        }
        if device.is_own {
            // Clear previous own flag.
            for d in self.by_id.values_mut() {
                d.is_own = false;
            }
            self.own_device_id = Some(device.device_id.clone());
        } else if self.own_device_id.as_deref() == Some(device.device_id.as_str()) {
            self.own_device_id = None;
        }
        device.is_own = self.own_device_id.as_deref() == Some(device.device_id.as_str());
        self.by_id.insert(device.device_id.clone(), device);
        Ok(())
    }

    pub fn get(&self, device_id: &str) -> Option<&DeviceSummary> {
        self.by_id.get(device_id).filter(|d| !d.is_deleted)
    }

    /// Active (non-deleted) devices; own first, then verified, then id.
    pub fn list_active(&self) -> Vec<&DeviceSummary> {
        let mut v: Vec<_> = self.by_id.values().filter(|d| !d.is_deleted).collect();
        v.sort_by(|a, b| {
            b.is_own
                .cmp(&a.is_own)
                .then_with(|| b.is_verified.cmp(&a.is_verified))
                .then_with(|| a.device_id.cmp(&b.device_id))
        });
        v
    }

    pub fn set_verified(&mut self, device_id: &str, verified: bool) -> Result<(), DeviceError> {
        let d = self.by_id.get_mut(device_id).ok_or(DeviceError::Invalid {
            diagnostic_id: "p8.2-unknown-device-id",
        })?;
        if d.is_deleted {
            return Err(DeviceError::Invalid {
                diagnostic_id: "p8.2-device-deleted",
            });
        }
        d.is_verified = verified;
        Ok(())
    }

    /// Soft-delete (mark deleted; keeps id for idempotent UI).
    pub fn mark_deleted(&mut self, device_id: &str) -> Result<(), DeviceError> {
        let d = self.by_id.get_mut(device_id).ok_or(DeviceError::Invalid {
            diagnostic_id: "p8.2-unknown-device-id",
        })?;
        d.is_deleted = true;
        d.is_own = false;
        if self.own_device_id.as_deref() == Some(device_id) {
            self.own_device_id = None;
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.by_id.clear();
        self.own_device_id = None;
    }

    /// Bump generation and wipe (logout / account switch).
    pub fn retire_generation(&mut self, new_generation: u64) {
        self.session_generation = new_generation;
        self.clear();
    }
}
