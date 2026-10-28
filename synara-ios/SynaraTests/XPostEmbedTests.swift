import XCTest
@testable import Synara

/// X post embeds: desktop `xEmbed.test.ts` and the shell's
/// `desktop_x_embed.rs` request rules.
final class XPostEmbedTests: XCTestCase {
    private var videoPost: [String: Any] {
        [
            "__typename": "Tweet",
            "id_str": "1700000000000000001",
            "text": "Launch day &amp; more https://t.co/abc https://t.co/media",
            "display_text_range": [0, 38],
            "entities": ["urls": [["url": "https://t.co/abc", "expanded_url": "https://example.org/launch"]]],
            "created_at": "2026-10-01T12:00:00.000Z",
            "user": [
                "name": "Example",
                "screen_name": "example",
                "profile_image_url_https": "https://pbs.twimg.com/profile_images/1/a_normal.jpg",
            ],
            "mediaDetails": [
                [
                    "type": "video",
                    "media_url_https": "https://pbs.twimg.com/ext_tw_video_thumb/1/pu/img/poster.jpg",
                    "original_info": ["width": 1280, "height": 720],
                    "video_info": [
                        "aspect_ratio": [16, 9],
                        "variants": [
                            ["content_type": "application/x-mpegURL", "url": "https://video.twimg.com/a.m3u8"],
                            ["bitrate": 832_000, "content_type": "video/mp4", "url": "https://video.twimg.com/low.mp4"],
                            ["bitrate": 2_176_000, "content_type": "video/mp4", "url": "https://video.twimg.com/high.mp4"],
                            ["bitrate": 9_000_000, "content_type": "video/mp4", "url": "https://evil.example/x.mp4"],
                        ] as [[String: Any]],
                    ] as [String: Any],
                ] as [String: Any],
            ],
        ]
    }

    func testStatusLinksFromXAndTwitterGiveThePostID() {
        XCTAssertEqual(XPostEmbed.statusID("https://x.com/example/status/1700000000000000001"), "1700000000000000001")
        XCTAssertEqual(XPostEmbed.statusID("https://twitter.com/example/status/20?s=20"), "20")
        XCTAssertEqual(XPostEmbed.statusID("https://mobile.x.com/example/status/20/video/1"), "20")
        XCTAssertEqual(XPostEmbed.statusID("https://x.com/i/web/status/20"), "20")
        XCTAssertEqual(XPostEmbed.statusID("https://www.twitter.com/example/statuses/20"), "20")
        XCTAssertNil(XPostEmbed.statusID("https://x.com/example"))
        XCTAssertNil(XPostEmbed.statusID("https://x.com.evil.example/a/status/20"))
        XCTAssertNil(XPostEmbed.statusID("https://x.com/a/status/20abc"))
        XCTAssertNil(XPostEmbed.statusID("https://user@x.com/a/status/20"))
        XCTAssertNil(XPostEmbed.statusID("ftp://x.com/a/status/20"))
    }

    func testTheFirstLinkOfAMessageIsTheOneThatEmbeds() {
        XCTAssertEqual(
            XPostEmbed.firstStatusLink(in: "look https://x.com/example/status/20 and https://example.org"),
            "https://x.com/example/status/20"
        )
        XCTAssertNil(XPostEmbed.firstStatusLink(in: "first https://example.org then https://x.com/a/status/20"))
        XCTAssertNil(XPostEmbed.firstStatusLink(in: "no links"))
    }

    func testTheTokenMatchesTheOneXEmbedsDerive() {
        // Values from `((Number(id) / 1e15) * Math.PI).toString(36)` in V8.
        XCTAssertEqual(XPostEmbed.syndicationToken("20"), "6dq1a2xwd93")
        XCTAssertEqual(XPostEmbed.syndicationToken("1700000000000000001"), "44cpgxmyurn")
        XCTAssertEqual(XPostEmbed.syndicationToken("1849392023712358400"), "4he1b624r69")
        XCTAssertEqual(XPostEmbed.syndicationToken("463440424141459456"), "14fxvks611f")
    }

    func testJavaScriptRadixStringsMatchV8() {
        XCTAssertEqual(XPostEmbed.javaScriptRadixString(0.5, radix: 36), "0.i")
        XCTAssertEqual(XPostEmbed.javaScriptRadixString(0.5, radix: 2), "0.1")
        XCTAssertEqual(XPostEmbed.javaScriptRadixString(255, radix: 16), "ff")
        XCTAssertEqual(XPostEmbed.javaScriptRadixString(1.0 / 3.0, radix: 36), "0.c")
        XCTAssertEqual(XPostEmbed.javaScriptRadixString(-4.35, radix: 36), "-4.cllllllllk")
    }

