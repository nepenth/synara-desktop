import Foundation
import XCTest
@testable import Synara

/// I27: user-facing copy comes from the String Catalogs, counts use plural
/// variations, and dates use localized format styles. English is unchanged.
final class LocalizationCatalogTests: XCTestCase {
    private typealias Catalog = [String: Any]

    // MARK: Catalog contents

    func testSampleLocalizedKeysExistInTheAppCatalog() throws {
        let strings = try catalogStrings("synara-ios/Synara/Resources/Localizable.xcstrings")
        let sample = [
            "Connected",
            "Couldn't restore this session. Log out and sign in again.",
            "Remove this message? This can't be undone.",
            "Remove",
            "Cancel",
            "Log Out",
            "Log out session?",
            "Could not log out that session.",
            "Avatar updated.",
            "Due %@",
            "Last activity %@",
            "%@ is typing...",
            "%lld replies",
            "%lld members",
        ]
        for key in sample {
            XCTAssertNotNil(strings[key], "missing from Localizable.xcstrings: \(key)")
        }
    }

    func testNotificationCopyLivesInTheSharedCatalog() throws {
        let strings = try catalogStrings("synara-ios/SynaraShared/SynaraShared.xcstrings")
        for key in ["New activity", "Agent approval needed", "sent an image", "%@: %@"] {
            XCTAssertNotNil(strings[key], "missing from SynaraShared.xcstrings: \(key)")
        }
    }

    /// Every `String(localized: "…")` without interpolation in the app's and
    /// the shared sources has its key in the catalog for its table, so a
    /// catalog that falls behind the code fails here instead of shipping.
    func testEveryPlainLocalizedLiteralIsInItsCatalog() throws {
        let root = repositoryRoot()
        let app = try catalogStrings("synara-ios/Synara/Resources/Localizable.xcstrings")
        let shared = try catalogStrings("synara-ios/SynaraShared/SynaraShared.xcstrings")
        let pattern = try NSRegularExpression(
            pattern: #"String\(localized: "((?:[^"\\]|\\.)*)"(, table: "SynaraShared")?"#
        )
        var checked = 0
        for directory in ["synara-ios/Synara", "synara-ios/SynaraShared", "synara-ios/SynaraNotificationService"] {
            let base = URL(fileURLWithPath: root).appendingPathComponent(directory)
            let enumerator = FileManager.default.enumerator(at: base, includingPropertiesForKeys: nil)
            while let url = enumerator?.nextObject() as? URL {
                guard url.pathExtension == "swift" else { continue }
                let source = try String(contentsOf: url, encoding: .utf8)
                let range = NSRange(source.startIndex..., in: source)
                for match in pattern.matches(in: source, range: range) {
                    guard let literalRange = Range(match.range(at: 1), in: source) else { continue }
                    let literal = String(source[literalRange])
                    guard literal.contains("\\(") == false else { continue }
                    let key = unescape(literal)
                    let table = match.range(at: 2).location == NSNotFound ? app : shared
                    XCTAssertNotNil(table[key], "\(url.lastPathComponent): \(key)")
                    checked += 1
                }
            }
        }
        XCTAssertGreaterThan(checked, 300, "the scan found the localized literals")
    }

    func testCountKeysHavePluralVariations() throws {
        let strings = try catalogStrings("synara-ios/Synara/Resources/Localizable.xcstrings")
        let plural: [String: String] = [
            "%lld replies": "%lld reply",
            "%lld members": "%lld member",
            "%lld rooms": "%lld room",
            "%lld channels": "%lld channel",
            "%lld items changed": "%lld item changed",
            "%lld chars": "%lld char",
            "%lld lines": "%lld line",
            "Choose up to %lld answers": "Choose up to %lld answer",
            "You can attach up to %lld attachments.": "You can attach up to %lld attachment.",
        ]
        for (key, one) in plural {
            let variations = try XCTUnwrap(
                (((strings[key] as? Catalog)?["localizations"] as? Catalog)?["en"] as? Catalog)?["variations"]
                    as? Catalog,
                key
            )
            let forms = try XCTUnwrap(variations["plural"] as? Catalog, key)
            XCTAssertEqual(value(forms["one"]), one, key)
            XCTAssertEqual(value(forms["other"]), key, key)
        }
        for key in ["%@, %lld votes", "%@, %lld unread", "%@, %@, and %lld more are typing..."] {
            let english = try XCTUnwrap(
                ((strings[key] as? Catalog)?["localizations"] as? Catalog)?["en"] as? Catalog,
                key
            )
            XCTAssertNotNil(english["substitutions"], "\(key) varies its count argument")
        }
    }

