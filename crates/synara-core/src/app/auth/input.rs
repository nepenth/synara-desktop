//! Homeserver / server-name discovery inputs (normalize + validate).
//!
//! The shared core validates the homeserver base before any discovery or
//! login-flow request is assembled.

use super::error::AuthError;

/// How the user (or product UI) supplied the homeserver location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryInput {
    /// Explicit homeserver base URL (`https://matrix.example.org`).
    HomeserverUrl(String),
    /// Matrix server name only (`example.org` or `example.org:8448`); a shell
    /// discovery adapter may resolve it through well-known discovery.
    ServerName(String),
    /// Ambiguous user typing: try server-name discovery first, then treat as URL.
    ServerNameOrUrl(String),
}

/// Which input shape produced a successful discovery result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoveryInputKind {
    HomeserverUrl,
    ServerName,
    ServerNameOrUrlAsServerName,
    ServerNameOrUrlAsUrl,
}

impl DiscoveryInputKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HomeserverUrl => "homeserver_url",
            Self::ServerName => "server_name",
            Self::ServerNameOrUrlAsServerName => "server_name_or_url_as_server_name",
            Self::ServerNameOrUrlAsUrl => "server_name_or_url_as_url",
        }
    }
}

/// Normalized, validated homeserver base URL (HTTPS, or HTTP loopback for development).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedHomeserverUrl {
    url: String,
}

impl NormalizedHomeserverUrl {
    pub fn as_str(&self) -> &str {
        &self.url
    }

    pub fn into_string(self) -> String {
        self.url
    }
}

/// Normalized Matrix server name (hostname[:port], no scheme/path).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedServerName {
    name: String,
}

impl NormalizedServerName {
    pub fn as_str(&self) -> &str {
        &self.name
    }

    pub fn into_string(self) -> String {
        self.name
    }
}

/// Normalize and validate an explicit homeserver base URL.
///
/// - Trims whitespace
/// - Strips a single trailing `/`
/// - Requires `https://`, except `http://` is permitted for loopback development
/// - Rejects whitespace, NUL, `..`, empty host
pub fn normalize_homeserver_url(raw: &str) -> Result<NormalizedHomeserverUrl, AuthError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-empty-homeserver-url",
            reason: "homeserver url is empty",
        });
    }
    if !is_plausible_homeserver_url(trimmed) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid-homeserver-url",
            reason: "homeserver url must use https (http is loopback-only)",
        });
    }
    Ok(NormalizedHomeserverUrl {
        url: trimmed.to_owned(),
    })
}

/// Normalize and validate a Matrix server name (no scheme).
///
/// Accepts `hostname`, `hostname:port`, and IDNA-ish host labels. Rejects
/// schemes, paths, userinfo, query, fragment, and empty labels.
pub fn normalize_server_name(raw: &str) -> Result<NormalizedServerName, AuthError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-empty-server-name",
            reason: "server name is empty",
        });
    }
    // Strip accidental scheme if the product typed a bare server host with https.
    let without_scheme = strip_url_scheme(trimmed);
    if without_scheme.contains(['/', '?', '#', '@', '\\', '\0', ' ', '\n', '\r', '\t']) {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid-server-name",
            reason: "server name contains path, query, or forbidden characters",
        });
    }
    if without_scheme.contains("..") {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid-server-name-dotdot",
            reason: "server name must not contain path traversal segments",
        });
    }
    let host_port = without_scheme;
    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (h, Some(p)),
        _ => (host_port, None),
    };
    if host.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-empty-server-host",
            reason: "server host is empty",
        });
    }
    // Bracketed IPv6 is allowed; simple hostname labels otherwise.
    let host_ok = if host.starts_with('[') && host.ends_with(']') {
        host.len() > 2
    } else {
        host.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
            && !host.starts_with('.')
            && !host.ends_with('.')
            && host.contains(|c: char| c.is_ascii_alphanumeric())
    };
    if !host_ok {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid-server-host",
            reason: "server host is not a plausible hostname",
        });
    }
    if let Some(p) = port {
        let n: u32 = p.parse().map_err(|_| AuthError::InvalidInput {
            diagnostic_id: "p3.1-invalid-server-port",
            reason: "server port is not a valid number",
        })?;
        if n == 0 || n > 65535 {
            return Err(AuthError::InvalidInput {
                diagnostic_id: "p3.1-invalid-server-port-range",
                reason: "server port out of range",
            });
        }
    }
    // Lowercase host for stability (ports stay as-is).
    let name = match port {
        Some(p) => format!("{}:{}", host.to_ascii_lowercase(), p),
        None => host.to_ascii_lowercase(),
    };
    Ok(NormalizedServerName { name })
}

