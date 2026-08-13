//! Pure client-build configuration and policies (no network I/O).

use std::path::Path;
use std::time::Duration;

use super::ClientBuilderError;
use crate::app::store::{
    AccountIdentity, StoreKeyMaterial, StoreLayout, StorePaths, STORE_KEY_LEN,
};

/// Default HTTP request timeout (seconds) for product clients.
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;

/// Default retry limit for product HTTP requests.
pub const DEFAULT_RETRY_LIMIT: usize = 3;

/// How the homeserver is supplied to the SDK builder.
///
/// Product default is explicit HTTPS homeserver URL (no well-known discovery
/// at build time). Discovery-based modes are available for later auth flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeserverMode {
    /// `ClientBuilder::homeserver_url` — no network at build (preferred default).
    ExplicitUrl,
}

/// Proxy / TLS network policy for the Matrix HTTP stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkPolicy {
    /// Optional HTTP proxy URL (`http://host:port`). HTTPS proxies unsupported by SDK note.
    pub proxy_url: Option<String>,
    /// Product default: `true`. Disabling TLS verification is forbidden for product configs.
    pub ssl_verification: bool,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            proxy_url: None,
            ssl_verification: true,
        }
    }
}

impl NetworkPolicy {
    pub fn validate(&self) -> Result<(), ClientBuilderError> {
        if !self.ssl_verification {
            return Err(ClientBuilderError::InvalidConfig(
                "ssl verification must remain enabled for product clients",
            ));
        }
        if let Some(proxy) = &self.proxy_url {
            let p = proxy.trim();
            if p.is_empty() {
                return Err(ClientBuilderError::InvalidConfig("proxy url is empty"));
            }
            if !(p.starts_with("http://") || p.starts_with("https://")) {
                return Err(ClientBuilderError::InvalidConfig(
                    "proxy url must use http:// or https:// scheme",
                ));
            }
            // R0.6 / REV-003: reject credential-bearing proxy URLs.
            if crate::app::diagnostics::looks_like_url_with_credentials(p) {
                return Err(ClientBuilderError::InvalidConfig(
                    "proxy url must not embed credentials",
                ));
            }
        }
        Ok(())
    }
}

/// Request timeout / retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeoutPolicy {
    pub request_timeout: Duration,
    pub retry_limit: usize,
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
            retry_limit: DEFAULT_RETRY_LIMIT,
        }
    }
}

impl TimeoutPolicy {
    pub fn validate(&self) -> Result<(), ClientBuilderError> {
        if self.request_timeout.is_zero() {
            return Err(ClientBuilderError::InvalidConfig(
                "request timeout must be non-zero",
            ));
        }
        if self.request_timeout > Duration::from_secs(600) {
            return Err(ClientBuilderError::InvalidConfig(
                "request timeout exceeds 600s upper bound",
            ));
        }
        Ok(())
    }
}

/// Product user agent string for Matrix HTTP requests.
pub fn default_user_agent() -> String {
    format!(
        "Synara-Desktop/{} (matrix-sdk/{})",
        env!("CARGO_PKG_VERSION"),
        super::MATRIX_SDK_PIN_VERSION
    )
}

/// Full configuration required to open an unauthenticated SDK client.
///
/// Does **not** hold access/refresh tokens. Store key material is optional at
/// config time; when present it is used only as the SQLite store passphrase
/// (never logged).
#[derive(Debug)]
pub struct ClientBuildConfig {
    pub identity: AccountIdentity,
    pub store_paths: StorePaths,
    pub homeserver_mode: HomeserverMode,
    pub network: NetworkPolicy,
    pub timeouts: TimeoutPolicy,
    pub user_agent: String,
    /// When true, enable SDK refresh-token handling hooks (no tokens stored here).
    pub handle_refresh_tokens: bool,
    /// SQLite store passphrase bytes (32). Prefer `StoreKeyMaterial` from P2.2 vault.
    store_key: Option<StoreKeyMaterial>,
}

