use url::Url;

fn normalize_url_host(host: &str) -> String {
    host.trim_matches(|character| character == '[' || character == ']')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn is_local_hostname(host: &str) -> bool {
    let host = normalize_url_host(host);
    if host == "localhost" || host == "0.0.0.0" {
        return true;
    }

    const LOCAL_SUFFIXES: &[&str] = &[
        ".localhost",
        ".local",
        ".localdomain",
        ".internal",
        ".lan",
        ".home.arpa",
    ];
    LOCAL_SUFFIXES.iter().any(|suffix| host.ends_with(suffix))
}

fn is_private_ipv4(host: &str) -> bool {
    let host = normalize_url_host(host);
    if host.starts_with("0.")
        || host.starts_with("10.")
        || host.starts_with("127.")
        || host.starts_with("169.254.")
        || host.starts_with("192.168.")
    {
        return true;
    }

    if let Some(second_octet) = host
        .split('.')
        .nth(1)
        .and_then(|value| value.parse::<u8>().ok())
    {
        if host.starts_with("172.") && (16..=31).contains(&second_octet) {
            return true;
        }
        if host.starts_with("100.") && (64..=127).contains(&second_octet) {
            return true;
        }
    }

    false
}

fn is_private_ipv6(host: &str) -> bool {
    let host = normalize_url_host(host);
    host == "::1"
        || host == "::"
        || host.starts_with("fc")
        || host.starts_with("fd")
        || host.starts_with("fe80")
        || host.starts_with("::ffff:")
}

fn is_safe_public_https_host(host: &str) -> bool {
    let normalized = normalize_url_host(host);
    if normalized.is_empty() || is_local_hostname(&normalized) {
        return false;
    }

    if normalized.contains(':') {
        return !is_private_ipv6(&normalized);
    }

    !is_private_ipv4(&normalized)
}

fn is_safe_structured_external_url(url: &Url) -> bool {
    match url.scheme() {
        "mailto" => {
            let address = url.path().trim();
            !address.is_empty() && address.contains('@')
        }
        "matrix" => {
            url.host_str().is_some()
                || (!url.path().is_empty() && url.path() != "/")
                || !url.path().trim().is_empty()
        }
        _ => false,
    }
}

/// User-clicked external URLs are handed to the OS browser, not fetched by Synara.
/// Block local file/code schemes and embedded credentials, but allow ordinary
/// http(s) links including LAN/internal hosts users intentionally click.
pub(crate) fn is_safe_external_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }

    match url.scheme() {
        "http" | "https" => url.host_str().is_some(),
        "mailto" | "matrix" => is_safe_structured_external_url(&url),
        _ => false,
    }
}

pub(crate) fn is_safe_agent_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    url.scheme() == "https"
        && url
            .host_str()
            .map(is_safe_public_https_host)
            .unwrap_or(false)
        && url.username().is_empty()
        && url.password().is_none()
}

#[cfg(test)]
mod tests {
    use super::{is_safe_agent_url, is_safe_external_url};

    #[test]
    fn external_urls_allow_user_clicked_web_mail_and_matrix_links() {
        assert!(is_safe_external_url("https://example.org/path"));
        assert!(is_safe_external_url("http://192.168.1.10/admin"));
        assert!(is_safe_external_url("mailto:user@example.org"));
        assert!(is_safe_external_url("matrix:u/alice:example.org"));

        assert!(!is_safe_external_url("file:///etc/passwd"));
        assert!(!is_safe_external_url("javascript:alert(1)"));
        assert!(!is_safe_external_url("https://user:pass@example.org/"));
        assert!(!is_safe_external_url("mailto:not-an-email"));
        assert!(!is_safe_external_url("matrix:"));
    }

    #[test]
    fn agent_urls_require_public_https_without_credentials() {
        assert!(is_safe_agent_url("https://agent.example.org/run"));

        assert!(!is_safe_agent_url("http://agent.example.org/run"));
        assert!(!is_safe_agent_url("https://10.0.0.5/run"));
        assert!(!is_safe_agent_url("https://localhost/run"));
        assert!(!is_safe_agent_url(
            "https://user:pass@agent.example.org/run"
        ));
    }
}
