//! Unit tests for the P7.7 Matrix media ownership boundary.

use super::*;

#[test]
fn default_inventory_is_exact_and_host_owned() {
    assert_eq!(
        default_rules(),
        [
            BoundaryRule {
                capability: SW_TOKEN_STORAGE,
                allowed_owner: MediaOwner::HostNative,
            },
            BoundaryRule {
                capability: SW_ENCRYPTED_MEDIA_DECRYPT,
                allowed_owner: MediaOwner::HostNative,
            },
            BoundaryRule {
                capability: SW_LONG_LIVED_MXC_SECRET_CACHE,
                allowed_owner: MediaOwner::HostNative,
            },
        ]
    );
}

#[test]
fn native_host_is_allowed_for_every_default_capability() {
    for rule in default_rules() {
        assert_eq!(
            assert_owner(rule.capability, MediaOwner::HostNative),
            Ok(())
        );
    }
}

#[test]
fn service_worker_is_forbidden_for_every_default_capability() {
    for rule in default_rules() {
        assert_eq!(
            assert_owner(rule.capability, MediaOwner::ServiceWorkerForbidden),
            Err(BoundaryError::ForbiddenOwner)
        );
    }
}

#[test]
fn unknown_capabilities_fail_closed_without_echoing_input() {
    let private_input = "access_token=must-not-appear";
    let error = assert_owner(private_input, MediaOwner::HostNative).unwrap_err();

    assert_eq!(error, BoundaryError::UnknownCapability);
    assert!(!format!("{error:?}").contains(private_input));
}

#[test]
fn marker_is_registered() {
    assert_eq!(
        matrix_media_boundary_markers(),
        MATRIX_MEDIA_BOUNDARY_MARKER
    );
}
