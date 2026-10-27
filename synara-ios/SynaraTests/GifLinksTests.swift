import ImageIO
import UIKit
import UniformTypeIdentifiers
import XCTest
@testable import Synara

/// GIF links (desktop `gifLinks.test.ts`), the typed `/gif` command, the
/// remote-content safety rules, and animated GIF decoding.
final class GifLinksTests: XCTestCase {
    // MARK: Inline GIF links

    func testKlipyAndGiphyMediaLinksPlayInlineKeptExactlyAsSent() {
        let klipy = "https://static1.klipy.com/ii/abc/def/md.gif?x=1"
        XCTAssertEqual(GifLinks.inlineGifLink(klipy), InlineGifLink(url: klipy, host: "static1.klipy.com"))
        XCTAssertNotNil(GifLinks.inlineGifLink("https://media3.giphy.com/media/abc/giphy.gif"))
        XCTAssertNotNil(GifLinks.inlineGifLink("https://i.giphy.com/abc.webp"))
        XCTAssertNotNil(GifLinks.inlineGifLink("https://static.klipy.com/ii/abc/md.GIF"), "the extension is case-insensitive")
    }

    func testOtherHostsNonImagesAndUnsafeLinksStayOrdinaryLinks() {
        XCTAssertNil(GifLinks.inlineGifLink("https://example.org/cat.gif"))
        XCTAssertNil(GifLinks.inlineGifLink("https://static.klipy.com.evil.example/cat.gif"))
        XCTAssertNil(GifLinks.inlineGifLink("http://static.klipy.com/cat.gif"))
        XCTAssertNil(GifLinks.inlineGifLink("https://static.klipy.com/cat.mp4"))
        XCTAssertNil(GifLinks.inlineGifLink("https://klipy.com/gifs/cat"))
        XCTAssertNil(GifLinks.inlineGifLink("https://user@static.klipy.com/cat.gif"))
    }

    func testOnlyAMessageThatIsNothingButTheLinkBecomesAGif() {
        let url = "https://static.klipy.com/ii/abc/md.gif"
        XCTAssertEqual(GifLinks.messageInlineGif("  \(url)\n")?.url, url)
        XCTAssertNil(GifLinks.messageInlineGif("look \(url)"))
        XCTAssertNil(GifLinks.messageInlineGif(""))
    }

    // MARK: Typed /gif

    func testGifCommandOpensThePickerWithItsSearch() {
        XCTAssertEqual(GifCommand.query(in: "/gif"), "")
        XCTAssertEqual(GifCommand.query(in: "  /gif  "), "")
        XCTAssertEqual(GifCommand.query(in: "/gif high five"), "high five")
        XCTAssertEqual(GifCommand.query(in: "/gif   party  "), "party")
        XCTAssertEqual(GifCommand.query(in: "/gif\ncats"), "cats")
    }

    func testOtherSlashTextIsAnOrdinaryMessage() {
        XCTAssertNil(GifCommand.query(in: "/gifs are fun"))
        XCTAssertNil(GifCommand.query(in: "/giffy"))
        XCTAssertNil(GifCommand.query(in: "a /gif"))
        XCTAssertNil(GifCommand.query(in: "/GIF"))
        XCTAssertNil(GifCommand.query(in: "gif"))
        XCTAssertNil(GifCommand.query(in: ""))
    }

    // MARK: Remote content safety (desktop `remoteContent.ts`)

    func testRemoteContentNeedsPublicHTTPSWithoutCredentials() {
        XCTAssertTrue(RemoteContentURL.isSafeHTTPS("https://cdn.example.org/a.gif"))
        XCTAssertTrue(RemoteContentURL.isSafeHTTPS("https://93.184.216.34/a.gif"))
        for unsafe in [
            "http://cdn.example.org/a.gif",
            "https://user:pass@cdn.example.org/a.gif",
            "https://localhost/a.gif",
            "https://printer.local/a.gif",
            "https://router.home.arpa/a.gif",
            "https://127.0.0.1/a.gif",
            "https://10.0.0.8/a.gif",
            "https://172.20.1.1/a.gif",
            "https://192.168.1.1/a.gif",
            "https://169.254.1.1/a.gif",
            "https://100.64.0.1/a.gif",
            "https://[::1]/a.gif",
            "https://[fd00::1]/a.gif",
            "https://[::ffff:127.0.0.1]/a.gif",
            "https://2130706433/a.gif",
            "https://0x7f.1/a.gif",
            "https://cdn.example.org/a b.gif",
            "not a url",
        ] {
            XCTAssertFalse(RemoteContentURL.isSafeHTTPS(unsafe), unsafe)
        }
    }

