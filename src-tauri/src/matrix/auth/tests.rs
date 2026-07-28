//! Unit tests for P3.1 discovery + login-flow service (mocks only; no live network).

use super::*;
use crate::matrix::ipc::MatrixIpcErrorCategory;

fn secret_fragments() -> &'static [&'static str] {
    &[
        "syt_secret_access_token_value_xyz",
        "refresh_token=rrrr",
        "password=hunter2",
        "Bearer abcdef",
    ]
}

#[test]
fn marker_stable() {
    assert_eq!(matrix_auth_markers(), MATRIX_AUTH_MARKER);
}

#[test]
fn valid_homeserver_url_input_no_network() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockDiscoveryTransport::new();
    let input = DiscoveryInput::HomeserverUrl("https://matrix.example.org/".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .expect("url discovery");
    assert_eq!(result.homeserver_base_url, "https://matrix.example.org");
    assert!(!result.used_well_known);
    assert_eq!(result.input_kind, DiscoveryInputKind::HomeserverUrl);
    assert!(result.identity_server_base_url.is_none());
}

#[test]
fn invalid_homeserver_url_rejected() {
    assert!(normalize_homeserver_url("").is_err());
    assert!(normalize_homeserver_url("not a url").is_err());
    assert!(normalize_homeserver_url("ftp://example.org").is_err());
    assert!(normalize_homeserver_url("https://").is_err());
    assert!(normalize_homeserver_url("https://example.org/../evil").is_err());

    let err = normalize_homeserver_url("").unwrap_err();
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
    assert_eq!(err.diagnostic_id(), "p3.1-empty-homeserver-url");
}

#[test]
fn invalid_server_name_rejected() {
    assert!(normalize_server_name("").is_err());
    assert!(normalize_server_name("example.org/path").is_err());
    assert!(normalize_server_name("user@example.org").is_err());
    assert!(normalize_server_name("example.org:99999").is_err());
    assert!(normalize_server_name("..").is_err());
}

#[test]
fn well_known_mock_success() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let wk = WellKnownClientConfig::new(
        "https://matrix-client.example.org",
        Some("https://identity.example.org".into()),
    )
    .unwrap();
    let transport = MockDiscoveryTransport::new().with_response("example.org", wk);
    let input = DiscoveryInput::ServerName("Example.ORG".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .expect("well-known");
    assert!(result.used_well_known);
    assert_eq!(result.server_name.as_deref(), Some("example.org"));
    assert_eq!(
        result.homeserver_base_url,
        "https://matrix-client.example.org"
    );
    assert_eq!(
        result.identity_server_base_url.as_deref(),
        Some("https://identity.example.org")
    );
    assert_eq!(result.input_kind, DiscoveryInputKind::ServerName);
}

#[test]
fn well_known_mock_failure_maps_category() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockDiscoveryTransport::new().with_error(
        "down.example",
        AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.1-well-known-down",
        },
    );
    let input = DiscoveryInput::ServerName("down.example".into());
    let err = rt
        .block_on(discover_homeserver(&input, &transport))
        .expect_err("must fail");
    assert_eq!(
        err.category(),
        MatrixIpcErrorCategory::HomeserverUnavailable
    );
    assert_eq!(err.diagnostic_id(), "p3.1-well-known-down");
}

#[test]
fn well_known_not_found_ignore_fallback_https_base() {
    // Product autoDiscovery IGNORE (404): use https://{server} as base URL.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockDiscoveryTransport::new().with_error(
        "local.example",
        AuthError::WellKnownNotFound {
            diagnostic_id: "p3.1-well-known-404",
        },
    );
    let input = DiscoveryInput::ServerName("local.example".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .expect("IGNORE fallback");
    assert!(!result.used_well_known);
    assert_eq!(result.homeserver_base_url, "https://local.example");
    assert_eq!(result.server_name.as_deref(), Some("local.example"));
}

