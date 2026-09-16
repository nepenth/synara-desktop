//! Desktop X.509 identity settings adapter.

pub const MATRIX_X509_IDENTITY_MARKER: &str = "matrix-x509-identity-experimental";

pub fn matrix_x509_identity_markers() -> &'static str {
    MATRIX_X509_IDENTITY_MARKER
}
