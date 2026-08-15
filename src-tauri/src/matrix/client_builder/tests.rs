//! Unit + harness tests for P2.3 SDK client builder.

use super::*;
use crate::matrix::store::{AccountIdentity, StoreKeyMaterial, StorePaths};
use crate::matrix::supervisor::{
    ClientFactory, ClientHandle, MatrixSupervisor, SupervisorCommand, SupervisorState,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

fn temp_root(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "synara-p2.3-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp root");
    dir
}

fn alice() -> AccountIdentity {
    AccountIdentity::new("@alice:example.org", "https://example.org").unwrap()
}

fn bob() -> AccountIdentity {
    AccountIdentity::new("@bob:example.org", "https://example.org").unwrap()
}

/// Multi-thread runtime so SQLite/deadpool Client drops can see a Tokio handle.
fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime")
}

#[test]
fn marker_stable() {
    assert_eq!(
        matrix_client_builder_markers(),
        MATRIX_CLIENT_BUILDER_MARKER
    );
    assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"sqlite"));
    assert!(APPROVED_MATRIX_SDK_FEATURES.contains(&"bundled-sqlite"));
    assert!(FORBIDDEN_MATRIX_SDK_FEATURES.contains(&"experimental-widgets"));
}

#[test]
fn default_user_agent_includes_product_and_sdk_pin() {
    let ua = default_user_agent();
    assert!(ua.contains("Synara-Desktop/"), "{ua}");
    assert!(
        ua.contains(&format!("(matrix-sdk/{MATRIX_SDK_PIN_VERSION})")),
        "{ua}"
    );
}

#[test]
fn network_policy_rejects_disabled_ssl() {
    let policy = NetworkPolicy {
        proxy_url: None,
        ssl_verification: false,
    };
    assert!(policy.validate().is_err());
}

#[test]
fn network_policy_rejects_empty_or_bad_proxy() {
    assert!(NetworkPolicy {
        proxy_url: Some("".into()),
        ssl_verification: true,
    }
    .validate()
    .is_err());
    assert!(NetworkPolicy {
        proxy_url: Some("socks5://localhost:1080".into()),
        ssl_verification: true,
    }
    .validate()
    .is_err());
    assert!(NetworkPolicy {
        proxy_url: Some("http://127.0.0.1:8080".into()),
        ssl_verification: true,
    }
    .validate()
    .is_ok());
    // R0.6: credential-bearing proxy URLs are rejected.
    assert!(NetworkPolicy {
        proxy_url: Some("http://user:secret@127.0.0.1:8080".into()),
        ssl_verification: true,
    }
    .validate()
    .is_err());
}

#[test]
fn timeout_policy_bounds() {
    assert!(TimeoutPolicy {
        request_timeout: Duration::from_secs(0),
        retry_limit: 1,
    }
    .validate()
    .is_err());
    assert!(TimeoutPolicy {
        request_timeout: Duration::from_secs(601),
        retry_limit: 1,
    }
    .validate()
    .is_err());
    assert!(TimeoutPolicy::default().validate().is_ok());
}