    // MARK: Runtime lookups (the compiled catalog in the app bundle)

    func testPluralsResolveFromTheCompiledCatalog() {
        XCTAssertEqual(String(localized: "\(1) replies"), "1 reply")
        XCTAssertEqual(String(localized: "\(3) replies"), "3 replies")
        XCTAssertEqual(String(localized: "\(1) members"), "1 member")
        XCTAssertEqual(String(localized: "\(2) members"), "2 members")
        XCTAssertEqual(String(localized: "\(1) items changed"), "1 item changed")
        XCTAssertEqual(String(localized: "\("Yes"), \(1) votes"), "Yes, 1 vote")
        XCTAssertEqual(String(localized: "\("Yes"), \(4) votes"), "Yes, 4 votes")
        XCTAssertEqual(RoomTypingPresentation.text(for: ["@a:x", "@b:x", "@c:x"]), "a, b, and 1 more are typing...")
        XCTAssertEqual(
            RoomTypingPresentation.text(for: ["@a:x", "@b:x", "@c:x", "@d:x"]),
            "a, b, and 2 more are typing..."
        )
        XCTAssertEqual(TimelineRedactConfirmCopy.question, "Remove this message? This can't be undone.")
        XCTAssertEqual(ConnectionStatusCopy.connected, "Connected")
    }

    // MARK: Dates

    func testTimeFormattingKeepsTheTwentyFourHourSettingAndNamedDays() throws {
        let utc = try XCTUnwrap(TimeZone(identifier: "UTC"))
        let locale = Locale(identifier: "en_US")
        let morning = Date(timeIntervalSince1970: 1_767_258_300) // 2026-01-01 09:05 UTC
        let twentyFour = SynaraTimeFormatting.Formatters(
            configuration: .init(uses24HourClock: true, locale: locale, timeZone: utc)
        )
        XCTAssertEqual(twentyFour.clockTime.format(morning), "09:05", "two-digit hour, as before")
        XCTAssertEqual(twentyFour.dateTime.format(morning), "Jan 1, 2026 at 09:05")
        XCTAssertEqual(twentyFour.relativeDayName(offset: 0), "Today")
        XCTAssertEqual(twentyFour.relativeDayName(offset: -1), "Yesterday")

        let twelve = SynaraTimeFormatting.Formatters(
            configuration: .init(uses24HourClock: false, locale: locale, timeZone: utc)
        )
        XCTAssertEqual(twelve.clockTime.format(morning), "9:05\u{202F}AM")
        XCTAssertEqual(twelve.dateTime.format(morning), "Jan 1, 2026 at 9:05\u{202F}AM")
    }

    // MARK: Helpers

    private func value(_ form: Any?) -> String? {
        ((form as? Catalog)?["stringUnit"] as? Catalog)?["value"] as? String
    }

    private func unescape(_ literal: String) -> String {
        literal
            .replacingOccurrences(of: "\\n", with: "\n")
            .replacingOccurrences(of: "\\\"", with: "\"")
            .replacingOccurrences(of: "\\\\", with: "\\")
    }

    private func catalogStrings(_ relativePath: String) throws -> Catalog {
        let url = URL(fileURLWithPath: repositoryRoot()).appendingPathComponent(relativePath)
        let data = try Data(contentsOf: url)
        let json = try XCTUnwrap(try JSONSerialization.jsonObject(with: data) as? Catalog)
        XCTAssertEqual(json["sourceLanguage"] as? String, "en")
        return try XCTUnwrap(json["strings"] as? Catalog)
    }

    private func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}
