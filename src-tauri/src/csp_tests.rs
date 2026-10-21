//! CSP unit tests: they assert the scoped loopback discovery sources in
//! tauri.conf.json, so this test-only file is the one place in the desktop
//! shell allowed to spell Matrix client paths (DESKTOP-REST-EXCEPTION-004).

fn csp(key: &str) -> String {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json");
    config["app"]["security"][key]
        .as_str()
        .unwrap_or_else(|| panic!("app.security.{key}"))
        .to_owned()
}

fn directive(policy: &str, name: &str) -> Vec<String> {
    policy
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(name).map(str::trim))
        .unwrap_or_else(|| panic!("{name} directive"))
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

#[test]
fn production_csp_loads_media_only_through_the_app() {
    let policy = csp("csp");
    // Images and media come from the bundle, blobs and synara-media:; the
    // only remote images are the built-in GIF providers' CDNs.
    assert_eq!(
        directive(&policy, "img-src"),
        [
            "'self'",
            "data:",
            "blob:",
            "synara-media:",
            "https://media.tenor.com",
            "https://*.giphy.com"
        ]
    );
    assert_eq!(
        directive(&policy, "media-src"),
        ["'self'", "data:", "blob:", "synara-media:"]
    );
    assert_eq!(directive(&policy, "frame-src"), ["'none'"]);
    assert_eq!(directive(&policy, "form-action"), ["'none'"]);
    assert_eq!(directive(&policy, "object-src"), ["'none'"]);
    assert_eq!(
        directive(&policy, "script-src"),
        ["'self'", "'wasm-unsafe-eval'"]
    );
}

#[test]
fn production_csp_reaches_loopback_only_for_homeserver_discovery() {
    let connect = directive(&csp("csp"), "connect-src");
    // Sign-in still discovers the homeserver from the renderer, so https:
    // stays; nothing opens WebSockets any more.
    assert!(connect.iter().any(|source| source == "https:"));
    assert!(!connect
        .iter()
        .any(|source| source.starts_with("ws") || source == "http:"));
    for source in connect
        .iter()
        .filter(|source| source.starts_with("http://"))
    {
        if source == "http://ipc.localhost" {
            continue;
        }
        assert!(
            source.ends_with("/.well-known/matrix/client")
                || source.ends_with("/_matrix/client/versions"),
            "unscoped loopback source {source}"
        );
    }
    assert!(connect
        .iter()
        .any(|source| source == "http://localhost:*/_matrix/client/versions"));
}

#[test]
fn dev_csp_alone_allows_the_vite_dev_server() {
    let dev = directive(&csp("devCsp"), "connect-src");
    assert!(dev.iter().any(|source| source == "ws://localhost:*"));
    assert!(dev.iter().any(|source| source == "http://localhost:*"));
    let production = directive(&csp("csp"), "connect-src");
    assert!(!production
        .iter()
        .any(|source| source == "http://localhost:*"));
}
