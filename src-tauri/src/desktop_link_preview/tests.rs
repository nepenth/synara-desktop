use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Tests serve pages from 127.0.0.1, so they allow exactly that address on
/// top of the production policy.
fn allow_test_loopback(ip: IpAddr) -> bool {
    ip == IpAddr::V4(Ipv4Addr::LOCALHOST) || is_public_ip(ip)
}

fn test_previewer() -> LinkPreviewer {
    LinkPreviewer::new(allow_test_loopback).expect("test client builds")
}

fn ip(value: &str) -> IpAddr {
    value.parse().expect("test address parses")
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

fn html(body: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_raw(body.as_bytes().to_vec(), "text/html; charset=utf-8")
}

fn redirect(to: &str) -> ResponseTemplate {
    ResponseTemplate::new(302).insert_header("location", to)
}

const PAGE: &str = r#"<!doctype html><html><head>
<title>Fallback title</title>
<meta property="og:title" content="Rust &amp; You &#8212; a &quot;guide&quot;">
<meta property="og:description" content="  Learn   it&#x21; ">
<meta property="og:site_name" content="Example">
<meta property="og:image" content="/img.png">
</head><body><meta property="og:title" content="from the body"></body></html>"#;

#[test]
fn private_and_special_addresses_are_refused() {
    for blocked in [
        "0.0.0.0",
        "127.0.0.1",
        "127.255.255.254",
        "10.1.2.3",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "169.254.169.254",
        "169.254.1.1",
        "100.64.0.1",
        "100.127.255.255",
        "224.0.0.1",
        "239.255.255.250",
        "255.255.255.255",
        "240.0.0.1",
        "192.0.2.1",
        "198.18.0.1",
        "::",
        "::1",
        "fc00::1",
        "fd00:ec2::254",
        "fe80::1",
        "fec0::1",
        "ff02::1",
        "2001:db8::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:169.254.169.254",
        "::ffff:192.168.0.1",
        "::127.0.0.1",
        "64:ff9b::a00:1",
        "2002:7f00:1::",
        "2002:c0a8:101::1",
        "2001::1",
    ] {
        assert!(!is_public_ip(ip(blocked)), "{blocked} should be refused");
    }
    for allowed in [
        "1.1.1.1",
        "8.8.8.8",
        "93.184.215.14",
        "100.63.255.255",
        "100.128.0.1",
        "172.15.255.255",
        "172.32.0.1",
        "2606:4700:4700::1111",
        "2a00:1450:4009:81f::200e",
        "::ffff:8.8.8.8",
        "64:ff9b::808:808",
        "2002:808:808::1",
    ] {
        assert!(is_public_ip(ip(allowed)), "{allowed} should be allowed");
    }
}

#[test]
fn only_http_urls_without_credentials_or_private_literals_pass() {
    let ok = |value: &str| checked_url(&Url::parse(value).unwrap(), is_public_ip);
    assert!(ok("https://example.org/a?b=c"));
    assert!(ok("http://example.org/"));
    assert!(ok("https://8.8.8.8/"));
    assert!(ok("https://[2606:4700:4700::1111]/"));
    for refused in [
        "file:///etc/passwd",
        "data:text/html,<title>x</title>",
        "ftp://example.org/",
        "javascript:alert(1)",
        "https://user:pass@example.org/",
        "https://user@example.org/",
        "http://127.0.0.1/",
        "http://0x7f.1/",
        "http://2130706433/",
        "http://10.0.0.1:8080/",
        "http://169.254.169.254/latest/meta-data/",
        "http://[::1]/",
        "http://[::ffff:127.0.0.1]/",
        "http://[fd00::1]/",
        "http://[fe80::1]/",
    ] {
        assert!(!ok(refused), "{refused} should be refused");
    }
    assert!(!ok(&format!(
        "https://example.org/{}",
        "a".repeat(MAX_URL_CHARS)
    )));
}

#[tokio::test]
async fn the_resolver_refuses_names_that_resolve_to_loopback() {
    use reqwest::dns::Resolve;
    let resolver = GuardedResolver {
        allow: is_public_ip,
    };
    let name: reqwest::dns::Name = "localhost".parse().unwrap();
    assert!(resolver.resolve(name).await.is_err());
    let permissive = GuardedResolver { allow: |_| true };
    let name: reqwest::dns::Name = "localhost".parse().unwrap();
    let addrs: Vec<_> = permissive.resolve(name).await.unwrap().collect();
    assert!(addrs.iter().all(|addr| addr.ip().is_loopback()));
}

#[tokio::test]
async fn the_production_policy_never_connects_to_a_local_server() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(html(PAGE))
        .expect(0)
        .mount(&server)
        .await;
    let previewer = LinkPreviewer::new(is_public_ip).unwrap();
    // A literal loopback address and a name that resolves to it.
    assert_eq!(previewer.preview(&format!("{}/", server.uri())).await, None);
    let port = server.address().port();
    assert_eq!(
        previewer
            .preview(&format!("http://localhost:{port}/"))
            .await,
        None
    );
}