#[test]
fn product_default_config_plan_has_no_secrets() {
    let root = temp_root("plan");
    let key = StoreKeyMaterial::generate().unwrap();
    let cfg = ClientBuildConfig::product_default(&root, alice(), Some(key)).unwrap();
    let plan = cfg.plan();
    let json = serde_json::to_string(&plan).unwrap();
    if let Some(pass) = cfg.store_passphrase_hex() {
        assert!(!json.contains(&pass));
    }
    // R0.6 / REV-003: no homeserver URL, user id, or absolute store paths.
    assert!(!json.contains("https://example.org"));
    assert!(!json.contains("@alice:example.org"));
    assert!(!json.contains(root.to_string_lossy().as_ref()));
    assert!(!json.contains("homeserverUrl"));
    assert!(plan.homeserver_configured);
    assert!(plan.store_key_present);
    assert!(plan.ssl_verification);
    assert!(!plan.proxy_configured);
    assert_eq!(plan.homeserver_mode, "explicit_url");
    assert_eq!(plan.matrix_sdk_version, MATRIX_SDK_PIN_VERSION);
    assert!(plan.approved_features.iter().any(|f| f == "sqlite"));
    assert!(plan.store_layout.confined_under_matrix_root);
    assert_eq!(plan.store_layout.relative_state_dir, "state");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn two_accounts_get_distinct_store_paths_in_config() {
    let root = temp_root("collision");
    let a = ClientBuildConfig::product_default(&root, alice(), None).unwrap();
    let b = ClientBuildConfig::product_default(&root, bob(), None).unwrap();
    assert_ne!(a.account_root(), b.account_root());
    assert_ne!(a.state_store_path(), b.state_store_path());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn empty_user_agent_rejected() {
    let root = temp_root("ua");
    let cfg = ClientBuildConfig::product_default(&root, alice(), None).unwrap();
    assert!(cfg.with_user_agent("   ").is_err());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn build_unauthenticated_client_offline_sqlite() {
    let root = temp_root("open");
    let key = StoreKeyMaterial::generate().unwrap();
    let cfg = ClientBuildConfig::product_default(&root, alice(), Some(key)).unwrap();

    let rt = test_runtime();
    let _enter = rt.enter();
    let client = rt
        .block_on(build_unauthenticated_client(&cfg))
        .expect("client open");
    let hs = client.homeserver().to_string();
    assert!(
        hs.contains("example.org"),
        "homeserver should reflect config url, got {hs}"
    );
    assert!(
        client.session().is_none(),
        "unauthenticated builder must not install a session"
    );
    assert!(cfg.state_store_path().is_dir());
    assert!(cfg.cache_store_path().is_dir());

    drop(client);
    drop(_enter);
    drop(rt);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn reopen_client_on_same_store_paths() {
    let root = temp_root("reopen");
    let key_bytes = *StoreKeyMaterial::generate().unwrap().as_bytes();
    let key1 = StoreKeyMaterial::from_bytes(key_bytes);
    let key2 = StoreKeyMaterial::from_bytes(key_bytes);
    let cfg1 = ClientBuildConfig::product_default(&root, alice(), Some(key1)).unwrap();
    let cfg2 = ClientBuildConfig::product_default(&root, alice(), Some(key2)).unwrap();

    let rt = test_runtime();
    let _enter = rt.enter();
    let c1 = rt
        .block_on(build_unauthenticated_client(&cfg1))
        .expect("first open");
    drop(c1);
    let c2 = rt
        .block_on(build_unauthenticated_client(&cfg2))
        .expect("reopen");
    assert!(c2.session().is_none());
    drop(c2);
    drop(_enter);
    drop(rt);

    let _ = fs::remove_dir_all(&root);
}

/// Harness factory: builds a real SDK client on a shared multi-thread runtime.
struct HarnessSdkFactory {
    root: PathBuf,
    identity: AccountIdentity,
    next_id: AtomicU64,
    runtime: tokio::runtime::Runtime,
}

impl ClientFactory for HarnessSdkFactory {
    fn build(
        &self,
        _generation: u64,
    ) -> Result<Box<dyn ClientHandle>, crate::matrix::supervisor::FactoryError> {
        let _enter = self.runtime.enter();
        let key =
            StoreKeyMaterial::generate().map_err(|_| crate::matrix::supervisor::FactoryError {
                category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
                diagnostic_id: "p2.3-harness-keygen",
            })?;
        let cfg = ClientBuildConfig::product_default(&self.root, self.identity.clone(), Some(key))
            .map_err(|e| e.to_factory_error())?;
        let client = self
            .runtime
            .block_on(build_unauthenticated_client(&cfg))
            .map_err(|e| e.to_factory_error())?;
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        Ok(Box::new(SdkClientHandle::new(id, client)))
    }
}

#[test]
fn supervisor_installs_sdk_handle_from_builder_factory() {
    let root = temp_root("supervisor");
    let factory = HarnessSdkFactory {
        root: root.clone(),
        identity: alice(),
        next_id: AtomicU64::new(0),
        runtime: test_runtime(),
    };
    // Keep a handle entered for the whole supervisor lifecycle (Client drop on logout).
    let _enter = factory.runtime.enter();

    let mut supervisor = MatrixSupervisor::new();
    supervisor
        .apply(SupervisorCommand::BeginOpen)
        .expect("begin open");
    supervisor
        .apply(SupervisorCommand::BeginAuthenticate)
        .expect("auth");
    supervisor
        .apply_with_factory(SupervisorCommand::InstallClient, &factory)
        .expect("install sdk client");
    assert!(supervisor.has_client());
    supervisor
        .apply(SupervisorCommand::BeginSync)
        .expect("sync");
    supervisor
        .apply(SupervisorCommand::MarkReady)
        .expect("ready");
    assert_eq!(supervisor.state(), SupervisorState::Ready);

    supervisor
        .apply(SupervisorCommand::BeginStop)
        .expect("stop");
    supervisor
        .apply(SupervisorCommand::CompleteLogout)
        .expect("logout");
    assert!(!supervisor.has_client());
    assert_eq!(supervisor.state(), SupervisorState::LoggedOut);

    drop(supervisor);
    drop(_enter);
    drop(factory);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn store_paths_module_still_derives_under_same_root() {
    let root = temp_root("paths-align");
    let id = alice();
    let paths = StorePaths::derive(&root, &id).unwrap();
    let cfg = ClientBuildConfig::product_default(&root, id, None).unwrap();
    assert_eq!(paths.state_dir(), cfg.state_store_path());
    assert_eq!(paths.cache_dir(), cfg.cache_store_path());
    let _ = fs::remove_dir_all(&root);
}
