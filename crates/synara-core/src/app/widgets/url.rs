//! Widget URL policy (`is_safe_widget_url`).
//!
//! Public HTTPS is allowed for room-state and agent widgets. Loopback HTTP is
//! allowed only when `allow_loopback` is set (user-configured agent list).
//! Room-state widgets must never load loopback or LAN.

use url::Url;

const TOKEN_QUERY_KEYS: &[&str] = &["access_token", "logintoken", "accesstoken"];

fn normalize_host(host: &str) -> String {
    host.trim_matches(|character| character == '[' || character == ']')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn is_local_hostname(host: &str) -> bool {
    let host = normalize_host(host);
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
    let host = normalize_host(host);
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
    let host = normalize_host(host);
    host == "::1"
        || host == "::"
        || host.starts_with("fc")
        || host.starts_with("fd")
        || host.starts_with("fe80")
        || host.starts_with("::ffff:")
}

fn is_loopback_host(host: &str) -> bool {
    let host = normalize_host(host);
    host == "127.0.0.1" || host == "localhost" || host == "::1"
}

fn is_safe_public_https_host(host: &str) -> bool {
    let normalized = normalize_host(host);
    if normalized.is_empty() || is_local_hostname(&normalized) {
        return false;
    }
    if normalized.contains(':') {
        return !is_private_ipv6(&normalized);
    }
    !is_private_ipv4(&normalized)
}

fn has_token_query(url: &Url) -> bool {
    url.query_pairs().any(|(key, _)| {
        TOKEN_QUERY_KEYS.contains(&key.to_ascii_lowercase().as_str())
    })
}

/// Returns whether `value` may be loaded as a widget URL.
///
/// `allow_loopback` is true only for user-configured agent-list entries.
pub fn is_safe_widget_url(value: &str, allow_loopback: bool) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if has_token_query(&url) {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    match url.scheme() {
        "https" => {
            if is_loopback_host(host) {
                allow_loopback
            } else {
                is_safe_public_https_host(host)
            }
        }
        "http" => allow_loopback && is_loopback_host(host),
        _ => false,
    }
}

/// Scheme + authority used as `postMessage` `targetOrigin`.
pub fn widget_target_origin(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    let mut origin = url.clone();
    origin.set_path("");
    origin.set_query(None);
    origin.set_fragment(None);
    if let Ok(mut segments) = origin.path_segments_mut() {
        segments.clear();
    }
    Some(origin[..url::Position::BeforePath].trim_end_matches('/').to_owned())
}