impl ClientBuildConfig {
    /// Build a product-default config for `identity` under `app_data_root`.
    ///
    /// Does not create directories; call [`StorePaths::ensure_dirs`] before open.
    pub fn product_default(
        app_data_root: &Path,
        identity: AccountIdentity,
        store_key: Option<StoreKeyMaterial>,
    ) -> Result<Self, ClientBuilderError> {
        let store_paths = StorePaths::derive(app_data_root, &identity)?;
        let cfg = Self {
            identity,
            store_paths,
            homeserver_mode: HomeserverMode::ExplicitUrl,
            network: NetworkPolicy::default(),
            timeouts: TimeoutPolicy::default(),
            user_agent: default_user_agent(),
            handle_refresh_tokens: true,
            store_key,
        };
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn with_proxy(mut self, proxy_url: impl Into<String>) -> Result<Self, ClientBuilderError> {
        self.network.proxy_url = Some(proxy_url.into());
        self.network.validate()?;
        Ok(self)
    }

    pub fn with_user_agent(
        mut self,
        user_agent: impl Into<String>,
    ) -> Result<Self, ClientBuilderError> {
        self.user_agent = user_agent.into();
        self.validate_user_agent()?;
        Ok(self)
    }

    pub fn with_timeouts(mut self, timeouts: TimeoutPolicy) -> Result<Self, ClientBuilderError> {
        timeouts.validate()?;
        self.timeouts = timeouts;
        Ok(self)
    }

    pub fn with_store_key(mut self, key: StoreKeyMaterial) -> Self {
        self.store_key = Some(key);
        self
    }

    pub fn store_key(&self) -> Option<&StoreKeyMaterial> {
        self.store_key.as_ref()
    }

    pub fn validate(&self) -> Result<(), ClientBuilderError> {
        self.network.validate()?;
        self.timeouts.validate()?;
        self.validate_user_agent()?;
        if self.identity.homeserver_url().is_empty() {
            return Err(ClientBuilderError::InvalidConfig("homeserver url empty"));
        }
        Ok(())
    }

    fn validate_user_agent(&self) -> Result<(), ClientBuilderError> {
        let ua = self.user_agent.trim();
        if ua.is_empty() {
            return Err(ClientBuilderError::InvalidConfig("user agent is empty"));
        }
        if ua.len() > 256 {
            return Err(ClientBuilderError::InvalidConfig(
                "user agent exceeds 256 characters",
            ));
        }
        Ok(())
    }

    /// Hex encoding of store key for SQLite passphrase (64 hex chars for 32 bytes).
    ///
    /// Never log the returned string.
    pub fn store_passphrase_hex(&self) -> Option<String> {
        self.store_key.as_ref().map(|k| {
            debug_assert_eq!(k.as_bytes().len(), STORE_KEY_LEN);
            let mut out = String::with_capacity(STORE_KEY_LEN * 2);
            for b in k.as_bytes() {
                out.push_str(&format!("{b:02x}"));
            }
            out
        })
    }

    /// Privacy-safe plan projection for diagnostics (R0.6 / REV-003).
    ///
    /// Never includes homeserver/proxy URLs, absolute paths, user IDs, tokens,
    /// or store key material.
    pub fn plan(&self) -> ClientBuildPlan {
        ClientBuildPlan {
            homeserver_configured: !self.identity.homeserver_url().is_empty(),
            homeserver_mode: match self.homeserver_mode {
                HomeserverMode::ExplicitUrl => "explicit_url",
            }
            .to_owned(),
            user_agent: self.user_agent.clone(),
            proxy_configured: self.network.proxy_url.is_some(),
            ssl_verification: self.network.ssl_verification,
            request_timeout_secs: self.timeouts.request_timeout.as_secs(),
            retry_limit: self.timeouts.retry_limit,
            handle_refresh_tokens: self.handle_refresh_tokens,
            store_key_present: self.store_key.is_some(),
            store_layout: self.store_paths.layout(),
            approved_features: super::APPROVED_MATRIX_SDK_FEATURES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            matrix_sdk_version: super::MATRIX_SDK_PIN_VERSION.to_owned(),
        }
    }

    /// Absolute state-store directory (SQLite primary path).
    pub fn state_store_path(&self) -> &Path {
        self.store_paths.state_dir()
    }

    /// Absolute cache directory (event-cache separation).
    pub fn cache_store_path(&self) -> &Path {
        self.store_paths.cache_dir()
    }

    pub fn account_root(&self) -> &Path {
        self.store_paths.account_root()
    }

    pub fn ensure_store_dirs(&self) -> Result<(), ClientBuilderError> {
        self.store_paths.ensure_dirs()?;
        Ok(())
    }
}

/// Serializable diagnostic plan (no secrets, URLs, or absolute paths).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientBuildPlan {
    /// Whether a homeserver endpoint is configured (never the URL itself).
    pub homeserver_configured: bool,
    pub homeserver_mode: String,
    pub user_agent: String,
    pub proxy_configured: bool,
    pub ssl_verification: bool,
    pub request_timeout_secs: u64,
    pub retry_limit: usize,
    pub handle_refresh_tokens: bool,
    pub store_key_present: bool,
    pub store_layout: StoreLayout,
    pub approved_features: Vec<String>,
    pub matrix_sdk_version: String,
}