    // MARK: Settings

    func testRichLinkSettingsDefaultOn() {
        let store = InMemorySettingsStore()
        XCTAssertTrue(store.bool(for: RichLinkSettings.inlineGifLinksKey))
        XCTAssertTrue(store.bool(for: RichLinkSettings.gifSearchKey))
        XCTAssertTrue(store.bool(for: RichLinkSettings.xEmbedsKey))
        store.set(false, for: RichLinkSettings.xEmbedsKey)
        XCTAssertFalse(store.bool(for: RichLinkSettings.xEmbedsKey))
        XCTAssertFalse(store.bool(for: "synara.settings.unrelated"), "other keys keep their false default")

        let suite = "synara.tests.richLinks.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }
        let persisted = UserDefaultsSettingsStore(defaults: defaults)
        XCTAssertTrue(persisted.bool(for: RichLinkSettings.gifSearchKey))
        persisted.set(false, for: RichLinkSettings.gifSearchKey)
        XCTAssertFalse(UserDefaultsSettingsStore(defaults: defaults).bool(for: RichLinkSettings.gifSearchKey))
    }

    // MARK: Animated decoding

    func testAnimatedGifKeepsItsFramesAndTiming() throws {
        let data = try Self.gif(frameDelays: [0.1, 0.3, 0.1], side: 40)
        let image = try XCTUnwrap(AnimatedImageDecoder.image(from: data))
        XCTAssertTrue(image.isAnimated)
        XCTAssertEqual(image.duration, 0.5, accuracy: 0.01)
        // The 0.3 s frame is shown three times as long as the others.
        XCTAssertEqual(image.images?.count, 5)
        XCTAssertEqual(Set((image.images ?? []).map(ObjectIdentifier.init)).count, 3)
    }

    func testLargeAnimationsDropFramesToStayInsideTheBudget() throws {
        let data = try Self.gif(frameDelays: Array(repeating: 0.05, count: 12), side: 64)
        let limits = AnimatedImageDecoder.Limits(maxPixelSize: 64, maxFrames: 200, maxDecodedBytes: 64 * 64 * 4 * 4)
        let image = try XCTUnwrap(AnimatedImageDecoder.image(from: data, limits: limits))
        let distinct = Set((image.images ?? []).map(ObjectIdentifier.init)).count
        XCTAssertLessThanOrEqual(distinct, 4)
        XCTAssertEqual(image.duration, 0.6, accuracy: 0.01, "dropped frames give their time to the kept ones")
    }

    func testFramesAreDownsampledAndAStillImageStaysStill() throws {
        let animated = try XCTUnwrap(
            AnimatedImageDecoder.image(
                from: try Self.gif(frameDelays: [0.1, 0.1], side: 400),
                limits: .init(maxPixelSize: 100)
            )
        )
        XCTAssertLessThanOrEqual(max(animated.size.width, animated.size.height) * animated.scale, 100)

        let still = try XCTUnwrap(AnimatedImageDecoder.image(from: try Self.gif(frameDelays: [0.1], side: 20)))
        XCTAssertFalse(still.isAnimated)
        XCTAssertNil(AnimatedImageDecoder.image(from: Data("not an image".utf8)))
    }

    private static func gif(frameDelays: [Double], side: Int) throws -> Data {
        let data = NSMutableData()
        let destination = try XCTUnwrap(
            CGImageDestinationCreateWithData(data, UTType.gif.identifier as CFString, frameDelays.count, nil)
        )
        let renderer = UIGraphicsImageRenderer(size: CGSize(width: side, height: side), format: {
            let format = UIGraphicsImageRendererFormat()
            format.scale = 1
            return format
        }())
        for (index, delay) in frameDelays.enumerated() {
            let frame = renderer.image { context in
                UIColor(hue: CGFloat(index) / CGFloat(max(1, frameDelays.count)), saturation: 1, brightness: 1, alpha: 1)
                    .setFill()
                context.fill(CGRect(x: 0, y: 0, width: side, height: side))
            }
            let properties = [
                kCGImagePropertyGIFDictionary: [kCGImagePropertyGIFDelayTime: delay],
            ] as CFDictionary
            CGImageDestinationAddImage(destination, try XCTUnwrap(frame.cgImage), properties)
        }
        XCTAssertTrue(CGImageDestinationFinalize(destination))
        return data as Data
    }
}