/// Parse flexible user input into a validated [`DiscoveryInput`].
///
/// - Strings that look like URLs → `HomeserverUrl`
/// - Pure server names → `ServerName`
/// - When `prefer_ambiguous` is true, non-URL strings stay as `ServerNameOrUrl`
///   so the discovery service can attempt well-known then URL fallback.
pub fn parse_discovery_input(
    raw: &str,
    prefer_ambiguous: bool,
) -> Result<DiscoveryInput, AuthError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AuthError::InvalidInput {
            diagnostic_id: "p3.1-empty-discovery-input",
            reason: "discovery input is empty",
        });
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        let url = normalize_homeserver_url(trimmed)?;
        return Ok(DiscoveryInput::HomeserverUrl(url.into_string()));
    }
    if prefer_ambiguous {
        // Keep original (trimmed) for ServerNameOrUrl; discovery normalizes paths.
        return Ok(DiscoveryInput::ServerNameOrUrl(trimmed.to_owned()));
    }
    let name = normalize_server_name(trimmed)?;
    Ok(DiscoveryInput::ServerName(name.into_string()))
}

fn strip_url_scheme(s: &str) -> &str {
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    {
        // Map back to original slice length.
        let prefix_len = s.len() - rest.len();
        return &s[prefix_len..];
    }
    s
}

fn is_plausible_homeserver_url(url: &str) -> bool {
    if url.contains(['\0', ' ', '\n', '\r', '\t']) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    // Preserve the existing traversal rejection and reject percent-encoded
    // separators/dot segments before a request URL is assembled.
    if url.contains("..") || lower.contains("%2e") || lower.contains("%2f") || lower.contains("%5c")
    {
        return false;
    }

    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    if !matches!(parsed.scheme(), "https" | "http")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || parsed.port() == Some(0)
    {
        return false;
    }

    if parsed.scheme() == "http"
        && !parsed
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]"))
    {
        return false;
    }

    // A homeserver base may include a deployment path, but never a dot path
    // segment that could alter the appended Client-Server endpoint.
    parsed
        .path_segments()
        .is_none_or(|mut segments| !segments.any(|segment| matches!(segment, "." | "..")))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn normalize_url_trims_and_strips_slash() {
        let u = normalize_homeserver_url("  https://Example.org/  ").unwrap();
        assert_eq!(u.as_str(), "https://Example.org");
    }

    #[test]
    fn normalize_url_rejects_empty_and_no_scheme() {
        assert!(normalize_homeserver_url("").is_err());
        assert!(normalize_homeserver_url("example.org").is_err());
        assert!(normalize_homeserver_url("ftp://example.org").is_err());
    }

    #[test]
    fn normalize_url_requires_tls_except_for_exact_loopback_hosts() {
        assert!(normalize_homeserver_url("http://matrix.example.org").is_err());
        assert!(normalize_homeserver_url("http://192.168.1.20:8008").is_err());
        assert!(normalize_homeserver_url("http://localhost:8008").is_ok());
        assert!(normalize_homeserver_url("http://127.0.0.1:8008").is_ok());
        assert!(normalize_homeserver_url("http://[::1]:8008").is_ok());
    }

    #[test]
    fn normalize_server_name_lowercases() {
        let n = normalize_server_name("Example.ORG").unwrap();
        assert_eq!(n.as_str(), "example.org");
        let n2 = normalize_server_name("Example.ORG:8448").unwrap();
        assert_eq!(n2.as_str(), "example.org:8448");
    }

    #[test]
    fn normalize_server_name_rejects_path() {
        assert!(normalize_server_name("example.org/path").is_err());
        assert!(normalize_server_name("https://example.org/foo").is_err());
    }

    #[test]
    fn normalize_url_rejects_unsafe_components_without_echoing_them() {
        for raw in [
            "https://example.org?endpoint=/other",
            "https://example.org/#fragment",
            "https://user@example.org",
            "https://example.org/%2e%2e/private",
            "https://example.org:0",
        ] {
            let error = normalize_homeserver_url(raw).expect_err("unsafe URL must fail closed");
            assert_eq!(error.diagnostic_id(), "p3.1-invalid-homeserver-url");
            assert!(!error.to_string().contains(raw));
        }
    }
}
