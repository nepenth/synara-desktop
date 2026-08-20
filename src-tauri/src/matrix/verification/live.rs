//! Desktop adapter for the Core verification owner.
//!
//! `NativeVerificationOwner` lives in synara-core and returns privacy-safe
//! diagnostic ids. This file maps those ids onto `MatrixAuthCommandError`
//! and the existing `matrix-verification-updated` Tauri wake-up.

use std::sync::Arc;

use matrix_sdk::Client;
use tauri::{AppHandle, Emitter};

use crate::matrix::auth::product::MatrixAuthCommandError;

pub use synara_core::app::verification::{
    NativeVerificationInbox, NativeVerificationOwner, NativeVerificationRequest,
    NativeVerificationUpdateSignal, VERIFICATION_UPDATED_EVENT,
};

/// Start the Core owner and emit verification inbox wakeups on the Tauri event.
pub fn start(client: &Client, app: AppHandle, session_generation: u64) -> NativeVerificationOwner {
    NativeVerificationOwner::with_emit(
        client,
        Arc::new(move |signal: NativeVerificationUpdateSignal| {
            let _ = app.emit(VERIFICATION_UPDATED_EVENT, signal);
        }),
        session_generation,
    )
}

pub fn map_verification_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-crypto.1-start-requires-session" => ("Forbidden", "No native Matrix session is active."),
        "v-crypto.1-device-not-found" => (
            "NotFound",
            "The Matrix device is not available for verification.",
        ),
        "v-crypto.1-own-identity-unavailable" => (
            "Unsupported",
            "Device verification has not been set up for this account.",
        ),
        "v-crypto.1-flow-not-found" => {
            ("NotFound", "The verification request is no longer active.")
        }
        "v-crypto.1-sas-invalid-state" => (
            "InvalidRequest",
            "The verification request is not ready to compare.",
        ),
        "v-crypto.1-confirm-before-sas" | "v-crypto.1-sas-unavailable" => (
            "InvalidRequest",
            "The verification comparison is not ready.",
        ),
        "v-crypto.1-dismiss-active-flow" => (
            "InvalidRequest",
            "An active verification request must be cancelled before it is dismissed.",
        ),
        _ => ("Unknown", "Device verification could not be completed."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
