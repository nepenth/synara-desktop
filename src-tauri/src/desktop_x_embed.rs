//! X post embeds: fetches a post's public JSON from X's embed syndication
//! endpoint for the renderer, which parses it and draws the post and its
//! video. The endpoint only allows `platform.twitter.com` as a CORS origin,
//! so the request has to come from here rather than the webview.
//!
//! Only a numeric post id and its derived token cross the bridge, so the
//! renderer cannot point this request anywhere else. The response is bounded
//! in time and size and returned as text; nothing is stored.

use std::time::Duration;

const SYNDICATION_URL: &str = "https://cdn.syndication.twimg.com/tweet-result";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_POST_ID_DIGITS: usize = 20;
const MAX_TOKEN_CHARS: usize = 32;
/// X answers a request without a User-Agent with an empty 400, so every post
/// failed to load and its card never appeared.
const USER_AGENT: &str = concat!("Synara/", env!("CARGO_PKG_VERSION"), " (X post embed)");

fn is_post_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_POST_ID_DIGITS
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TOKEN_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
}

fn syndication_url(id: &str, token: &str) -> Option<String> {
    (is_post_id(id) && is_token(token))
        .then(|| format!("{SYNDICATION_URL}?id={id}&lang=en&token={token}"))
}

fn post_request(client: &reqwest::Client, url: String) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
}

async fn fetch_post(url: String) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .ok()?;
    let mut response = post_request(&client, url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let is_json = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !is_json || response.content_length().unwrap_or(0) > MAX_BODY_BYTES as u64 {
        return None;
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return None;
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).ok()
}

/// The post's syndication JSON as text, or `None` when the id or token is
/// malformed or X does not answer with JSON in time.
#[tauri::command]
pub async fn desktop_x_post(id: String, token: String) -> Option<String> {
    fetch_post(syndication_url(&id, &token)?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_numeric_ids_and_base36_tokens_build_a_request() {
        assert_eq!(
            syndication_url("20", "6dq1a2xwd93").as_deref(),
            Some("https://cdn.syndication.twimg.com/tweet-result?id=20&lang=en&token=6dq1a2xwd93")
        );
        assert!(syndication_url("", "a").is_none());
        assert!(syndication_url("20", "").is_none());
        assert!(syndication_url("20&x=1", "a").is_none());
        assert!(syndication_url("123456789012345678901", "a").is_none());
        assert!(syndication_url("20", "A").is_none());
        assert!(syndication_url("20", "a.b").is_none());
        assert!(syndication_url("20", &"a".repeat(33)).is_none());
    }

    #[test]
    fn the_request_names_synara_as_its_user_agent() {
        let request = post_request(
            &reqwest::Client::new(),
            syndication_url("20", "6dq1a2xwd93").unwrap(),
        )
        .build()
        .unwrap();
        let user_agent = request.headers()[reqwest::header::USER_AGENT]
            .to_str()
            .unwrap();
        assert!(user_agent.starts_with("Synara/"), "{user_agent}");
        assert_eq!(
            request.headers()[reqwest::header::ACCEPT],
            "application/json"
        );
    }
}