#[test]
fn server_name_or_url_not_found_also_ignores_to_https() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockDiscoveryTransport::new().with_error(
        "bare.example",
        AuthError::WellKnownNotFound {
            diagnostic_id: "p3.1-well-known-404",
        },
    );
    let input = DiscoveryInput::ServerNameOrUrl("bare.example".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .unwrap();
    assert_eq!(result.homeserver_base_url, "https://bare.example");
    assert!(!result.used_well_known);
}

#[test]
fn well_known_connectivity_failure() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockDiscoveryTransport::new().with_error(
        "offline.example",
        AuthError::Connectivity {
            diagnostic_id: "p3.1-well-known-offline",
        },
    );
    let input = DiscoveryInput::ServerName("offline.example".into());
    let err = rt
        .block_on(discover_homeserver(&input, &transport))
        .expect_err("offline");
    assert_eq!(err.category(), MatrixIpcErrorCategory::Connectivity);
}

#[test]
fn server_name_or_url_prefers_well_known() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let wk = WellKnownClientConfig::new("https://hs.example.org", None).unwrap();
    let transport = MockDiscoveryTransport::new().with_response("example.org", wk);
    let input = DiscoveryInput::ServerNameOrUrl("example.org".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .unwrap();
    assert!(result.used_well_known);
    assert_eq!(
        result.input_kind,
        DiscoveryInputKind::ServerNameOrUrlAsServerName
    );
    assert_eq!(result.homeserver_base_url, "https://hs.example.org");
}

#[test]
fn server_name_or_url_falls_back_to_url_when_scheme_present() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // Input is a full URL that is NOT a valid bare server name (has scheme+host).
    // normalize_server_name on "https://direct.example.org" strips scheme → host ok,
    // so well-known is attempted first; on failure we fall back because scheme present.
    let transport = MockDiscoveryTransport::new().with_error(
        "direct.example.org",
        AuthError::UnsupportedCapability {
            diagnostic_id: "p3.1-no-well-known",
        },
    );
    let input = DiscoveryInput::ServerNameOrUrl("https://direct.example.org".into());
    let result = rt
        .block_on(discover_homeserver(&input, &transport))
        .unwrap();
    assert!(!result.used_well_known);
    assert_eq!(result.input_kind, DiscoveryInputKind::ServerNameOrUrlAsUrl);
    assert_eq!(result.homeserver_base_url, "https://direct.example.org");
}

#[test]
fn parse_discovery_input_shapes() {
    let url = parse_discovery_input("https://hs.example.org/", false).unwrap();
    assert!(matches!(url, DiscoveryInput::HomeserverUrl(_)));
    let name = parse_discovery_input("example.org", false).unwrap();
    assert!(matches!(name, DiscoveryInput::ServerName(_)));
    let amb = parse_discovery_input("example.org", true).unwrap();
    assert!(matches!(amb, DiscoveryInput::ServerNameOrUrl(_)));
    assert!(parse_discovery_input("", false).is_err());
}

#[test]
fn login_flow_list_mapping() {
    let mapped = map_matrix_login_types(&[
        "m.login.password",
        "m.login.token",
        "m.login.sso",
        "m.login.application_service",
        "m.login.custom.widget",
    ]);
    assert_eq!(mapped.len(), 5);
    assert_eq!(mapped[0].kind, LoginFlowKind::Password);
    assert_eq!(mapped[1].kind, LoginFlowKind::Token);
    assert_eq!(mapped[2].kind, LoginFlowKind::Sso);
    assert_eq!(mapped[3].kind, LoginFlowKind::ApplicationService);
    assert_eq!(mapped[4].kind, LoginFlowKind::Unknown);
    assert_eq!(mapped[4].matrix_type, "m.login.custom.widget");
}

#[test]
fn login_flow_discovery_mock_success() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let flows = vec![
        LoginFlow::password(),
        LoginFlow::sso(
            vec![SsoIdentityProvider {
                id: "oidc".into(),
                name: "OIDC".into(),
                brand: Some("oidc".into()),
            }],
            false,
        ),
        LoginFlow::token(true),
    ];
    let transport =
        MockLoginFlowTransport::new().with_response("https://hs.example.org", flows.clone());
    let result = rt
        .block_on(discover_login_flows("https://hs.example.org/", &transport))
        .expect("flows");
    assert_eq!(result.homeserver_base_url, "https://hs.example.org");
    assert!(result.password_available());
    assert!(result.sso_available());
    assert!(result.supports(LoginFlowKind::Token));
    assert_eq!(result.flows.len(), 3);
    assert_eq!(result.flows[1].identity_providers[0].id, "oidc");
    assert_eq!(result.flows[2].get_login_token, Some(true));
}

