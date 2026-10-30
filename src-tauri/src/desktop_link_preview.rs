//! On-device link previews: fetches the linked page from this device and
//! reads its title, description, site name and image from the `<head>`, the
//! way Signal or Telegram do. Nothing goes through the homeserver, so previews
//! work in encrypted rooms too; the site sees this device's IP address, which
//! the Link previews setting says.
//!
//! The renderer hands over any URL from a message, so every request is
//! treated as hostile (SSRF):
//! - only `http` and `https`, without credentials;
//! - a literal IP in the URL, and every address a host name resolves to, must
//!   be public (no loopback, private, link-local, unique-local, CGNAT,
//!   multicast, unspecified, documentation or metadata addresses, including
//!   IPv4 embedded in IPv6). The client resolves names only through
//!   [`GuardedResolver`], so the connection goes to the addresses that were
//!   checked and DNS rebinding cannot swap them; the connected peer is checked
//!   again after the response;
//! - no proxy (a proxy would resolve the name itself);
//! - redirects are followed by hand, at most [`MAX_REDIRECTS`], each target
//!   checked like the first URL;
//! - an 8 s timeout, a 1 MiB HTML cap read in chunks that stops at `</head>`,
//!   `text/html` only; the image is capped at 5 MB, must say `image/*` and
//!   must sniff as PNG, JPEG, GIF, WebP or AVIF (never SVG).
//!
//! The image is kept in memory and served to the webview through the
//! `synara-media:` protocol at `link-preview/<handle>`, like other downloaded
//! media, so the CSP `img-src` stays closed to the web. Results are cached per
//! URL for the session; failures for a few minutes.

use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use regex::Regex;
use serde::Serialize;
use url::{Host, Url};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REDIRECTS: usize = 3;
const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_URL_CHARS: usize = 2048;
const MAX_TITLE_CHARS: usize = 300;
const MAX_DESCRIPTION_CHARS: usize = 500;
const MAX_SITE_NAME_CHARS: usize = 100;
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_CACHE_ENTRIES: usize = 512;
const MAX_STORED_IMAGES: usize = 64;
const MAX_STORED_IMAGE_BYTES: usize = 48 * 1024 * 1024;

/// Path prefix of preview images on the `synara-media:` protocol.
pub const IMAGE_PATH_PREFIX: &str = "link-preview/";

/// What the renderer draws. `image_handle_id` names an image served at
/// `synara-media:` `link-preview/<id>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPreview {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_handle_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Address policy
// ---------------------------------------------------------------------------

/// Whether this device may connect to `ip` for a preview: only globally
/// routable unicast addresses.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_ipv4(v4),
        IpAddr::V6(v6) => is_public_ipv6(v6),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0 // "this network", including 0.0.0.0
        || a == 10
        || a == 127
        || (a == 100 && (64..128).contains(&b)) // CGNAT 100.64.0.0/10
        || (a == 169 && b == 254) // link-local, including 169.254.169.254
        || (a == 172 && (16..32).contains(&b))
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments
        || (a == 192 && b == 0 && c == 2) // TEST-NET-1
        || (a == 192 && b == 88 && c == 99) // 6to4 relay anycast
        || (a == 192 && b == 168)
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || (a == 198 && b == 51 && c == 100) // TEST-NET-2
        || (a == 203 && b == 0 && c == 113) // TEST-NET-3
        || a >= 224) // multicast, reserved and broadcast
}

