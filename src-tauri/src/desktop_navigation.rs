//! Keep the privileged main webview on the exact application origin.
//!
//! Packaged assets use a dynamically selected localhost port. The capability
//! pattern spans ports, so navigation must not admit another local service.
use url::Url;

pub fn is_app_navigation(app_url: &Url, target: &Url) -> bool {
    matches!(target.scheme(), "http" | "https")
        && target.username().is_empty()
        && target.password().is_none()
        && target.origin() == app_url.origin()
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
}
