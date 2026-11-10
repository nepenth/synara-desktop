//! `synara://` links: the same grammar the iOS app accepts, mapped onto the
//! desktop router's hash routes.
//!
//! macOS delivers a link through the app's open-URL event. Linux and Windows
//! pass it as a command-line argument, to this process on a cold start or to
//! the running one through the single-instance plugin.

const SCHEME_PREFIX: &str = "synara://";
/// Longest link accepted; Matrix ids are at most 255 bytes.
const MAX_LINK_LENGTH: usize = 1024;
const MAX_ID_LENGTH: usize = 255;

const ACTIVITY_SLUGS: &[&str] = &["activity", "notifications", "notification", "inbox"];

/// Decode `%XX` escapes. `None` for a malformed escape, invalid UTF-8 or an
/// empty result.
fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = value.get(index + 1..index + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded)
        .ok()
        .filter(|decoded| !decoded.is_empty())
}

fn percent_encode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

/// A room id (`!…`) or alias (`#…`), or an event id (`$…`), with nothing a
/// route could be broken with.
fn valid_matrix_id(value: &str, sigils: &[char]) -> bool {
    value.len() > 1
        && value.len() <= MAX_ID_LENGTH
        && value.starts_with(sigils)
        && !value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || ch == '/')
}

/// The router route for a `synara://` link, or `None` for anything else.
/// Only fixed places and a room (with an optional event) are reachable: a
/// link from another app must not be able to name an arbitrary route.
pub fn route_for_deep_link(link: &str) -> Option<String> {
    let link = link.trim();
    if link.len() > MAX_LINK_LENGTH {
        return None;
    }
    let scheme = link.get(..SCHEME_PREFIX.len())?;
    if !scheme.eq_ignore_ascii_case(SCHEME_PREFIX) {
        return None;
    }
    let rest = &link[SCHEME_PREFIX.len()..];
    let rest = rest.split(['?', '#']).next().unwrap_or("");
    let mut segments = rest.split('/').filter(|segment| !segment.is_empty());
    let place = segments.next()?.to_ascii_lowercase();

    match place.as_str() {
        "home" => Some("/home/".to_owned()),
        "settings" => Some("/settings/".to_owned()),
        "approvals" => Some("/approvals/".to_owned()),
        "later" => Some("/inbox/later/".to_owned()),
        slug if ACTIVITY_SLUGS.contains(&slug) => {
            match segments.next().map(str::to_ascii_lowercase).as_deref() {
                Some("later") => Some("/inbox/later/".to_owned()),
                Some("invites") => Some("/inbox/invites/".to_owned()),
                _ => Some("/inbox/notifications/".to_owned()),
            }
        }
        "room" => {
            let room = percent_decode(segments.next()?)?;
            if !valid_matrix_id(&room, &['!', '#']) {
                return None;
            }
            let mut route = format!("/home/{}/", percent_encode(&room));
            if let Some(event) = segments.next().and_then(percent_decode) {
                if !valid_matrix_id(&event, &['$']) {
                    return None;
                }
                route.push_str(&percent_encode(&event));
                route.push('/');
            }
            Some(route)
        }
        _ => None,
    }
}

/// The first `synara://` link among command-line arguments, as its route.
pub fn route_from_args<S: AsRef<str>>(args: &[S]) -> Option<String> {
    args.iter()
        .map(AsRef::as_ref)
        .find(|arg| {
            arg.get(..SCHEME_PREFIX.len())
                .is_some_and(|scheme| scheme.eq_ignore_ascii_case(SCHEME_PREFIX))
        })
        .and_then(route_for_deep_link)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_places_map_to_their_routes() {
        for (link, route) in [
            ("synara://home", "/home/"),
            ("synara://settings", "/settings/"),
            ("SYNARA://Settings/", "/settings/"),
            ("synara://approvals", "/approvals/"),
            ("synara://later", "/inbox/later/"),
            ("synara://activity", "/inbox/notifications/"),
            ("synara://notifications", "/inbox/notifications/"),
            ("synara://notification", "/inbox/notifications/"),
            ("synara://inbox", "/inbox/notifications/"),
            ("synara://inbox/later", "/inbox/later/"),
            ("synara://inbox/invites/", "/inbox/invites/"),
            ("synara://activity?source=bar", "/inbox/notifications/"),
        ] {
            assert_eq!(route_for_deep_link(link).as_deref(), Some(route), "{link}");
        }
    }

    #[test]
    fn a_room_link_keeps_the_room_and_optional_event() {
        assert_eq!(
            route_for_deep_link("synara://room/!abc:example.org").as_deref(),
            Some("/home/%21abc%3Aexample.org/")
        );
        assert_eq!(
            route_for_deep_link("synara://room/%21abc%3Aexample.org/%24event").as_deref(),
            Some("/home/%21abc%3Aexample.org/%24event/")
        );
        assert_eq!(
            route_for_deep_link("synara://room/%23general%3Aexample.org").as_deref(),
            Some("/home/%23general%3Aexample.org/")
        );
        assert_eq!(
            route_for_deep_link("synara://room/!abc:example.org/$a=b&c").as_deref(),
            Some("/home/%21abc%3Aexample.org/%24a%3Db%26c/")
        );
        // A plus sign in an id is a plus sign, not a space.
        assert_eq!(
            route_for_deep_link("synara://room/!a+b:example.org").as_deref(),
            Some("/home/%21a%2Bb%3Aexample.org/")
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for link in [
            "",
            "synara://",
            "synara://route/settings/devices",
            "synara://room",
            "synara://room/not-a-room",
            "synara://room/!abc:example.org/not-an-event",
            "synara://room/%21abc%2F..%2Fsettings",
            "synara://room/!a%20b:example.org",
            "synara://room/!a%0Ab:example.org",
            "synara://room/!abc%2",
            "synara://room/!abc%zz:example.org",
            "synara://room/!abc%ff:example.org",
            "https://synara.app/room/!abc:example.org",
            "matrix:r/general:example.org",
            "synara:/settings",
        ] {
            assert_eq!(route_for_deep_link(link), None, "{link}");
        }
        let long = format!("synara://room/!{}:example.org", "a".repeat(2000));
        assert_eq!(route_for_deep_link(&long), None);
    }

    #[test]
    fn the_link_is_found_among_other_arguments() {
        assert_eq!(
            route_from_args(&["--flag", "synara://later"]).as_deref(),
            Some("/inbox/later/")
        );
        assert_eq!(route_from_args(&["/some/file.txt"]), None);
        assert_eq!(route_from_args::<&str>(&[]), None);
        // The first link decides, even when it is not one we accept.
        assert_eq!(
            route_from_args(&["synara://nope", "synara://settings"]),
            None
        );
    }
}
