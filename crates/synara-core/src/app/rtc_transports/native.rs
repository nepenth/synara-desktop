//! Privacy-safe MatrixRTC transport snapshot. URLs only; never JWTs or tokens.

use serde::{Deserialize, Serialize};

pub const RTC_TRANSPORTS_MARKER: &str = "matrix-rtc-transports-msc4143";
pub const MAX_RTC_TRANSPORTS: usize = 8;
const MAX_SERVICE_URL_CHARS: usize = 2048;

/// Closed discovery outcome. `unsupported` is missing endpoint / nothing
/// discovered (`None`). `unavailable` is an advertised empty list or a
/// failed lookup. `ready` is at least one mapped transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeRtcTransportsStatus {
    Ready,
    Unsupported,
    Unavailable,
}

/// Closed transport kind. Custom types are named only; extra JSON is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeRtcTransportKind {
    #[serde(rename = "livekit")]
    Livekit,
    Custom,
}

impl NativeRtcTransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Livekit => "livekit",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRtcTransport {
    pub kind: NativeRtcTransportKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRtcTransportsSnapshot {
    pub session_generation: u64,
    pub status: NativeRtcTransportsStatus,
    pub transports: Vec<NativeRtcTransport>,
}

impl NativeRtcTransportsSnapshot {
    pub fn unsupported(session_generation: u64) -> Self {
        Self {
            session_generation,
            status: NativeRtcTransportsStatus::Unsupported,
            transports: Vec::new(),
        }
    }

    pub fn unavailable(session_generation: u64) -> Self {
        Self {
            session_generation,
            status: NativeRtcTransportsStatus::Unavailable,
            transports: Vec::new(),
        }
    }
}

/// Map SDK discovery: `None` → unsupported, `Some([])` → unavailable,
/// `Some(non-empty)` → ready. Never copies JWTs, tokens, or widget types.
pub fn map_discovered_rtc_transports(
    session_generation: u64,
    discovered: Option<Vec<matrix_sdk::ruma::api::client::rtc::RtcTransport>>,
) -> NativeRtcTransportsSnapshot {
    match discovered {
        None => NativeRtcTransportsSnapshot::unsupported(session_generation),
        Some(list) if list.is_empty() => {
            NativeRtcTransportsSnapshot::unavailable(session_generation)
        }
        Some(list) => NativeRtcTransportsSnapshot {
            session_generation,
            status: NativeRtcTransportsStatus::Ready,
            transports: list
                .iter()
                .take(MAX_RTC_TRANSPORTS)
                .map(map_one_transport)
                .collect(),
        },
    }
}

fn map_one_transport(
    transport: &matrix_sdk::ruma::api::client::rtc::RtcTransport,
) -> NativeRtcTransport {
    if transport.transport_type() == "livekit" {
        NativeRtcTransport {
            kind: NativeRtcTransportKind::Livekit,
            service_url: sanitize_service_url(
                transport
                    .data()
                    .get("livekit_service_url")
                    .and_then(|value| value.as_str()),
            ),
        }
    } else {
        NativeRtcTransport {
            kind: NativeRtcTransportKind::Custom,
            service_url: None,
        }
    }
}

fn sanitize_service_url(raw: Option<&str>) -> Option<String> {
    let value = raw?.trim();
    if value.is_empty() || value.len() > MAX_SERVICE_URL_CHARS {
        return None;
    }
    let parsed = url::Url::parse(value).ok()?;
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return None;
    }
    if parsed.username() != "" || parsed.password().is_some() {
        return None;
    }
    Some(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::api::client::rtc::RtcTransport;
    use serde_json::{json, Value};

    fn transport(value: Value) -> RtcTransport {
        serde_json::from_value(value).expect("rtc transport fixture")
    }

    #[test]
    fn none_maps_to_unsupported() {
        let snapshot = map_discovered_rtc_transports(3, None);
        assert_eq!(snapshot.status, NativeRtcTransportsStatus::Unsupported);
        assert!(snapshot.transports.is_empty());
        assert_eq!(snapshot.session_generation, 3);
    }

    #[test]
    fn empty_list_maps_to_unavailable() {
        let snapshot = map_discovered_rtc_transports(3, Some(Vec::new()));
        assert_eq!(snapshot.status, NativeRtcTransportsStatus::Unavailable);
        assert!(snapshot.transports.is_empty());
    }

    #[test]
    fn livekit_maps_service_url_only() {
        let snapshot = map_discovered_rtc_transports(
            4,
            Some(vec![transport(json!({
                "type": "livekit",
                "livekit_service_url": "https://livekit.example.org/jwt"
            }))]),
        );
        assert_eq!(snapshot.status, NativeRtcTransportsStatus::Ready);
        assert_eq!(snapshot.transports.len(), 1);
        assert_eq!(snapshot.transports[0].kind, NativeRtcTransportKind::Livekit);
        assert_eq!(
            snapshot.transports[0].service_url.as_deref(),
            Some("https://livekit.example.org/jwt")
        );
        let wire = serde_json::to_value(&snapshot).expect("serialize");
        assert_eq!(wire["status"], "ready");
        assert_eq!(wire["sessionGeneration"], 4);
        assert_eq!(wire["transports"][0]["kind"], "livekit");
        assert_eq!(
            wire["transports"][0]["serviceUrl"],
            "https://livekit.example.org/jwt"
        );
        let raw = serde_json::to_string(&snapshot).expect("serialize string");
        for forbidden in [
            "jwt_token",
            "accessToken",
            "access_token",
            "widget",
            "password",
        ] {
            assert!(!raw.contains(forbidden), "{raw}");
        }
    }

    #[test]
    fn custom_transport_drops_extra_fields() {
        let snapshot = map_discovered_rtc_transports(
            1,
            Some(vec![transport(json!({
                "type": "local.custom.sfu",
                "jwt": "secret.token.value",
                "authorization": "Bearer abc"
            }))]),
        );
        assert_eq!(snapshot.status, NativeRtcTransportsStatus::Ready);
        assert_eq!(snapshot.transports[0].kind, NativeRtcTransportKind::Custom);
        assert!(snapshot.transports[0].service_url.is_none());
        let raw = serde_json::to_string(&snapshot).expect("serialize");
        assert!(!raw.contains("secret.token.value"));
        assert!(!raw.contains("Bearer"));
        assert!(!raw.contains("authorization"));
        assert!(!raw.contains("widget"));
    }

    #[test]
    fn livekit_drops_userinfo_and_non_http_urls() {
        let snapshot = map_discovered_rtc_transports(
            1,
            Some(vec![
                transport(json!({
                    "type": "livekit",
                    "livekit_service_url": "javascript:alert(1)"
                })),
                transport(json!({
                    "type": "livekit",
                    "livekit_service_url": "https://user:token@livekit.example.org/"
                })),
            ]),
        );
        assert_eq!(snapshot.transports.len(), 2);
        assert!(snapshot
            .transports
            .iter()
            .all(|row| row.service_url.is_none()));
    }
}
