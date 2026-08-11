//! Read-only Matrix registration-flow probe (P2.5).
//!
//! The probe sends only an empty `POST /_matrix/client/v3/register` request
//! and turns a UIAA challenge into the existing React/Tauri-safe DTO. It never
//! accepts registration credentials, continues UIAA, or creates/persists a
//! Matrix session.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::error::AuthError;
use super::input::normalize_homeserver_url;

/// Registration UIAA stages supported by the existing desktop product flow.
pub const SUPPORTED_REGISTER_STAGES: &[&str] = &[
    "m.login.registration_token",
    "m.login.terms",
    "m.login.recaptcha",
    "m.login.email.identity",
    "m.login.dummy",
];

/// One registration UIAA flow as exposed to the React/Tauri boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUiaFlow {
    #[serde(default)]
    pub stages: Vec<String>,
}

/// Probe outcome for an empty registration request.
///
/// This is deliberately the exact existing desktop wire shape. It contains no
/// credentials, request body, raw homeserver errors, or transport diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RegisterFlowsProbe {
    /// Homeserver requires UIAA; product may show the registration form.
    #[serde(rename_all = "camelCase")]
    FlowRequired {
        session: Option<String>,
        flows: Vec<RegisterUiaFlow>,
        completed: Vec<String>,
        params: Option<JsonValue>,
    },
    RegistrationDisabled,
    RateLimited,
    InvalidRequest,
}

/// Read-only registration-flow transport. Implementations must not submit
/// password, token, email, captcha, auth continuation, or session material.
pub trait RegisterFlowsTransport {
    fn probe_register_flows(
        &self,
        homeserver_base_url: &str,
    ) -> impl std::future::Future<Output = Result<RegisterFlowsProbe, AuthError>> + Send;
}

/// Probe the registration UIAA flows for a validated homeserver base URL.
pub async fn probe_register_flows<T: RegisterFlowsTransport>(
    homeserver_base_url: &str,
    transport: &T,
) -> Result<RegisterFlowsProbe, AuthError> {
    let base = normalize_homeserver_url(homeserver_base_url)?.into_string();
    transport.probe_register_flows(&base).await
}

/// Parse a Matrix UIAA registration challenge without retaining its raw body.
///
/// The shape intentionally mirrors Ruma's permissive `UiaaInfo` decoding:
/// unknown top-level fields (such as Matrix's `errcode` / `error`) are ignored,
/// `completed`, `params`, and `session` are optional, and a flow's `stages`
/// defaults to empty. A missing or incorrectly typed `flows` value fails
/// closed.
pub fn parse_register_uiaa_json(raw: &str) -> Result<RegisterFlowsProbe, AuthError> {
    let response: RegisterUiaaResponse =
        serde_json::from_str(raw).map_err(|_| AuthError::UnsupportedCapability {
            diagnostic_id: "p2-register-flows-uiaa-response-invalid",
        })?;
    if has_unsupported_only_register_flows(&response.flows, &response.completed) {
        return Err(AuthError::UnsupportedCapability {
            diagnostic_id: "v-auth.4b-register-unsupported-uia-stage",
        });
    }
    Ok(RegisterFlowsProbe::FlowRequired {
        session: response.session,
        flows: response.flows,
        completed: response.completed,
        params: response.params,
    })
}

/// Whether every offered flow requires a stage unsupported by the product.
///
/// Completed stages are allowed, preserving the existing desktop policy.
pub fn has_unsupported_only_register_flows(
    flows: &[RegisterUiaFlow],
    completed: &[String],
) -> bool {
    !flows.is_empty()
        && !flows.iter().any(|flow| {
            flow.stages.iter().all(|stage| {
                completed
                    .iter()
                    .any(|completed_stage| completed_stage == stage)
                    || SUPPORTED_REGISTER_STAGES.contains(&stage.as_str())
            })
        })
}

#[derive(Deserialize)]
struct RegisterUiaaResponse {
    flows: Vec<RegisterUiaFlow>,
    #[serde(default)]
    completed: Vec<String>,
    #[serde(default)]
    params: Option<JsonValue>,
    #[serde(default)]
    session: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_preserves_the_supported_desktop_uiaa_wire_shape() {
        let result = parse_register_uiaa_json(
            r#"{
                "flows":[
                    {"stages":["m.login.terms","m.login.dummy"]},
                    {"stages":["m.login.registration_token"]}
                ],
                "completed":["m.login.terms"],
                "params":{"m.login.terms":{"policies":[]}},
                "session":"opaque-uia-session",
                "errcode":"M_UNAUTHORIZED",
                "error":"untrusted remote detail"
            }"#,
        )
        .expect("valid UIAA response");
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "status":"flow_required",
                "session":"opaque-uia-session",
                "flows":[
                    {"stages":["m.login.terms","m.login.dummy"]},
                    {"stages":["m.login.registration_token"]}
                ],
                "completed":["m.login.terms"],
                "params":{"m.login.terms":{"policies":[]}},
            })
        );
    }

    #[test]
    fn parser_fails_closed_for_malformed_or_unsupported_uiaa() {
        let malformed = parse_register_uiaa_json(
            r#"{"flows":"not-an-array","error":"sensitive remote error"}"#,
        )
        .expect_err("malformed UIAA response must fail closed");
        assert_eq!(
            malformed.diagnostic_id(),
            "p2-register-flows-uiaa-response-invalid"
        );
        assert!(!malformed.to_string().contains("sensitive remote error"));

        let unsupported = parse_register_uiaa_json(r#"{"flows":[{"stages":["m.login.sso"]}]}"#)
            .expect_err("unsupported-only UIAA must fail closed");
        assert_eq!(
            unsupported.diagnostic_id(),
            "v-auth.4b-register-unsupported-uia-stage"
        );
    }

    #[test]
    fn completed_unsupported_stage_preserves_existing_policy() {
        let result = parse_register_uiaa_json(
            r#"{
                "flows":[{"stages":["m.login.sso","m.login.dummy"]}],
                "completed":["m.login.sso"]
            }"#,
        )
        .expect("completed unsupported stage is not an unsupported-only flow");
        assert!(matches!(result, RegisterFlowsProbe::FlowRequired { .. }));
    }
}