#[tokio::test]
async fn reads_open_graph_from_the_head_and_serves_the_image() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(html(PAGE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/img.png"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(PNG.to_vec(), "image/png"))
        .mount(&server)
        .await;
    let previewer = test_previewer();
    let url = format!("{}/page", server.uri());
    let preview = previewer.preview(&url).await.expect("preview");
    assert_eq!(preview.url, url);
    assert_eq!(preview.title, "Rust & You — a \"guide\"");
    assert_eq!(preview.description.as_deref(), Some("Learn it!"));
    assert_eq!(preview.site_name.as_deref(), Some("Example"));
    let handle = preview.image_handle_id.clone().expect("image handle");
    let (bytes, content_type) = stored_image(&handle).expect("stored image");
    assert_eq!(bytes.as_slice(), PNG);
    assert_eq!(content_type, "image/png");
    assert!(stored_image("../etc").is_none());

    // Cached for the session: the page is requested once.
    assert_eq!(previewer.preview(&url).await, Some(preview));
    let requests = server.received_requests().await.unwrap();
    let page = requests.iter().find(|r| r.url.path() == "/page").unwrap();
    assert_eq!(
        page.headers.get("user-agent").unwrap().to_str().unwrap(),
        format!("Synara/{} (link preview)", env!("CARGO_PKG_VERSION"))
    );
}

#[tokio::test]
async fn failures_are_cached_for_a_while() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let previewer = test_previewer();
    let url = format!("{}/missing", server.uri());
    assert_eq!(previewer.preview(&url).await, None);
    assert_eq!(previewer.preview(&url).await, None);
}

#[tokio::test]
async fn redirects_are_followed_up_to_three_times_and_rechecked() {
    let server = MockServer::start().await;
    let base = server.uri();
    for (from, to) in [
        ("/r1", "/r2"),
        ("/r2", "/r3"),
        ("/r3", "/page"),
        ("/r0", "/r1"),
    ] {
        Mock::given(method("GET"))
            .and(path(from))
            .respond_with(redirect(&format!("{base}{to}")))
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(html("<head><title>Landed</title></head>"))
        .mount(&server)
        .await;
    for (target, private) in [
        ("/to-private", "http://10.0.0.1/"),
        ("/to-metadata", "http://169.254.169.254/latest/meta-data/"),
        ("/to-mapped", "http://[::ffff:127.0.0.2]/"),
        ("/to-ula", "http://[fd00::1]/"),
        ("/to-file", "file:///etc/passwd"),
    ] {
        Mock::given(method("GET"))
            .and(path(target))
            .respond_with(redirect(private))
            .mount(&server)
            .await;
    }
    let previewer = test_previewer();
    let three = previewer.preview(&format!("{base}/r1")).await.unwrap();
    assert_eq!(three.title, "Landed");
    assert_eq!(previewer.preview(&format!("{base}/r0")).await, None);
    for target in [
        "/to-private",
        "/to-metadata",
        "/to-mapped",
        "/to-ula",
        "/to-file",
    ] {
        let refused = previewer
            .get(
                Url::parse(&format!("{base}{target}")).unwrap(),
                "text/html",
                is_html,
                MAX_HTML_BYTES,
                true,
            )
            .await;
        assert_eq!(refused.err(), Some(FetchError::Refused), "{target}");
    }
}

#[tokio::test]
async fn non_html_pages_are_refused() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"<title>json</title>".to_vec(), "application/json"),
        )
        .mount(&server)
        .await;
    let previewer = test_previewer();
    let result = previewer
        .get(
            Url::parse(&server.uri()).unwrap(),
            "text/html",
            is_html,
            MAX_HTML_BYTES,
            true,
        )
        .await;
    assert_eq!(result.err(), Some(FetchError::WrongType));
    assert_eq!(previewer.preview(&server.uri()).await, None);
}

