//! Unit tests for P8.2 device index.

use super::*;
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn device(id: &str, own: bool, verified: bool) -> DeviceSummary {
    DeviceSummary {
        device_id: id.into(),
        display_name: Some(format!("dev-{id}")),
        last_seen_ts: Some(1),
        is_verified: verified,
        is_own: own,
        is_deleted: false,
    }
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_devices_markers(), MATRIX_DEVICES_MARKER);
}

#[test]
fn upsert_list_order() {
    let mut idx = DeviceIndex::new(1);
    idx.upsert(device("OTHER", false, false)).unwrap();
    idx.upsert(device("OWN", true, true)).unwrap();
    idx.upsert(device("VER", false, true)).unwrap();
    let list = idx.list_active();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].device_id, "OWN");
    assert!(list[0].is_own);
    assert_eq!(list[1].device_id, "VER");
    assert!(list[1].is_verified);
    assert_eq!(idx.own_device_id(), Some("OWN"));
}

#[test]
fn set_verified_and_delete() {
    let mut idx = DeviceIndex::new(1);
    idx.upsert(device("D1", true, false)).unwrap();
    idx.set_verified("D1", true).unwrap();
    assert!(idx.get("D1").unwrap().is_verified);
    idx.mark_deleted("D1").unwrap();
    assert!(idx.get("D1").is_none());
    assert!(idx.is_empty());
    assert!(idx.own_device_id().is_none());
}

#[test]
fn invalid_id() {
    let mut idx = DeviceIndex::new(1);
    let err = idx
        .upsert(DeviceSummary {
            device_id: "".into(),
            display_name: None,
            last_seen_ts: None,
            is_verified: false,
            is_own: false,
            is_deleted: false,
        })
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.2-invalid-device-id");
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn cap_and_overwrite() {
    let mut idx = DeviceIndex::new(1);
    for i in 0..MAX_DEVICES {
        idx.upsert(device(&format!("D{i}"), false, false)).unwrap();
    }
    let err = idx.upsert(device("OVERFLOW", false, false)).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p8.2-device-cap");
    idx.upsert(device("D0", true, true)).unwrap();
    assert_eq!(idx.get("D0").unwrap().is_own, true);
}

#[test]
fn retire_generation() {
    let mut idx = DeviceIndex::new(2);
    idx.upsert(device("X", true, true)).unwrap();
    idx.retire_generation(3);
    assert_eq!(idx.session_generation(), 3);
    assert!(idx.is_empty());
}