#[test]
fn login_flow_discovery_mock_failure() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transport = MockLoginFlowTransport::new().with_error(
        "https://hs.example.org",
        AuthError::Connectivity {
            diagnostic_id: "p3.1-login-flows-offline",
        },
    );
    let err = rt
        .block_on(discover_login_flows("https://hs.example.org", &transport))
        .expect_err("offline");
    assert_eq!(err.category(), MatrixIpcErrorCategory::Connectivity);
}

#[test]
fn end_to_end_discovery_then_login_flows_with_mocks() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let wk = WellKnownClientConfig::new("https://matrix.example.org", None).unwrap();
    let disc = MockDiscoveryTransport::new().with_response("example.org", wk);
    let flows_transport = MockLoginFlowTransport::new().with_response(
        "https://matrix.example.org",
        vec![LoginFlow::password(), LoginFlow::sso(vec![], false)],
    );

    let (discovered, flows) = rt
        .block_on(discover_homeserver_and_login_flows(
            &DiscoveryInput::ServerName("example.org".into()),
            &disc,
            &flows_transport,
        ))
        .unwrap();
    assert_eq!(discovered.homeserver_base_url, "https://matrix.example.org");
    assert!(flows.password_available());
    assert!(flows.sso_available());
}

#[test]
fn client_builder_bridge_sets_homeserver_from_discovery() {
    let discovery = DiscoveryResult {
        input_kind: DiscoveryInputKind::ServerName,
        server_name: Some("example.org".into()),
        homeserver_base_url: "https://matrix.example.org".into(),
        identity_server_base_url: None,
        used_well_known: true,
    };
    let identity = identity_with_discovered_homeserver("@alice:example.org", &discovery).unwrap();
    assert_eq!(identity.user_id(), "@alice:example.org");
    assert_eq!(identity.homeserver_url(), "https://matrix.example.org");
    assert_eq!(
        homeserver_url_for_client_builder(&discovery).unwrap(),
        "https://matrix.example.org"
    );
}

#[test]
fn privacy_errors_never_echo_tokens_or_passwords() {
    // Construct errors the way production code does — only diagnostic ids.
    let errors = [
        AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid",
            reason: "homeserver url is invalid",
        },
        AuthError::Connectivity {
            diagnostic_id: "p3.1-offline",
        },
        AuthError::HomeserverUnavailable {
            diagnostic_id: "p3.1-hs-down",
        },
        AuthError::WellKnownNotFound {
            diagnostic_id: "p3.1-wk-404",
        },
        AuthError::UnsupportedCapability {
            diagnostic_id: "p3.1-no-wk",
        },
        AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected",
        },
        AuthError::UserDeactivated {
            diagnostic_id: "p3.2-login-user-deactivated",
        },
        AuthError::InteractiveAuthRequired {
            diagnostic_id: "p3.2-login-uiaa-required",
        },
        AuthError::RateLimited {
            diagnostic_id: "p3.2-login-rate-limited",
            retry_after_ms: Some(1_000),
        },
        AuthError::SdkInvariant {
            diagnostic_id: "p3.1-invariant",
        },
        AuthError::Unknown {
            diagnostic_id: "p3.1-unknown",
        },
    ];
    for err in &errors {
        assert!(
            err.display_is_privacy_safe(secret_fragments()),
            "leaked secret in: {err}"
        );
        let text = format!("{err:?}");
        for frag in secret_fragments() {
            assert!(
                !text
                    .to_ascii_lowercase()
                    .contains(&frag.to_ascii_lowercase()),
                "debug leaked {frag}: {text}"
            );
        }
        // Categories are the stable IPC surface — never free-form token fields.
        let _ = err.category();
        assert!(!err.diagnostic_id().is_empty());
    }
    assert_eq!(
        AuthError::AuthenticationRejected {
            diagnostic_id: "p3.2-login-rejected",
        }
        .category(),
        MatrixIpcErrorCategory::AuthenticationRejected
    );
}

