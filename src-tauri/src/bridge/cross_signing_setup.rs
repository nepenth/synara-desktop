//! Desktop bridge for `matrix_cross_signing_setup` through `Core::command`.

use synara_core::app::cross_signing::NativeCrossSigningSetupResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn cross_signing_setup(
    core: &Core,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    let response = core
        .cross_signing_setup()
        .await
        .map_err(map_cross_signing_setup_core_error)?;
    Ok(response)
}

fn map_cross_signing_setup_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-crypto.2-cross-signing-bootstrap-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            let (code, message) = match diagnostic {
                "v-crypto.2-cross-signing-auth-unsupported" => (
                    "Forbidden",
                    "The homeserver requires an unsupported authentication step for cross-signing setup.",
                ),
                _ => (
                    "Forbidden",
                    "No native Matrix session is active.",
                )};
            MatrixAuthCommandError::new(
                code,
                message,
                if diagnostic == "v-crypto.2-cross-signing-auth-unsupported" {
                    diagnostic
                } else {
                    "v-crypto.2-cross-signing-requires-session"
                },
            )
        }
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native cross-signing setup could not be completed.",
            diagnostic,
        ),
    }
}
