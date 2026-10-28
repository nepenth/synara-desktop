//! Password sign-in made before the account's store is known.
//!
//! A homeserver reached through a proxy whose host is not its server name
//! names the account only in its `/login` answer. When no local store from an
//! earlier sign-in matches what was typed, the shells sign in here on a
//! memory-only client, open the store for the returned `user_id`, and
//! restore the session onto it with [`DetachedSignIn::restore_onto`].
//!
//! Until that restore takes, the server holds a session no client owns.
//! Every way out that does not install it revokes it in the background:
//! [`DetachedSignIn::revoke`], a failed restore, and dropping the value, which
//! is what happens when the caller's future is cancelled (a Swift `Task`
//! cancelled mid sign-in). The revoke retries a few times and records a
//! journal line if the server never confirms.
//!
//! The account each typed username resolved to is remembered per homeserver
//! URL ([`remember_login_account`]), so the next sign-in opens that account's
//! store before signing in and reuses its device instead of creating one.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use matrix_sdk::Client;
use serde::{Deserialize, Serialize};

use super::error::AuthError;
use super::login::{
    login_with_password_detached, require_native_sliding_sync, LoginMethodKind, LoginOptions,
    LoginResult, LoginUser,
};
use super::{has_account_store, RegisterCompleteSecrets};
use crate::app::client_builder::{build_memory_only_client, ClientBuildConfig};
use crate::app::lifecycle::session::{
    bounded_remote_logout, record_detached_sign_in_revoke_failed, VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
};
use crate::app::lifecycle::{restore_session_onto_client, SessionMaterial};
use crate::app::store::{AccountIdentity, MATRIX_STORE_ROOT_SEGMENT};

/// Pauses before each revoke retry; the first attempt runs at once.
const REVOKE_RETRY_DELAYS: [Duration; 2] = [Duration::from_millis(500), Duration::from_secs(2)];

/// A signed-in session whose account store is not open yet.
pub struct DetachedSignIn {
    identity: AccountIdentity,
    pending: Option<PendingSession>,
    runtime: tokio::runtime::Handle,
}

/// What a revoke needs: the session's tokens and the memory-only client
/// that signed in.
struct PendingSession {
    secrets: RegisterCompleteSecrets,
    client: Client,
    journal_root: PathBuf,
}

impl DetachedSignIn {
    /// Sign in on a memory-only client built from `config`, whose identity
    /// is a synthetic one for the homeserver URL signed in through.
    ///
    /// Refuses a server without native sliding sync before any device
    /// exists. `journal_root` receives a line if a later revoke fails.
    pub async fn sign_in(
        config: ClientBuildConfig,
        user: &LoginUser,
        password: &str,
        options: &LoginOptions,
        journal_root: &Path,
    ) -> Result<Self, AuthError> {
        let client =
            build_memory_only_client(&config)
                .await
                .map_err(|_| AuthError::SdkInvariant {
                    diagnostic_id: "p3.2-login-client-build-failed",
                })?;
        require_native_sliding_sync(&client).await?;
        let secrets = login_with_password_detached(&client, user, password, options).await?;
        let identity = AccountIdentity::new(&secrets.user_id, config.identity.homeserver_url());
        let signed_in = Self {
            identity: identity.clone().unwrap_or_else(|_| config.identity.clone()),
            pending: Some(PendingSession {
                secrets,
                client,
                journal_root: journal_root.to_path_buf(),
            }),
            runtime: tokio::runtime::Handle::current(),
        };
        match identity {
            Ok(_) => Ok(signed_in),
            Err(_) => {
                signed_in.revoke();
                Err(AuthError::SdkInvariant {
                    diagnostic_id: "p3.2-login-identity-invalid",
                })
            }
        }
    }

    /// The account the server named.
    pub fn identity(&self) -> &AccountIdentity {
        &self.identity
    }

    /// Log the session out in the background, retrying; it will not be
    /// installed.
    pub fn revoke(mut self) {
        self.spawn_revoke();
    }