#[tokio::test]
async fn an_html_body_over_one_mebibyte_is_refused() {
    let server = MockServer::start().await;
    let big = format!("<head><title>Big</title>{}", "x".repeat(MAX_HTML_BYTES + 1));
    Mock::given(method("GET"))
        .respond_with(html(&big))
        .mount(&server)
        .await;
    let previewer = test_previewer();
    assert_eq!(previewer.preview(&server.uri()).await, None);
}

/// Serves `head` and then an endless stream of filler without a length, and
/// counts the bytes it managed to send before the client hung up.
async fn endless_html_server(head: &'static str) -> (String, tokio::task::JoinHandle<usize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        let _ = socket.read(&mut request).await;
        let header = "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\nconnection: close\r\n\r\n";
        let mut sent = 0;
        if socket.write_all(header.as_bytes()).await.is_err()
            || socket.write_all(head.as_bytes()).await.is_err()
        {
            return sent;
        }
        let filler = vec![b'x'; 64 * 1024];
        while sent < 64 * 1024 * 1024 {
            if socket.write_all(&filler).await.is_err() {
                break;
            }
            sent += filler.len();
        }
        sent
    });
    (format!("http://{addr}/"), task)
}

#[tokio::test]
async fn a_streamed_body_stops_at_the_cap_or_at_the_end_of_the_head() {
    let (url, server) = endless_html_server("<head><title>Endless</title>").await;
    let previewer = test_previewer();
    assert_eq!(previewer.preview(&url).await, None);
    assert!(
        server.await.unwrap() < 16 * 1024 * 1024,
        "read stopped early"
    );

    let (url, server) = endless_html_server("<head><title>Short head</title></head>").await;
    let preview = previewer.preview(&url).await.unwrap();
    assert_eq!(preview.title, "Short head");
    assert!(
        server.await.unwrap() < 16 * 1024 * 1024,
        "read stopped at </head>"
    );
}