    func testOnlyNumericIDsAndBase36TokensBuildARequest() {
        XCTAssertEqual(
            XPostEmbed.syndicationRequestURL(id: "20", token: "6dq1a2xwd93")?.absoluteString,
            "https://cdn.syndication.twimg.com/tweet-result?id=20&lang=en&token=6dq1a2xwd93"
        )
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "", token: "a"))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "20", token: ""))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "20&x=1", token: "a"))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "123456789012345678901", token: "a"))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "20", token: "A"))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "20", token: "a.b"))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "20", token: String(repeating: "a", count: 33)))
        XCTAssertNil(XPostEmbed.syndicationRequestURL(id: "２０", token: "a"), "only ASCII digits")
    }

    func testAVideoPostKeepsTheBestMP4FromXAndItsPoster() throws {
        let post = try XCTUnwrap(XPostEmbed.parse(videoPost, url: "https://x.com/example/status/1700000000000000001"))
        XCTAssertEqual(post.authorHandle, "example")
        XCTAssertEqual(post.authorName, "Example")
        XCTAssertEqual(post.avatarURL, "https://pbs.twimg.com/profile_images/1/a_normal.jpg")
        XCTAssertEqual(post.text, "Launch day & more https://example.org/launch")
        XCTAssertEqual(post.createdAt, "2026-10-01T12:00:00.000Z")
        XCTAssertEqual(post.media, [
            .video(
                src: "https://video.twimg.com/high.mp4",
                poster: "https://pbs.twimg.com/ext_tw_video_thumb/1/pu/img/poster.jpg",
                width: 1280,
                height: 720
            ),
        ])
        XCTAssertEqual(post.cardMedia.count, 1)
    }

    func testDeletedPostsAndMediaOffXCDNsAreDropped() throws {
        XCTAssertNil(XPostEmbed.parse(["__typename": "TweetTombstone"], url: "https://x.com/a/status/1"))
        XCTAssertNil(XPostEmbed.parse(["__typename": "Tweet", "id_str": "1"], url: "https://x.com/a/status/1"))
        var offCDN = videoPost
        offCDN["user"] = ["name": "Example", "screen_name": "example", "profile_image_url_https": "https://evil.example/a.jpg"]
        offCDN["mediaDetails"] = [["type": "photo", "media_url_https": "https://evil.example/p.jpg"]]
        let post = try XCTUnwrap(XPostEmbed.parse(offCDN, url: "https://x.com/a/status/1"))
        XCTAssertNil(post.avatarURL)
        XCTAssertEqual(post.media, [])
    }

    func testPhotosAreCappedAtFourAndGifsLoop() throws {
        var photos = videoPost
        photos["mediaDetails"] = (1 ... 5).map { index in
            ["type": "photo", "media_url_https": "https://pbs.twimg.com/media/\(index).jpg"]
        }
        let post = try XCTUnwrap(XPostEmbed.parse(photos, url: "https://x.com/a/status/1"))
        XCTAssertEqual(post.media.count, 4)
        XCTAssertEqual(post.cardMedia.count, 4)
        XCTAssertEqual(post.media.first, .photo(url: "https://pbs.twimg.com/media/1.jpg", width: nil, height: nil))

        var gif = videoPost
        gif["mediaDetails"] = [[
            "type": "animated_gif",
            "video_info": ["variants": [["content_type": "video/mp4", "url": "https://video.twimg.com/g.mp4"]]],
        ] as [String: Any]]
        XCTAssertEqual(
            XPostEmbed.parse(gif, url: "https://x.com/a/status/1")?.media,
            [.gif(src: "https://video.twimg.com/g.mp4", poster: nil, width: nil, height: nil)]
        )
    }

    func testLongTextIsCappedAndTheRangeCountsCodePoints() throws {
        var long = videoPost
        long["text"] = String(repeating: "a", count: 1200)
        long["display_text_range"] = [0, 1200]
        let capped = try XCTUnwrap(XPostEmbed.parse(long, url: "https://x.com/a/status/1"))
        XCTAssertEqual(capped.text, String(repeating: "a", count: 1000) + "…")

        var emoji = videoPost
        emoji["text"] = "😀😀 hi https://t.co/media"
        emoji["display_text_range"] = [0, 5]
        XCTAssertEqual(try XCTUnwrap(XPostEmbed.parse(emoji, url: "https://x.com/a/status/1")).text, "😀😀 hi")
    }

    func testFetchSendsOnlyTheIDAndTokenAndNeedsJSON() async throws {
        let body = try JSONSerialization.data(withJSONObject: videoPost)
        let session = StubURLProtocol.session { _ in (200, ["Content-Type": "application/json; charset=utf-8"], body) }
        let post = await XPostEmbed.fetch("https://x.com/example/status/1700000000000000001", session: session)
        XCTAssertEqual(post?.id, "1700000000000000001")
        let request = try XCTUnwrap(StubURLProtocol.requests.last)
        XCTAssertEqual(
            request.url?.absoluteString,
            "https://cdn.syndication.twimg.com/tweet-result?id=1700000000000000001&lang=en&token="
                + XPostEmbed.syndicationToken("1700000000000000001")
        )
        XCTAssertEqual(request.value(forHTTPHeaderField: "Accept"), "application/json")

        let notJSON = StubURLProtocol.session { _ in (200, ["Content-Type": "application/json"], Data("not json".utf8)) }
        let none = await XPostEmbed.fetch("https://x.com/example/status/1700000000000000002", session: notJSON)
        XCTAssertNil(none)
        let html = StubURLProtocol.session { _ in (200, ["Content-Type": "text/html"], body) }
        let wrongType = await XPostEmbed.fetch("https://x.com/example/status/1700000000000000003", session: html)
        XCTAssertNil(wrongType)
        let missing = StubURLProtocol.session { _ in (404, ["Content-Type": "application/json"], body) }
        let notFound = await XPostEmbed.fetch("https://x.com/example/status/1700000000000000004", session: missing)
        XCTAssertNil(notFound)
        let notAPost = await XPostEmbed.fetch("https://example.org/status/1", session: session)
        XCTAssertNil(notAPost)
    }
}