fn embedded_ipv4(ip: Ipv6Addr, from_segment: usize) -> Ipv4Addr {
    let s = ip.segments();
    let [h, l] = [s[from_segment], s[from_segment + 1]];
    Ipv4Addr::new((h >> 8) as u8, h as u8, (l >> 8) as u8, l as u8)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    // IPv4-mapped ::ffff:a.b.c.d and IPv4-compatible ::a.b.c.d (including
    // :: and ::1) carry an IPv4 address that the stack may connect to.
    if s[..5] == [0; 5] && (s[5] == 0xffff || s[5] == 0) {
        return s[5] == 0xffff && is_public_ipv4(embedded_ipv4(ip, 6));
    }
    // NAT64 64:ff9b::/96 reaches the embedded IPv4 address.
    if s[..6] == [0x64, 0xff9b, 0, 0, 0, 0] {
        return is_public_ipv4(embedded_ipv4(ip, 6));
    }
    // 6to4 2002::/16 embeds the IPv4 address in bits 16..48.
    if s[0] == 0x2002 {
        return is_public_ipv4(embedded_ipv4(ip, 1));
    }
    !((s[0] & 0xfe00) == 0xfc00 // unique local fc00::/7 (fd00:ec2::254 metadata)
        || (s[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        || (s[0] & 0xffc0) == 0xfec0 // site-local fec0::/10
        || (s[0] & 0xff00) == 0xff00 // multicast
        || (s[0] == 0x2001 && s[1] == 0x0db8) // documentation
        || (s[0] == 0x2001 && s[1] == 0) // Teredo
        || s[..4] == [0x100, 0, 0, 0] // discard 100::/64
        || s[..2] == [0x64, 0xff9b]) // local-use NAT64 64:ff9b:1::/48
}

type AddressPolicy = fn(IpAddr) -> bool;

/// Resolves names for the preview client and refuses the whole name when any
/// address is not allowed, so a mixed answer cannot be raced. reqwest
/// connects only to the addresses this returns.
struct GuardedResolver {
    allow: AddressPolicy,
}

impl reqwest::dns::Resolve for GuardedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let allow = self.allow;
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            if addrs.is_empty() || addrs.iter().any(|addr| !allow(addr.ip())) {
                return Err("link preview host resolves to a non-public address".into());
            }
            Ok(Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

/// A URL this fetcher may request: http(s), no credentials, a host, and a
/// literal IP only when it is allowed.
fn checked_url(url: &Url, allow: AddressPolicy) -> bool {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.as_str().len() > MAX_URL_CHARS
    {
        return false;
    }
    match url.host() {
        Some(Host::Ipv4(ip)) => allow(IpAddr::V4(ip)),
        Some(Host::Ipv6(ip)) => allow(IpAddr::V6(ip)),
        Some(Host::Domain(domain)) => !domain.is_empty(),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// Fetching
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
enum FetchError {
    Refused,
    Failed,
    TooLarge,
    WrongType,
}

struct Fetched {
    url: Url,
    body: Vec<u8>,
}

fn user_agent() -> String {
    format!("Synara/{} (link preview)", env!("CARGO_PKG_VERSION"))
}

fn is_html(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("text/html"))
}

fn is_image(content_type: &str) -> bool {
    content_type
        .split(';')
        .next()
        .is_some_and(|mime| mime.trim().to_ascii_lowercase().starts_with("image/"))
}

/// Where the HTML head ends, if it has: `</head>` or the start of `<body`.
fn head_end(body: &[u8]) -> Option<usize> {
    let lower = body.to_ascii_lowercase();
    let find = |needle: &[u8]| lower.windows(needle.len()).position(|w| w == needle);
    match (find(b"</head"), find(b"<body")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

pub struct LinkPreviewer {
    client: reqwest::Client,
    allow: AddressPolicy,
    cache: Mutex<HashMap<String, (Instant, Option<LinkPreview>)>>,
}

impl LinkPreviewer {
    fn new(allow: AddressPolicy) -> Option<Self> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .dns_resolver(GuardedResolver { allow })
            .user_agent(user_agent())
            .build()
            .ok()?;
        Some(Self {
            client,
            allow,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// GETs `url`, following up to [`MAX_REDIRECTS`] checked redirects, and
    /// reads at most `max_bytes` of a body whose content type passes
    /// `accept`. `stop_at_head` ends the read once the HTML head is complete.
    async fn get(
        &self,
        url: Url,
        accept: &str,
        type_ok: fn(&str) -> bool,
        max_bytes: usize,
        stop_at_head: bool,
    ) -> Result<Fetched, FetchError> {
        let mut url = url;
        let mut redirects = 0;
        loop {
            if !checked_url(&url, self.allow) {
                return Err(FetchError::Refused);
            }
            let mut response = self
                .client
                .get(url.clone())
                .header(reqwest::header::ACCEPT, accept)
                .send()
                .await
                .map_err(|_| FetchError::Failed)?;
            if response
                .remote_addr()
                .is_some_and(|addr| !(self.allow)(addr.ip()))
            {
                return Err(FetchError::Refused);
            }
            let status = response.status();
            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|value| value.to_str().ok())
                    .ok_or(FetchError::Failed)?;
                let next = url.join(location).map_err(|_| FetchError::Failed)?;
                redirects += 1;
                if redirects > MAX_REDIRECTS {
                    return Err(FetchError::Refused);
                }
                url = next;
                continue;
            }
            if !status.is_success() {
                return Err(FetchError::Failed);
            }
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("");
            if !type_ok(content_type) {
                return Err(FetchError::WrongType);
            }
            if response.content_length().unwrap_or(0) > max_bytes as u64 {
                return Err(FetchError::TooLarge);
            }
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| FetchError::Failed)? {
                if body.len() + chunk.len() > max_bytes {
                    return Err(FetchError::TooLarge);
                }
                body.extend_from_slice(&chunk);
                if stop_at_head {
                    if let Some(end) = head_end(&body) {
                        body.truncate(end);
                        break;
                    }
                }
            }
            return Ok(Fetched { url, body });
        }
    }

    async fn fetch_image(&self, url: Url) -> Option<String> {
        let fetched = self
            .get(url, "image/*", is_image, MAX_IMAGE_BYTES, false)
            .await
            .ok()?;
        let content_type = crate::image_content_type(&fetched.body)?;
        Some(store_image(fetched.body, content_type))
    }

    async fn fetch_preview(&self, requested: &str) -> Option<LinkPreview> {
        let url = Url::parse(requested).ok()?;
        let page = self
            .get(
                url,
                "text/html,application/xhtml+xml;q=0.9",
                is_html,
                MAX_HTML_BYTES,
                true,
            )
            .await
            .ok()?;
        let head = String::from_utf8_lossy(&page.body);
        let meta = parse_head(&head);
        let title = meta.title?;
        let image_handle_id = match meta
            .image
            .and_then(|image| page.url.join(&image).ok())
            .filter(|image| matches!(image.scheme(), "http" | "https"))
        {
            Some(image) => self.fetch_image(image).await,
            None => None,
        };
        Some(LinkPreview {
            url: requested.to_owned(),
            title,
            description: meta.description,
            site_name: meta.site_name,
            image_handle_id,
        })
    }

    fn cached(&self, url: &str) -> Option<Option<LinkPreview>> {
        let cache = self.cache.lock().ok()?;
        let (at, preview) = cache.get(url)?;
        (preview.is_some() || at.elapsed() < NEGATIVE_CACHE_TTL).then(|| preview.clone())
    }

    fn remember(&self, url: &str, preview: Option<LinkPreview>) {
        let Ok(mut cache) = self.cache.lock() else {
            return;
        };
        if cache.len() >= MAX_CACHE_ENTRIES {
            cache.retain(|_, (at, preview)| preview.is_some() || at.elapsed() < NEGATIVE_CACHE_TTL);
            if cache.len() >= MAX_CACHE_ENTRIES {
                cache.clear();
            }
        }
        cache.insert(url.to_owned(), (Instant::now(), preview));
    }

    /// The preview for `url`, from the session cache when it has one.
    pub async fn preview(&self, url: &str) -> Option<LinkPreview> {
        if url.len() > MAX_URL_CHARS {
            return None;
        }
        if let Some(cached) = self.cached(url) {
            return cached;
        }
        let preview = self.fetch_preview(url).await;
        self.remember(url, preview.clone());
        preview
    }
}

fn previewer() -> Option<&'static LinkPreviewer> {
    static PREVIEWER: OnceLock<Option<LinkPreviewer>> = OnceLock::new();
    PREVIEWER
        .get_or_init(|| LinkPreviewer::new(is_public_ip))
        .as_ref()
}

/// The page's preview, or `None` when the URL is refused, the page does not
/// answer with HTML in time, or it has no title.
#[tauri::command]
pub async fn desktop_link_preview(url: String) -> Option<LinkPreview> {
    previewer()?.preview(&url).await
}

// ---------------------------------------------------------------------------
// Images served to the webview
// ---------------------------------------------------------------------------

type StoredImage = (String, Arc<Vec<u8>>, &'static str);

fn image_store() -> &'static Mutex<VecDeque<StoredImage>> {
    static STORE: OnceLock<Mutex<VecDeque<StoredImage>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn store_image(bytes: Vec<u8>, content_type: &'static str) -> String {
    let mut id = [0u8; 32];
    getrandom::fill(&mut id).expect("cryptographic random bytes for a link preview image");
    let handle: String = id.iter().map(|byte| format!("{byte:02x}")).collect();
    if let Ok(mut store) = image_store().lock() {
        store.push_back((handle.clone(), Arc::new(bytes), content_type));
        while store.len() > MAX_STORED_IMAGES
            || store.iter().map(|(_, bytes, _)| bytes.len()).sum::<usize>() > MAX_STORED_IMAGE_BYTES
        {
            store.pop_front();
        }
    }
    handle
}

/// A stored preview image by handle, for the `synara-media:` protocol.
pub fn stored_image(handle: &str) -> Option<(Arc<Vec<u8>>, &'static str)> {
    if handle.len() != 64 || !handle.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let store = image_store().lock().ok()?;
    store
        .iter()
        .find(|(id, _, _)| id == handle)
        .map(|(_, bytes, content_type)| (bytes.clone(), *content_type))
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Default, PartialEq, Eq)]
struct HeadMeta {
    title: Option<String>,
    description: Option<String>,
    site_name: Option<String>,
    image: Option<String>,
}

fn regexes() -> &'static (Regex, Regex, Regex, Regex) {
    static RE: OnceLock<(Regex, Regex, Regex, Regex)> = OnceLock::new();
    RE.get_or_init(|| {
        (
            Regex::new(r"(?is)<!--.*?-->|<script\b.*?</script\s*>|<style\b.*?</style\s*>")
                .expect("valid ignore pattern"),
            Regex::new(r"(?is)<meta\b([^>]*)>").expect("valid meta pattern"),
            Regex::new(r#"(?is)([a-z_:.-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'>]+))"#)
                .expect("valid attribute pattern"),
            Regex::new(r"(?is)<title\b[^>]*>(.*?)</title\s*>").expect("valid title pattern"),
        )
    })
}

/// Title, description, site name and image from an HTML head: Open Graph
/// first, then Twitter cards, then `<title>` and `description`.
fn parse_head(html: &str) -> HeadMeta {
    let (ignore, meta_re, attr_re, title_re) = regexes();
    let head = match head_end(html.as_bytes()) {
        Some(end) => &html[..end],
        None => html,
    };
    let head = ignore.replace_all(head, " ");
    let mut tags: HashMap<String, String> = HashMap::new();
    for meta in meta_re.captures_iter(&head) {
        let mut key = None;
        let mut content = None;
        for attr in attr_re.captures_iter(&meta[1]) {
            let value = attr
                .get(2)
                .or_else(|| attr.get(3))
                .or_else(|| attr.get(4))
                .map_or("", |m| m.as_str());
            match attr[1].to_ascii_lowercase().as_str() {
                "property" | "name" => key = Some(value.trim().to_ascii_lowercase()),
                "content" => content = Some(value),
                _ => {}
            }
        }
        if let (Some(key), Some(content)) = (key, content) {
            if !content.trim().is_empty() {
                tags.entry(key).or_insert_with(|| content.to_owned());
            }
        }
    }
    let pick = |keys: &[&str], max: usize| {
        keys.iter()
            .filter_map(|key| tags.get(*key))
            .map(|value| clean_text(value, max))
            .find(|value| !value.is_empty())
    };
    let title = pick(&["og:title", "twitter:title"], MAX_TITLE_CHARS).or_else(|| {
        title_re
            .captures(&head)
            .map(|captures| clean_text(&captures[1], MAX_TITLE_CHARS))
            .filter(|title| !title.is_empty())
    });
    let image = [
        "og:image:secure_url",
        "og:image",
        "og:image:url",
        "twitter:image",
        "twitter:image:src",
    ]
    .iter()
    .filter_map(|key| tags.get(*key))
    .map(|value| decode_entities(value).trim().to_owned())
    .find(|value| !value.is_empty() && value.len() <= MAX_URL_CHARS);
    HeadMeta {
        title,
        description: pick(
            &["og:description", "twitter:description", "description"],
            MAX_DESCRIPTION_CHARS,
        ),
        site_name: pick(&["og:site_name", "application-name"], MAX_SITE_NAME_CHARS),
        image,
    }
}

/// Decoded, whitespace-collapsed text of at most `max` characters, ending in
/// an ellipsis when cut.
fn clean_text(raw: &str, max: usize) -> String {
    let decoded = decode_entities(raw);
    let collapsed = decoded.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        return collapsed;
    }
    let mut cut: String = collapsed.chars().take(max - 1).collect();
    cut.truncate(cut.trim_end().len());
    cut.push('…');
    cut
}

fn named_entity(name: &str) -> Option<char> {
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "ndash" => '–',
        "mdash" => '—',
        "hellip" => '…',
        "lsquo" => '‘',
        "rsquo" => '’',
        "ldquo" => '“',
        "rdquo" => '”',
        "laquo" => '«',
        "raquo" => '»',
        "middot" => '·',
        "bull" => '•',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "euro" => '€',
        "pound" => '£',
        "deg" => '°',
        _ => return None,
    })
}

/// Decodes numeric and common named HTML character references; anything
/// unknown stays as written.
fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let decoded = rest[1..]
            .find(';')
            .filter(|end| *end <= 10)
            .and_then(|end| {
                let name = &rest[1..=end];
                let ch = if let Some(hex) =
                    name.strip_prefix("#x").or_else(|| name.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = name.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32)
                } else {
                    named_entity(name)
                };
                ch.filter(|ch| *ch != '\0').map(|ch| (ch, end + 2))
            });
        match decoded {
            Some((ch, len)) => {
                out.push(ch);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests;
