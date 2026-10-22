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

/// SDK default holder used by the containing product process.
const DEFAULT_STORE_LOCK_HOLDER: &str = "main";

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

/// Fallback user agent for a config whose shell did not name itself.
///
/// It carries Core's crate version, not an app version. Product shells pass
/// [`product_user_agent`] through [`ClientBuildConfig::with_user_agent`].
pub fn default_user_agent() -> String {
    format!(
        "Synara-Core/{} (matrix-sdk/{})",
        env!("CARGO_PKG_VERSION"),
        super::MATRIX_SDK_PIN_VERSION
    )
}

/// The shell process a client belongs to, as named in its User-Agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientProduct {
    Desktop,
    Ios,
    /// The iOS notification service extension.
    IosNotificationExtension,
}

impl ClientProduct {
    fn user_agent_name(self) -> &'static str {
        match self {
            Self::Desktop => "Synara-Desktop",
            Self::Ios => "Synara-iOS",
            Self::IosNotificationExtension => "Synara-iOS-NSE",
        }
    }
}

/// Longest app version kept in a User-Agent.
const MAX_APP_VERSION_CHARS: usize = 32;

/// `Synara-<platform>/<app version> (matrix-sdk/<pin>)`: what the homeserver
/// records for each device. `app_version` comes from the shell (bundle short
/// version, desktop package version). Characters outside `[0-9A-Za-z.+-]`
/// are dropped so a malformed value cannot inject header syntax; an empty
/// result reads `unknown`.
pub fn product_user_agent(product: ClientProduct, app_version: &str) -> String {
    let version: String = app_version
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '-'))
        .take(MAX_APP_VERSION_CHARS)
        .collect();
    let version = if version.is_empty() {
        "unknown"
    } else {
        version.as_str()
    };
    format!(
        "{}/{version} (matrix-sdk/{})",
        product.user_agent_name(),
        super::MATRIX_SDK_PIN_VERSION
    )
}

/// How much memory a client's SQLite stores may hold.
///
/// Each of the four databases (state, crypto, event cache, media) has its own
/// connection pool, and every connection keeps its own page cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreFootprint {
    /// SDK defaults: up to physical CPUs × 4 connections per database, a
    /// 2 MB page cache per connection and a 10 MB WAL limit. Desktop.
    Standard,
    /// The SDK's low-memory profile: one connection per physical CPU, a
    /// 500 KB page cache and a 2 MB WAL limit. The iOS app, which iOS
    /// terminates in the background when it holds too much memory.
    LowMemory,
    /// The low-memory profile with the SDK's minimum pool of two connections
    /// (one writer, one reader). The notification extension resolves one
    /// event under a memory ceiling of about 24 MB.
    Extension,
}