    /// Apply the session to `client`, the account store's unauthenticated
    /// client for [`Self::identity`]. If it does not take, the session is
    /// revoked here; once it has, the caller's login rollback owns it.
    pub async fn restore_onto(
        mut self,
        client: &Client,
        device_display_name: &str,
    ) -> Result<LoginResult, AuthError> {
        let Some(pending) = self.pending.as_ref() else {
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.2-login-restore-failed",
            });
        };
        if !restore_secrets(client, &self.identity, &pending.secrets).await {
            if client.session().is_none() {
                self.spawn_revoke();
            } else {
                self.pending = None;
            }
            return Err(AuthError::SdkInvariant {
                diagnostic_id: "p3.2-login-restore-failed",
            });
        }
        let pending = self.pending.take().expect("checked above");
        Ok(LoginResult {
            user_id: pending.secrets.user_id.clone(),
            device_id: pending.secrets.device_id.clone(),
            homeserver_url: client.homeserver().to_string(),
            device_display_name: device_display_name.to_owned(),
            method: LoginMethodKind::Password,
        })
    }

    fn spawn_revoke(&mut self) {
        if let Some(pending) = self.pending.take() {
            let identity = self.identity.clone();
            self.runtime.spawn(revoke_pending(identity, pending));
        }
    }
}

impl Drop for DetachedSignIn {
    /// A dropped sign-in (the caller's future was cancelled, or an error path
    /// returned early) never leaves its session logged in.
    fn drop(&mut self) {
        self.spawn_revoke();
    }
}

async fn restore_secrets(
    client: &Client,
    identity: &AccountIdentity,
    secrets: &RegisterCompleteSecrets,
) -> bool {
    // Session install must go through lifecycle (guardrail: no Client::restore_session under matrix/auth/).
    match SessionMaterial::from_matrix_tokens(
        identity,
        secrets.device_id.as_str(),
        secrets.access_token.as_str(),
        secrets.refresh_token.as_ref().map(|token| token.as_str()),
    ) {
        Ok(material) => restore_session_onto_client(client, identity, &material)
            .await
            .is_ok(),
        Err(_) => false,
    }
}

async fn revoke_pending(identity: AccountIdentity, pending: PendingSession) {
    let PendingSession {
        secrets,
        client,
        journal_root,
    } = pending;
    if restore_secrets(&client, &identity, &secrets).await {
        if bounded_remote_logout(
            VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
            client.matrix_auth().logout(),
        )
        .await
        {
            return;
        }
        for delay in REVOKE_RETRY_DELAYS {
            tokio::time::sleep(delay).await;
            if bounded_remote_logout(
                VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                client.matrix_auth().logout(),
            )
            .await
            {
                return;
            }
        }
    }
    // The device stays signed in on the server; the user can remove it from
    // another session's device list. The journal keeps a static record.
    record_detached_sign_in_revoke_failed(&journal_root);
}

/// File under the matrix store root mapping (homeserver URL, typed username)
/// to the account it signed in to. Holds only Matrix IDs and URLs.
const LOGIN_ACCOUNTS_FILE: &str = "login-accounts.json";
/// Entries kept, most recent first.
const MAX_LOGIN_ACCOUNTS: usize = 32;

