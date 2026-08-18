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
    assert!(normalize_homeserver_url("http://matrix.example.org").is_err());
    assert!(normalize_homeserver_url("http://localhost:8008").is_ok());
    assert!(normalize_homeserver_url("http://127.0.0.1:8008").is_ok());

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
fn end_to_end_discovery_then_login_flows_with_mocks() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let wk = WellKnownClientConfig::new("https://matrix.example.org", None).unwrap();
    let disc = MockDiscoveryTransport::new().with_response("example.org", wk);
    let flows_transport = MockLoginFlowTransport::new().with_response(
        "https://matrix.example.org",
        vec![LoginFlow::password(), LoginFlow::token(false)],
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
    assert!(flows.supports(LoginFlowKind::Token));
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

// --- P3.4 UIA session ---

fn password_then_dummy_stages() -> Vec<UiaStage> {
    vec![
        UiaStage {
            kind: UiaStageKind::Password,
            matrix_type: "m.login.password".into(),
            public_param_id: None,
        },
        UiaStage {
            kind: UiaStageKind::Dummy,
            matrix_type: "m.login.dummy".into(),
            public_param_id: None,
        },
    ]
}

#[test]
fn uia_happy_path_two_stages() {
    let mut s = UiaSession::new(3);
    let op = s
        .begin(UiaFlowKind::Login, "sess-1", password_then_dummy_stages())
        .unwrap();
    assert_eq!(s.phase(), UiaPhase::ChallengePending);
    assert_eq!(s.current_stage().unwrap().kind, UiaStageKind::Password);
    assert_eq!(s.begin_submit(op).unwrap(), UiaStageKind::Password);
    s.stage_accepted(op).unwrap();
    assert_eq!(s.current_stage().unwrap().kind, UiaStageKind::Dummy);
    s.begin_submit(op).unwrap();
    s.stage_accepted(op).unwrap();
    assert_eq!(s.phase(), UiaPhase::Completed);
    assert!(s.uia_session_id().is_none());
    let out = s.complete_success(op).unwrap();
    assert_eq!(out.stages_completed, 2);
    assert_eq!(out.flow_kind, UiaFlowKind::Login);
    assert!(s.never_stores_secrets());
}

#[test]
fn uia_stage_rejected_retries_same_stage() {
    let mut s = UiaSession::new(1);
    let op = s
        .begin(
            UiaFlowKind::Registration,
            "reg",
            password_then_dummy_stages(),
        )
        .unwrap();
    s.begin_submit(op).unwrap();
    s.stage_rejected(op, "p3.4-bad-password").unwrap();
    assert_eq!(s.phase(), UiaPhase::ChallengePending);
    assert_eq!(s.current_stage().unwrap().kind, UiaStageKind::Password);
    assert_eq!(s.failure_diagnostic_id(), Some("p3.4-bad-password"));
    assert_eq!(s.stages_completed(), 0);
}

#[test]
fn uia_empty_stages_and_cap() {
    let mut s = UiaSession::new(1);
    let err = s.begin(UiaFlowKind::Login, "s", vec![]).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.4-empty-stages");
    let many: Vec<UiaStage> = (0..=MAX_UIA_STAGES)
        .map(|i| UiaStage {
            kind: UiaStageKind::Dummy,
            matrix_type: format!("m.login.dummy.{i}"),
            public_param_id: None,
        })
        .collect();
    let err = s.begin(UiaFlowKind::Login, "s", many).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.4-stage-cap");
}

#[test]
fn uia_cancel_and_stale_op() {
    let mut s = UiaSession::new(1);
    let op = s
        .begin(
            UiaFlowKind::PasswordReset,
            "s",
            password_then_dummy_stages(),
        )
        .unwrap();
    s.cancel(op).unwrap();
    assert_eq!(s.phase(), UiaPhase::Cancelled);
    let err = s.begin_submit(op).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.4-submit-wrong-phase");
    s.reset();
    let op2 = s
        .begin(UiaFlowKind::StepUp, "s2", password_then_dummy_stages())
        .unwrap();
    let err = s.begin_submit(op2 + 1).unwrap_err();
    assert_eq!(err.diagnostic_id(), "p3.4-stale-uia-op");
}

#[test]
fn uia_stage_kind_from_matrix_type() {
    assert_eq!(
        UiaStageKind::from_matrix_type("m.login.recaptcha"),
        UiaStageKind::Recaptcha
    );
    assert!(UiaStageKind::Password.requires_secret_input());
    assert!(!UiaStageKind::Dummy.requires_secret_input());
}

#[test]
fn uia_retire_generation() {
    let mut s = UiaSession::new(1);
    s.begin(UiaFlowKind::Login, "s", password_then_dummy_stages())
        .unwrap();
    s.retire_generation(9);
    assert_eq!(s.session_generation(), 9);
    assert!(!s.is_active());
    assert_eq!(s.phase(), UiaPhase::Idle);
}