#[tokio::test]
async fn images_must_be_small_raster_images() {
    let server = MockServer::start().await;
    let base = server.uri();
    let page = |image: &str| {
        html(&format!(
            r#"<head><title>T</title><meta property="og:image" content="{image}"></head>"#
        ))
    };
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(page("/big.png"))
        .mount(&server)
        .await;
    let mut big = PNG.to_vec();
    big.resize(MAX_IMAGE_BYTES + 1, 0);
    Mock::given(method("GET"))
        .and(path("/big.png"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(big, "image/png"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/svg"))
        .respond_with(page("/a.svg"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/a.svg"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(b"<svg onload=alert(1)/>".to_vec(), "image/svg+xml"),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/html-image"))
        .respond_with(page("/not-an-image"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/not-an-image"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(PNG.to_vec(), "text/html"))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/private-image"))
        .respond_with(page("http://192.168.1.1/a.png"))
        .mount(&server)
        .await;
    let previewer = test_previewer();
    for page in ["/big", "/svg", "/html-image", "/private-image"] {
        let preview = previewer.preview(&format!("{base}{page}")).await.unwrap();
        assert_eq!(preview.title, "T", "{page}");
        assert_eq!(preview.image_handle_id, None, "{page}");
    }
    let image = previewer
        .get(
            Url::parse(&format!("{base}/big.png")).unwrap(),
            "image/*",
            is_image,
            MAX_IMAGE_BYTES,
            false,
        )
        .await;
    assert_eq!(image.err(), Some(FetchError::TooLarge));
}

#[test]
fn parsing_falls_back_from_open_graph_to_twitter_to_title() {
    let twitter = parse_head(
        r#"<head><title>Doc</title>
        <meta name="twitter:title" content='Bird &lt;title&gt;'>
        <meta name="twitter:description" content="Tweet">
        <meta name="twitter:image" content="https://example.org/t.jpg">
        <meta name="description" content="Plain"></head>"#,
    );
    assert_eq!(twitter.title.as_deref(), Some("Bird <title>"));
    assert_eq!(twitter.description.as_deref(), Some("Tweet"));
    assert_eq!(twitter.image.as_deref(), Some("https://example.org/t.jpg"));

    let plain = parse_head(
        "<HTML><HEAD><TITLE>\n  Only &#169; a   title\n</TITLE>\
         <META NAME=description CONTENT=\"Plain &amp; simple\"></HEAD>",
    );
    assert_eq!(plain.title.as_deref(), Some("Only © a title"));
    assert_eq!(plain.description.as_deref(), Some("Plain & simple"));
    assert_eq!(plain.site_name, None);
    assert_eq!(plain.image, None);

    assert_eq!(
        parse_head("<head></head><body><title>x</title></body>").title,
        None
    );
}

#[test]
fn parsing_ignores_the_body_scripts_and_comments() {
    let meta = parse_head(
        r#"<head><!-- <meta property="og:title" content="comment"> -->
        <script>document.write('<meta property="og:title" content="script">')</script>
        <meta property="og:title" content="">
        <meta property="og:title" content="Real">
        </head><body><meta property="og:description" content="body"></body>"#,
    );
    assert_eq!(meta.title.as_deref(), Some("Real"));
    assert_eq!(meta.description, None);
}

#[test]
fn text_is_decoded_collapsed_and_capped() {
    assert_eq!(
        decode_entities("a &amp; b &lt;&gt; &#39;&#x41;&#X42; &bogus; & &#0;"),
        "a & b <> 'AB &bogus; & &#0;"
    );
    assert_eq!(clean_text(" a \n\t b ", 10), "a b");
    let long = clean_text(&"word ".repeat(200), MAX_TITLE_CHARS);
    assert_eq!(long.chars().count(), MAX_TITLE_CHARS);
    assert!(long.ends_with('…'));
    let accented = clean_text(&"é".repeat(600), MAX_DESCRIPTION_CHARS);
    assert_eq!(accented.chars().count(), MAX_DESCRIPTION_CHARS);
}

#[test]
fn only_text_html_and_images_are_accepted() {
    assert!(is_html("text/html"));
    assert!(is_html("Text/HTML; charset=utf-8"));
    assert!(!is_html("application/json"));
    assert!(!is_html("text/plain"));
    assert!(!is_html(""));
    assert!(is_image("image/jpeg"));
    assert!(!is_image("text/html"));
}

#[test]
fn the_preview_serializes_in_camel_case_without_empty_fields() {
    let preview = LinkPreview {
        url: "https://example.org/".into(),
        title: "T".into(),
        description: None,
        site_name: Some("S".into()),
        image_handle_id: None,
    };
    assert_eq!(
        serde_json::to_value(preview).unwrap(),
        serde_json::json!({"url": "https://example.org/", "title": "T", "siteName": "S"})
    );
}

/// Answers every connection with a small page after `delay`, and reports the
/// most connections it ever had open at once.
async fn slow_page_server(delay: Duration) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let open = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let reported = peak.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let (open, peak) = (open.clone(), peak.clone());
            tokio::spawn(async move {
                let now = open.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(now, Ordering::SeqCst);
                let mut request = [0u8; 4096];
                let _ = socket.read(&mut request).await;
                tokio::time::sleep(delay).await;
                let body = "<head><title>Slow</title></head>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
                open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    (format!("http://{addr}"), reported)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn at_most_four_previews_fetch_at_once() {
    use std::sync::atomic::Ordering;
    let (base, peak) = slow_page_server(Duration::from_millis(300)).await;
    let previewer = Arc::new(test_previewer());
    let started = Instant::now();
    let tasks: Vec<_> = (0..8)
        .map(|index| {
            let previewer = previewer.clone();
            let url = format!("{base}/page{index}");
            tokio::spawn(async move { previewer.preview(&url).await })
        })
        .collect();
    for task in tasks {
        assert_eq!(task.await.unwrap().unwrap().title, "Slow");
    }
    assert_eq!(peak.load(Ordering::SeqCst), MAX_CONCURRENT_FETCHES);
    assert!(
        started.elapsed() >= Duration::from_millis(600),
        "two waves of four"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_calls_for_one_url_share_one_fetch() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            html("<head><title>Shared</title></head>").set_delay(Duration::from_millis(300)),
        )
        .expect(1)
        .mount(&server)
        .await;
    let previewer = Arc::new(test_previewer());
    let url = format!("{}/shared", server.uri());
    let tasks: Vec<_> = (0..6)
        .map(|_| {
            let (previewer, url) = (previewer.clone(), url.clone());
            tokio::spawn(async move { previewer.preview(&url).await })
        })
        .collect();
    for task in tasks {
        assert_eq!(task.await.unwrap().unwrap().title, "Shared");
    }
    assert!(previewer.in_flight.lock().unwrap().is_empty());
}

#[tokio::test]
async fn one_timeout_covers_every_redirect_hop() {
    let server = MockServer::start().await;
    let base = server.uri();
    for (from, to) in [("/h1", "/h2"), ("/h2", "/h3"), ("/h3", "/end")] {
        Mock::given(method("GET"))
            .and(path(from))
            .respond_with(redirect(&format!("{base}{to}")).set_delay(Duration::from_millis(400)))
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/end"))
        .respond_with(
            html("<head><title>Late</title></head>").set_delay(Duration::from_millis(400)),
        )
        .mount(&server)
        .await;
    // Every hop is well inside the 8 s request timeout; together they are not.
    let previewer =
        LinkPreviewer::with_limits(allow_test_loopback, Duration::from_secs(1), 4).unwrap();
    let started = Instant::now();
    assert_eq!(previewer.preview(&format!("{base}/h1")).await, None);
    assert!(started.elapsed() < Duration::from_millis(1500));
    let patient =
        LinkPreviewer::with_limits(allow_test_loopback, Duration::from_secs(5), 4).unwrap();
    assert_eq!(
        patient.preview(&format!("{base}/h1")).await.unwrap().title,
        "Late"
    );
}

#[test]
fn the_head_scan_finds_a_marker_split_across_chunks() {
    let page = b"<html><head><title>T</title></HEAD><body>rest";
    let expected = head_end(page).unwrap();
    assert_eq!(&page[expected..expected + 6], b"</HEAD");
    let scan = |chunks: Vec<&[u8]>| {
        let mut body = Vec::new();
        for chunk in chunks {
            let from = body.len().saturating_sub(HEAD_SCAN_OVERLAP);
            body.extend_from_slice(chunk);
            if let Some(end) = head_end_from(&body, from) {
                return Some(end);
            }
        }
        None
    };
    // Every two-chunk split, scanning only each new tail.
    for split in 1..page.len() {
        assert_eq!(
            scan(vec![&page[..split], &page[split..]]),
            Some(expected),
            "split at {split}"
        );
    }
    // One byte at a time.
    assert_eq!(scan(page.chunks(1).collect()), Some(expected));
    assert_eq!(head_end_from(b"<head><title>x", 3), None);
    assert_eq!(head_end_from(b"ab", 10), None);
}
