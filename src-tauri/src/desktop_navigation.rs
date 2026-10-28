//! Keep the privileged main webview on its application origin.
//!
//! Development stays on the configured dev server. Release stays on
//! `tauri://localhost` on macOS and Linux, and on `http://tauri.localhost` on
//! Windows, where WebView2 serves custom schemes over http. The `url` crate
//! treats `tauri` as a non-special scheme, so two parses of that URL do not
//! compare equal as origins; the Windows origin is an ordinary tuple origin.
use url::Url;

/// Packaged macOS and Linux assets.
#[cfg(any(not(target_os = "windows"), test))]
pub const CUSTOM_SCHEME_ASSET_ORIGIN: &str = "tauri://localhost";
/// Packaged Windows assets (Tauri's default, `useHttpsScheme` off).
#[cfg(any(target_os = "windows", test))]
pub const WINDOWS_ASSET_ORIGIN: &str = "http://tauri.localhost";

#[cfg(not(target_os = "windows"))]
pub const PACKAGED_ASSET_ORIGIN: &str = CUSTOM_SCHEME_ASSET_ORIGIN;
#[cfg(target_os = "windows")]
pub const PACKAGED_ASSET_ORIGIN: &str = WINDOWS_ASSET_ORIGIN;

pub fn is_app_navigation(app_url: &Url, target: &Url) -> bool {
    if !target.username().is_empty() || target.password().is_some() {
        return false;
    }
    if app_url.scheme() == "tauri" {
        return is_packaged_asset_origin(app_url) && is_packaged_asset_origin(target);
    }
    matches!(target.scheme(), "http" | "https") && target.origin() == app_url.origin()
}

fn is_packaged_asset_origin(url: &Url) -> bool {
    url.scheme() == "tauri"
        && url.host_str() == Some("localhost")
        && url.domain() == Some("localhost")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_routes_reload_and_fragments_stay_available_on_the_selected_port() {
        for port in [8080, 49152, 49153] {
            let app = Url::parse(&format!("http://localhost:{port}")).unwrap();
            for route in ["/", "/index.html", "/#/home", "/home/room?via=example.org"] {
                assert!(is_app_navigation(&app, &app.join(route).unwrap()));
            }
        }
    }

    #[test]
    fn packaged_asset_origin_allows_reloads_when_origins_do_not_compare_equal() {
        let app = Url::parse(CUSTOM_SCHEME_ASSET_ORIGIN).unwrap();
        let reload = Url::parse("tauri://localhost/index.html").unwrap();
        assert!(!app.origin().is_tuple());
        assert_ne!(app.origin(), reload.origin());
        for route in [
            "tauri://localhost",
            "tauri://localhost/",
            "tauri://localhost/index.html",
            "tauri://localhost/#/home",
            "tauri://localhost/index.html#/home/room",
        ] {
            assert!(
                is_app_navigation(&app, &Url::parse(route).unwrap()),
                "{route}"
            );
        }
    }

    #[test]
    fn packaged_asset_origin_rejects_other_schemes_hosts_and_ports() {
        let app = Url::parse(CUSTOM_SCHEME_ASSET_ORIGIN).unwrap();
        for target in [
            "http://localhost/",
            "http://localhost:8080/",
            "http://tauri.localhost/",
            "https://tauri.localhost/",
            "tauri://localhost:443/",
            "tauri://127.0.0.1/",
            "tauri://localhost.evil/",
            "tauri://user@localhost/",
            "tauri://localhost.example/",
            "javascript:alert(1)",
            "file:///tmp/index.html",
            "data:text/html,hello",
        ] {
            assert!(
                !is_app_navigation(&app, &Url::parse(target).unwrap()),
                "{target}"
            );
        }
    }

    #[test]
    fn windows_asset_origin_allows_its_routes_and_rejects_neighbours() {
        let app = Url::parse(WINDOWS_ASSET_ORIGIN).unwrap();
        for route in [
            "http://tauri.localhost",
            "http://tauri.localhost/",
            "http://tauri.localhost/index.html",
            "http://tauri.localhost/#/home",
            "http://tauri.localhost/index.html#/home/room",
        ] {
            assert!(
                is_app_navigation(&app, &Url::parse(route).unwrap()),
                "{route}"
            );
        }
        for target in [
            "https://tauri.localhost/",
            "http://tauri.localhost:8080/",
            "http://user@tauri.localhost/",
            "http://tauri.localhost.evil/",
            "http://localhost/",
            "http://ipc.localhost/",
            "http://synara-media.localhost/",
            "tauri://localhost/",
            "file:///C:/index.html",
        ] {
            assert!(
                !is_app_navigation(&app, &Url::parse(target).unwrap()),
                "{target}"
            );
        }
    }

    #[test]
    fn dev_server_rejects_the_packaged_asset_origin() {
        let app = Url::parse("http://localhost:8080").unwrap();
        for packaged in [CUSTOM_SCHEME_ASSET_ORIGIN, WINDOWS_ASSET_ORIGIN] {
            assert!(!is_app_navigation(&app, &Url::parse(packaged).unwrap()));
        }
        assert!(is_app_navigation(
            &app,
            &Url::parse("http://localhost:8080/#/home").unwrap()
        ));
    }

    #[test]
    fn rejects_other_origins_and_non_document_schemes() {
        let app = Url::parse("http://localhost:49152").unwrap();
        for target in [
            "http://localhost:49153/",
            "http://localhost/",
            "https://localhost:49152/",
            "http://127.0.0.1:49152/",
            "http://localhost.evil.example:49152/",
            "https://example.org/",
            "http://user:password@localhost:49152/",
            "data:text/html,hello",
            "javascript:alert(1)",
            "file:///tmp/index.html",
            "blob:http://localhost:49152/id",
            "about:blank",
        ] {
            assert!(
                !is_app_navigation(&app, &Url::parse(target).unwrap()),
                "{target}"
            );
        }
    }

    #[test]
    fn updater_install_is_macos_scoped_and_the_main_window_is_not_remote() {
        let main = include_str!("../capabilities/main.json");
        let updater = include_str!("../capabilities/macos-updater.json");
        for permission in [
            "updater:allow-check",
            "updater:allow-download-and-install",
            "updater:default",
            "process:allow-restart",
            "process:default",
        ] {
            assert!(
                !main.contains(permission),
                "main capability still grants {permission}"
            );
        }
        assert!(!main.contains("http://localhost"));
        assert!(updater.contains("\"platforms\": [\"macOS\"]"));
        assert!(updater.contains("\"windows\": [\"main\"]"));
        assert!(updater.contains("updater:allow-check"));
        assert!(updater.contains("updater:allow-download-and-install"));
        assert!(updater.contains("process:allow-restart"));
        assert!(!updater.contains("\"linux\""));
    }
}
