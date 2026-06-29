const MAX_DESKTOP_ROUTE_CHARS: usize = 2_048;

pub(crate) fn truncate_text(value: String, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(crate) fn sanitize_action_text(value: String, max_chars: usize) -> String {
    truncate_text(value.trim().to_string(), max_chars)
}

pub(crate) fn sanitize_route(route: String) -> Result<String, String> {
    let route = sanitize_action_text(route, MAX_DESKTOP_ROUTE_CHARS);
    if route.is_empty() {
        return Err("Route cannot be empty".to_owned());
    }
    if route.contains("://") {
        return Err("Route must be an internal app route".to_owned());
    }
    if !route.starts_with('/') && !route.starts_with('#') {
        return Err("Route must start with / or #".to_owned());
    }
    Ok(route)
}

pub(crate) fn sanitize_notification_route(route: String) -> Result<String, String> {
    sanitize_route(route)
}

#[cfg(test)]
mod tests {
    use super::{sanitize_action_text, sanitize_notification_route, sanitize_route, truncate_text};

    #[test]
    fn action_text_trims_and_truncates_by_chars() {
        assert_eq!(sanitize_action_text("  hello  ".to_owned(), 10), "hello");
        assert_eq!(sanitize_action_text("abcdef".to_owned(), 3), "abc");
        assert_eq!(truncate_text("abcdef".to_owned(), 4), "abcd");
    }

    #[test]
    fn routes_allow_only_internal_app_paths() {
        assert_eq!(
            sanitize_route("/inbox/later/".to_owned()).unwrap(),
            "/inbox/later/"
        );
        assert_eq!(
            sanitize_route("#/room/abc".to_owned()).unwrap(),
            "#/room/abc"
        );

        assert!(sanitize_route("https://example.org".to_owned()).is_err());
        assert!(sanitize_route("room/abc".to_owned()).is_err());
        assert!(sanitize_route("  ".to_owned()).is_err());
    }

    #[test]
    fn notification_routes_use_the_same_internal_route_policy() {
        assert_eq!(
            sanitize_notification_route("/inbox/notifications/".to_owned()).unwrap(),
            "/inbox/notifications/"
        );
        assert!(sanitize_notification_route("https://example.org".to_owned()).is_err());
    }
}
