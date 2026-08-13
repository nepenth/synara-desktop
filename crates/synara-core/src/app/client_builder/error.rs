//! Privacy-safe client builder errors (no tokens / store keys / passphrases).

use std::fmt;

use crate::app::store::{AccountIdentityError, StorePathError};
use crate::app::supervisor::FactoryError;
use crate::transport::MatrixIpcErrorCategory;

/// Failure while validating config or constructing an unauthenticated Client.
#[derive(Debug)]
pub enum ClientBuilderError {
    Identity(AccountIdentityError),
    StorePath(StorePathError),
    InvalidConfig(&'static str),
    /// SDK construction failed; detail is redacted to a stable diagnostic id + category.
    SdkBuild {
        category: MatrixIpcErrorCategory,
        diagnostic_id: &'static str,
        /// Non-secret short message (never includes keys/tokens).
        message: String,
    },
}

impl fmt::Display for ClientBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Identity(e) => write!(f, "identity: {e}"),
            Self::StorePath(e) => write!(f, "store path: {e}"),
            Self::InvalidConfig(msg) => write!(f, "invalid client build config: {msg}"),
            Self::SdkBuild {
                diagnostic_id,
                message,
                ..
            } => write!(f, "sdk client build failed ({diagnostic_id}): {message}"),
        }
    }
}

impl std::error::Error for ClientBuilderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Identity(e) => Some(e),
            Self::StorePath(e) => Some(e),
            Self::InvalidConfig(_) | Self::SdkBuild { .. } => None,
        }
    }
}

impl From<AccountIdentityError> for ClientBuilderError {
    fn from(value: AccountIdentityError) -> Self {
        Self::Identity(value)
    }
}

impl From<StorePathError> for ClientBuilderError {
    fn from(value: StorePathError) -> Self {
        Self::StorePath(value)
    }
}

impl ClientBuilderError {
    pub fn to_factory_error(&self) -> FactoryError {
        match self {
            Self::Identity(_) => FactoryError {
                category: MatrixIpcErrorCategory::SdkInvariant,
                diagnostic_id: "p2.3-client-builder-identity",
            },
            Self::StorePath(_) => FactoryError {
                category: MatrixIpcErrorCategory::StoreUnavailable,
                diagnostic_id: "p2.3-client-builder-store-path",
            },
            Self::InvalidConfig(_) => FactoryError {
                category: MatrixIpcErrorCategory::SdkInvariant,
                diagnostic_id: "p2.3-client-builder-invalid-config",
            },
            Self::SdkBuild {
                category,
                diagnostic_id,
                ..
            } => FactoryError {
                category: *category,
                diagnostic_id,
            },
        }
    }
}