/// SDK minimum: one connection for writes and one for reads.
const EXTENSION_POOL_MAX_SIZE: usize = 2;

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
    /// Stable process identity for the SDK's leased cross-process store locks.
    /// Every process that can open the same store must use a distinct value.
    cross_process_store_lock_holder: String,
    /// SQLite store passphrase bytes (32). Prefer `StoreKeyMaterial` from P2.2 vault.
    store_key: Option<StoreKeyMaterial>,
    /// Persist and query the desktop local message index. Default on.
    /// Off skips `EncryptedDirectory` and product queries (no CS `/search` fallback).
    indexed_message_search: bool,
    /// SQLite pool and page-cache sizing. Default [`StoreFootprint::Standard`].
    store_footprint: StoreFootprint,
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
            cross_process_store_lock_holder: DEFAULT_STORE_LOCK_HOLDER.to_owned(),
            store_key,
            indexed_message_search: true,
            store_footprint: StoreFootprint::Standard,
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

    /// Enable or disable the local encrypted message index at client open.
    ///
    /// Changing this after a client is built has no effect; the store kind is
    /// ClientBuilder-only. Product UI must reload the session.
    pub fn with_indexed_message_search(mut self, enabled: bool) -> Self {
        self.indexed_message_search = enabled;
        self
    }

    pub fn indexed_message_search(&self) -> bool {
        self.indexed_message_search
    }

    /// Size the SQLite pools and page caches for the hosting process.
    pub fn with_store_footprint(mut self, footprint: StoreFootprint) -> Self {
        self.store_footprint = footprint;
        self
    }

    pub fn store_footprint(&self) -> StoreFootprint {
        self.store_footprint
    }

    /// SQLite configuration for the state/crypto path. The SDK opens the
    /// event-cache and media databases with the same pool and cache sizes.
    pub(crate) fn sqlite_store_config(
        &self,
        passphrase: Option<&str>,
    ) -> matrix_sdk::SqliteStoreConfig {
        let path = self.state_store_path();
        let config = match self.store_footprint {
            StoreFootprint::Standard => matrix_sdk::SqliteStoreConfig::new(path),
            StoreFootprint::LowMemory => {
                matrix_sdk::SqliteStoreConfig::with_low_memory_config(path)
            }
            StoreFootprint::Extension => {
                matrix_sdk::SqliteStoreConfig::with_low_memory_config(path)
                    .pool_max_size(EXTENSION_POOL_MAX_SIZE)
            }
        };
        config.passphrase(passphrase)
    }

    pub fn with_cross_process_store_lock_holder(
        mut self,
        holder: impl Into<String>,
    ) -> Result<Self, ClientBuilderError> {
        let holder = holder.into();
        if holder.trim().is_empty() || holder.len() > 64 {
            return Err(ClientBuilderError::InvalidConfig(
                "cross-process store lock holder must be 1 to 64 characters",
            ));
        }
        self.cross_process_store_lock_holder = holder;
        Ok(self)
    }

    pub(crate) fn cross_process_store_lock_holder(&self) -> &str {
        &self.cross_process_store_lock_holder
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
            indexed_message_search: self.indexed_message_search,
            store_layout: self.store_paths.layout(),
            approved_features: super::APPROVED_MATRIX_SDK_FEATURES
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            matrix_sdk_version: super::MATRIX_SDK_PIN_VERSION.to_owned(),
            x509_verifier_configured: crate::app::x509::should_inject_verifier(
                self.store_paths.account_root(),
            ),
        }
    }

    /// Absolute state-store directory (SQLite primary path).
    pub fn state_store_path(&self) -> &Path {
        self.store_paths.state_dir()
    }

    /// Absolute event-cache directory.
    pub fn cache_store_path(&self) -> &Path {
        self.store_paths.cache_dir()
    }

    /// Absolute encrypted local-index directory (`search/` under the account root).
    pub fn search_store_path(&self) -> &Path {
        self.store_paths.search_dir()
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
    pub indexed_message_search: bool,
    pub store_layout: StoreLayout,
    pub approved_features: Vec<String>,
    pub matrix_sdk_version: String,
    /// Whether this build would inject an X.509 verifier. Never PEM / CMS.
    #[serde(default)]
    pub x509_verifier_configured: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ClientBuildConfig {
        let identity = AccountIdentity::new("@alice:example.org", "https://example.org")
            .expect("valid test identity");
        ClientBuildConfig::product_default(
            Path::new("/tmp/synara-client-config-test"),
            identity,
            None,
        )
        .expect("valid product config")
    }

    #[test]
    fn store_lock_holder_defaults_to_main_and_accepts_a_distinct_process() {
        let config = make_config();
        assert_eq!(config.cross_process_store_lock_holder(), "main");

        let config = make_config()
            .with_cross_process_store_lock_holder("synara-nse-parent")
            .expect("valid distinct holder");
        assert_eq!(
            config.cross_process_store_lock_holder(),
            "synara-nse-parent"
        );
    }

    #[test]
    fn store_lock_holder_rejects_empty_and_oversized_values() {
        assert!(make_config()
            .with_cross_process_store_lock_holder("   ")
            .is_err());
        assert!(make_config()
            .with_cross_process_store_lock_holder("x".repeat(65))
            .is_err());
    }

    #[test]
    fn indexed_message_search_defaults_on_and_plan_has_no_secrets() {
        let config = make_config();
        assert!(config.indexed_message_search());
        let off = config.with_indexed_message_search(false);
        assert!(!off.indexed_message_search());
        let plan = off.plan();
        let json = serde_json::to_string(&plan).expect("plan serializes");
        assert!(!plan.indexed_message_search);
        assert!(!json.contains("passphrase"));
        assert!(!json.contains("password"));
        assert!(!json.contains("/tmp/"));
        assert!(!json.contains("search/"));
        assert!(plan
            .approved_features
            .iter()
            .any(|feature| feature == "experimental-search"));
        assert_eq!(plan.store_layout.relative_search_dir, "search");
    }

    #[test]
    fn product_user_agent_names_the_shell_and_its_app_version() {
        let pin = super::super::MATRIX_SDK_PIN_VERSION;
        assert_eq!(
            product_user_agent(ClientProduct::Ios, "2.2.6"),
            format!("Synara-iOS/2.2.6 (matrix-sdk/{pin})")
        );
        assert_eq!(
            product_user_agent(ClientProduct::Desktop, "2.2.6"),
            format!("Synara-Desktop/2.2.6 (matrix-sdk/{pin})")
        );
        assert_eq!(
            product_user_agent(ClientProduct::IosNotificationExtension, " 2.3.0-rc.1 "),
            format!("Synara-iOS-NSE/2.3.0-rc.1 (matrix-sdk/{pin})")
        );
        // Header syntax and whitespace never reach the request.
        assert_eq!(
            product_user_agent(ClientProduct::Ios, "2.2\r\nX-Evil: 1"),
            format!("Synara-iOS/2.2X-Evil1 (matrix-sdk/{pin})")
        );
        assert_eq!(
            product_user_agent(ClientProduct::Ios, " (/) "),
            format!("Synara-iOS/unknown (matrix-sdk/{pin})")
        );
        assert!(product_user_agent(ClientProduct::Ios, &"9".repeat(500)).len() < 80);
        // The fallback no longer claims to be the desktop app.
        assert!(default_user_agent().starts_with("Synara-Core/"));
        assert_eq!(make_config().user_agent, default_user_agent());
    }

    #[test]
    fn store_footprint_sizes_every_sqlite_pool_and_cache() {
        let config = make_config();
        assert_eq!(config.store_footprint(), StoreFootprint::Standard);
        let standard = format!("{:?}", config.sqlite_store_config(None));
        assert!(standard.contains("cache_size: 2000000"), "{standard}");

        let low = format!(
            "{:?}",
            make_config()
                .with_store_footprint(StoreFootprint::LowMemory)
                .sqlite_store_config(None)
        );
        assert!(low.contains("cache_size: 500000"), "{low}");
        assert!(low.contains("journal_size_limit: 2000000"), "{low}");

        let extension = make_config().with_store_footprint(StoreFootprint::Extension);
        let rendered = format!("{:?}", extension.sqlite_store_config(Some("00")));
        assert!(rendered.contains("max_size: 2,"), "{rendered}");
        assert!(rendered.contains("cache_size: 500000"), "{rendered}");
        assert!(
            rendered.contains("journal_size_limit: 2000000"),
            "{rendered}"
        );
        // The passphrase never appears in the debug output.
        assert!(!rendered.contains("\"00\""), "{rendered}");
    }

    #[test]
    fn plan_x509_flag_defaults_false_and_omits_pem() {
        let config = make_config();
        let plan = config.plan();
        assert!(!plan.x509_verifier_configured);
        let json = serde_json::to_string(&plan).expect("plan json");
        assert!(json.contains("x509VerifierConfigured"));
        assert!(!json.to_ascii_lowercase().contains("begin certificate"));
        assert!(!json.to_ascii_lowercase().contains("private key"));
        assert!(!json.contains("io.element.x509"));
    }
}