#[derive(Debug, Default, Serialize, Deserialize)]
struct LoginAccounts {
    accounts: Vec<LoginAccount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct LoginAccount {
    homeserver_url: String,
    user: String,
    user_id: String,
}

fn login_accounts_path(root: &Path) -> PathBuf {
    root.join(MATRIX_STORE_ROOT_SEGMENT)
        .join(LOGIN_ACCOUNTS_FILE)
}

fn login_account_key(homeserver_url: &str, user: &LoginUser) -> (String, String) {
    (
        homeserver_url.trim().trim_end_matches('/').to_owned(),
        user.identifier().to_lowercase(),
    )
}

fn read_login_accounts(root: &Path) -> LoginAccounts {
    fs::read(login_accounts_path(root))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_login_accounts(root: &Path, accounts: &LoginAccounts) {
    let path = login_accounts_path(root);
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(accounts) else {
        return;
    };
    let staged = path.with_extension("json.tmp");
    if fs::write(&staged, bytes).is_ok() {
        let _ = fs::rename(&staged, &path);
    }
}

/// Remember that `user`, typed for `identity`'s homeserver URL, signed in to
/// `identity`. Best effort: a sign-in never fails because this cannot be
/// written.
pub fn remember_login_account(root: &Path, user: &LoginUser, identity: &AccountIdentity) {
    let (homeserver_url, user) = login_account_key(identity.homeserver_url(), user);
    let entry = LoginAccount {
        homeserver_url,
        user,
        user_id: identity.user_id().to_owned(),
    };
    let mut accounts = read_login_accounts(root);
    accounts.accounts.retain(|known| {
        (&known.homeserver_url, &known.user) != (&entry.homeserver_url, &entry.user)
    });
    accounts.accounts.insert(0, entry);
    accounts.accounts.truncate(MAX_LOGIN_ACCOUNTS);
    write_login_accounts(root, &accounts);
}

/// Forget what `user` resolved to on `homeserver_url`, after the server named
/// a different account for it.
pub fn forget_login_account(root: &Path, homeserver_url: &str, user: &LoginUser) {
    let key = login_account_key(homeserver_url, user);
    let mut accounts = read_login_accounts(root);
    let before = accounts.accounts.len();
    accounts
        .accounts
        .retain(|known| (&known.homeserver_url, &known.user) != (&key.0, &key.1));
    if accounts.accounts.len() != before {
        write_login_accounts(root, &accounts);
    }
}

/// The account whose local store a sign-in with `user` should open before
/// signing in, if one exists: the account this username signed in to last
/// time on `homeserver_url`, else the guess from `server_name`
/// ([`LoginUser::store_lookup_user_id`]).
///
/// Only an existing store counts. The server's answer is still checked
/// against it, so a stale entry cannot sign in to the wrong account.
pub fn login_store_identity(
    root: &Path,
    homeserver_url: &str,
    user: &LoginUser,
    server_name: Option<&str>,
) -> Option<AccountIdentity> {
    let (url, typed) = login_account_key(homeserver_url, user);
    let remembered = read_login_accounts(root)
        .accounts
        .into_iter()
        .find(|known| known.homeserver_url == url && known.user == typed)
        .map(|known| known.user_id);
    remembered
        .into_iter()
        .chain(user.store_lookup_user_id(server_name))
        .filter_map(|user_id| AccountIdentity::new(&user_id, homeserver_url).ok())
        .find(|identity| has_account_store(root, identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("synara-login-accounts-{tag}-{nanos}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn create_store(root: &Path, identity: &AccountIdentity) {
        crate::app::store::StorePaths::derive(root, identity)
            .unwrap()
            .ensure_dirs()
            .unwrap();
    }

    /// `matrix-v4.example.org` proxies a server named `matrix.example.org`:
    /// the guess from the chosen server never has a store, the remembered
    /// account does.
    #[test]
    fn a_remembered_account_finds_the_store_the_proxy_guess_misses() {
        let root = temp_root("remembered");
        let url = "https://matrix-v4.example.org";
        let user = LoginUser::parse("Pete").unwrap();
        let real = AccountIdentity::new("@pete:matrix.example.org", url).unwrap();
        create_store(&root, &real);
        assert_eq!(
            login_store_identity(&root, url, &user, Some("matrix-v4.example.org")),
            None
        );

        remember_login_account(&root, &user, &real);
        assert_eq!(
            login_store_identity(
                &root,
                &format!("{url}/"),
                &user,
                Some("matrix-v4.example.org")
            ),
            Some(real.clone())
        );
        // The same username in another case, or on another URL, is its own entry.
        let lower = LoginUser::parse("pete").unwrap();
        assert_eq!(
            login_store_identity(&root, url, &lower, None),
            Some(real.clone())
        );
        assert_eq!(
            login_store_identity(&root, "https://other.example.org", &user, None),
            None
        );

        forget_login_account(&root, url, &user);
        assert_eq!(login_store_identity(&root, url, &user, None), None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_remembered_account_without_a_store_falls_back_to_the_guess() {
        let root = temp_root("fallback");
        let url = "https://matrix.example.org";
        let user = LoginUser::parse("alice").unwrap();
        let gone = AccountIdentity::new("@alice:gone.example.org", url).unwrap();
        let guessed = AccountIdentity::new("@alice:matrix.example.org", url).unwrap();
        remember_login_account(&root, &user, &gone);
        create_store(&root, &guessed);
        assert_eq!(
            login_store_identity(&root, url, &user, Some("matrix.example.org")),
            Some(guessed)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remembered_accounts_are_capped_and_hold_no_secrets() {
        let root = temp_root("cap");
        let url = "https://matrix.example.org";
        for index in 0..(MAX_LOGIN_ACCOUNTS + 5) {
            let user = LoginUser::parse(&format!("user{index}")).unwrap();
            let identity = AccountIdentity::new(&format!("@user{index}:example.org"), url).unwrap();
            remember_login_account(&root, &user, &identity);
        }
        let accounts = read_login_accounts(&root).accounts;
        assert_eq!(accounts.len(), MAX_LOGIN_ACCOUNTS);
        assert_eq!(accounts[0].user, format!("user{}", MAX_LOGIN_ACCOUNTS + 4));
        let text = fs::read_to_string(login_accounts_path(&root)).unwrap();
        assert!(!text.contains("token") && !text.contains("password"));
        let _ = fs::remove_dir_all(root);
    }

    async fn mock_server(logout_status: u16) -> matrix_sdk::test_utils::mocks::MatrixMockServer {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, ResponseTemplate};
        let server = matrix_sdk::test_utils::mocks::MatrixMockServer::new().await;
        server
            .mock_versions()
            .with_simplified_sliding_sync()
            .ok()
            .mount()
            .await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "user_id": "@pete:matrix.example.org",
                "access_token": "syt_detached",
                "device_id": "PETEDETACHED",
            })))
            .mount(server.server())
            .await;
        Mock::given(method("POST"))
            .and(path("/_matrix/client/v3/logout"))
            .respond_with(ResponseTemplate::new(logout_status).set_body_json(serde_json::json!({})))
            .mount(server.server())
            .await;
        server
    }

    fn detached_config(root: &Path, homeserver_url: &str) -> ClientBuildConfig {
        let synthetic = AccountIdentity::new(
            &super::super::login_ephemeral_user_id(homeserver_url).unwrap(),
            homeserver_url,
        )
        .unwrap();
        ClientBuildConfig::product_default(root, synthetic, None).unwrap()
    }

    async fn logout_count(server: &matrix_sdk::test_utils::mocks::MatrixMockServer) -> usize {
        server
            .server()
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|request| request.url.path() == "/_matrix/client/v3/logout")
            .count()
    }

    /// A Swift `Task` cancelled between `/login` and the restore drops the
    /// sign-in future; the dropped sign-in still logs its session out.
    #[tokio::test]
    async fn a_cancelled_sign_in_logs_its_session_out() {
        let server = mock_server(200).await;
        let root = temp_root("cancelled");
        let config = detached_config(&root, &server.uri());
        let (signed_in_tx, signed_in_rx) = tokio::sync::oneshot::channel();
        let journal_root = root.clone();
        let caller = tokio::spawn(async move {
            let signed_in = DetachedSignIn::sign_in(
                config,
                &LoginUser::parse("Pete").unwrap(),
                "fixture-password",
                &LoginOptions::default(),
                &journal_root,
            )
            .await
            .expect("sign-in");
            let _ = signed_in_tx.send(signed_in.identity().user_id().to_owned());
            // Waiting on the store open when the caller goes away.
            std::future::pending::<()>().await;
            drop(signed_in);
        });
        assert_eq!(signed_in_rx.await.unwrap(), "@pete:matrix.example.org");
        assert_eq!(logout_count(&server).await, 0);
        caller.abort();
        let _ = caller.await;
        for _ in 0..100 {
            if logout_count(&server).await == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(logout_count(&server).await, 1);
        let _ = fs::remove_dir_all(root);
    }

    /// A revoke the server never confirms is retried, then recorded.
    #[tokio::test]
    async fn a_revoke_the_server_refuses_is_retried_then_journaled() {
        let server = mock_server(502).await;
        let root = temp_root("revoke-failed");
        let signed_in = DetachedSignIn::sign_in(
            detached_config(&root, &server.uri()),
            &LoginUser::parse("Pete").unwrap(),
            "fixture-password",
            &LoginOptions::default(),
            &root,
        )
        .await
        .expect("sign-in");
        signed_in.revoke();
        let journal = root.join("logs").join("matrix-session-lifecycle.log");
        for _ in 0..200 {
            if journal.is_file() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        // Each attempt is one SDK `/logout`, which itself retries a 5xx.
        assert!(logout_count(&server).await > REVOKE_RETRY_DELAYS.len());
        let text = fs::read_to_string(journal).unwrap();
        assert!(text.contains("native detached-sign-in-revoke-failed"));
        assert!(!text.contains("syt_detached") && !text.contains("@pete"));
        let _ = fs::remove_dir_all(root);
    }
}