#[test]
fn platform_device_display_names_are_product_fixed() {
    assert_eq!(DEVICE_DISPLAY_NAME_MACOS, "Synara macOS");
    assert_eq!(DEVICE_DISPLAY_NAME_LINUX, "Synara Linux");
    let host = platform_device_display_name();
    assert!(host.starts_with("Synara "), "got {host}");
    assert_eq!(host_device_platform().device_display_name(), host);
}

#[test]
fn well_known_config_rejects_invalid_base_url() {
    let err = WellKnownClientConfig::new("not-a-url", None).unwrap_err();
    assert_eq!(err.category(), MatrixIpcErrorCategory::SdkInvariant);
}

#[test]
fn login_flow_kind_matrix_type_roundtrip() {
    for kind in LoginFlowKind::ALL_KNOWN {
        let mt = kind.matrix_type().unwrap();
        assert_eq!(LoginFlowKind::from_matrix_type(mt), *kind);
        assert!(!kind.as_str().is_empty());
    }
}

// --- P3.3 SSO callback lifecycle ---

#[test]
fn sso_happy_path() {
    let mut flow = SsoCallbackFlow::new(2);
    let op = flow
        .begin(
            "state-abc",
            "synara-desktop://sso-callback",
            Some("github".into()),
            Some("https://matrix.example.org".into()),
        )
        .unwrap();
    assert_eq!(flow.phase(), SsoCallbackPhase::AwaitingBrowser);
    assert!(flow.is_active());
    flow.on_callback("state-abc", op).unwrap();
    assert_eq!(flow.phase(), SsoCallbackPhase::CallbackReceived);
    flow.begin_exchange(op).unwrap();
    let out = flow.complete_success(op).unwrap();
    assert_eq!(out.session_generation, 2);
    assert_eq!(out.idp_id.as_deref(), Some("github"));
    assert_eq!(flow.phase(), SsoCallbackPhase::Succeeded);
    assert!(flow.never_stores_tokens());
    assert!(flow.state_id().is_none());
}

#[test]
fn sso_state_mismatch_fails() {
    let mut flow = SsoCallbackFlow::new(1);
    let op = flow
        .begin("good", "https://app.example.org/cb", None, None)
        .unwrap();
    let err = flow.on_callback("bad", op).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.3-state-mismatch");
    assert_eq!(flow.phase(), SsoCallbackPhase::Failed);
}

#[test]
fn sso_forbids_secret_redirect_and_http() {
    let mut flow = SsoCallbackFlow::new(1);
    let err = flow
        .begin("s", "http://insecure.example/cb", None, None)
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.3-forbidden-redirect-scheme");
    let err = flow
        .begin(
            "s",
            "https://app.example.org/cb?access_token=leak",
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.3-redirect-forbids-secrets");
}

#[test]
fn sso_cancel_and_stale_op() {
    let mut flow = SsoCallbackFlow::new(1);
    let op = flow.begin("s", "synara://sso", None, None).unwrap();
    flow.cancel(op).unwrap();
    assert_eq!(flow.phase(), SsoCallbackPhase::Cancelled);
    let err = flow.begin_exchange(op).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.3-exchange-wrong-phase");
    flow.reset();
    let op2 = flow.begin("s2", "synara://sso", None, None).unwrap();
    let err = flow.on_callback("s2", op2 + 99).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.3-stale-sso-op");
}

#[test]
fn sso_retire_generation_wipes() {
    let mut flow = SsoCallbackFlow::new(1);
    flow.begin("s", "synara://sso", None, None).unwrap();
    flow.retire_generation(5);
    assert_eq!(flow.session_generation(), 5);
    assert!(!flow.is_active());
    assert_eq!(flow.phase(), SsoCallbackPhase::Idle);
}
