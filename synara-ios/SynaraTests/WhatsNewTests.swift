import Foundation
import XCTest
@testable import Synara

/// "What's New" after an update (docs/whats-new-notes.md): the notes loader,
/// the launch decision, version order and when the version is recorded.
final class WhatsNewTests: XCTestCase {
    /// A notes file whose `items` (and optionally `headline`) are raw JSON.
    private func notesFile(
        version: String = "3.2.0",
        headline: String = #""Easier settings, cleaner timestamps""#,
        items: String
    ) -> Data {
        Data(#"{"version": "\#(version)", "headline": \#(headline), "items": \#(items)}"#.utf8)
    }

    private let sampleNotes = WhatsNewNotes(
        version: "3.2.0",
        headline: "Easier settings",
        items: ["Settings rows on iPhone respond to taps again."]
    )

    // MARK: Loader

    func testLoaderKeepsOnlyIosItemsOfTheMatchingVersion() {
        let data = notesFile(items: #"""
        [
          { "text": "Click your profile picture to open Settings.", "platforms": ["desktop"] },
          { "text": "Settings rows on iPhone respond to taps again.", "platforms": ["ios"] },
          { "text": " After an update, Synara now shows what changed. ", "platforms": ["desktop", "ios"] }
        ]
        """#)
        let notes = WhatsNewNotesLoader.notes(fromJSON: data, version: "3.2.0")
        XCTAssertEqual(
            notes,
            WhatsNewNotes(
                version: "3.2.0",
                headline: "Easier settings, cleaner timestamps",
                items: [
                    "Settings rows on iPhone respond to taps again.",
                    "After an update, Synara now shows what changed.",
                ]
            )
        )
        XCTAssertEqual(
            notes?.releaseURL.absoluteString,
            "https://github.com/nepenth/synara-desktop/releases/tag/v3.2.0"
        )
    }

    func testLoaderRejectsAFileForAnotherVersion() {
        let data = notesFile(version: "3.1.1", items: #"[{ "text": "A fix.", "platforms": ["ios"] }]"#)
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: data, version: "3.2.0"))
    }

    func testLoaderRejectsMalformedFiles() {
        let oneItem = #"[{ "text": "A fix.", "platforms": ["ios"] }]"#
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: Data("{not json".utf8), version: "3.2.0"))
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: Data(#"["3.2.0"]"#.utf8), version: "3.2.0"))
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: notesFile(headline: "42", items: oneItem), version: "3.2.0"))
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: notesFile(headline: #""  ""#, items: oneItem), version: "3.2.0"))
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: notesFile(items: #""A fix.""#), version: "3.2.0"))
        // Broken items are skipped; a file left with none shows nothing.
        let brokenItems = #"""
        [
          "A fix.",
          { "text": 7, "platforms": ["ios"] },
          { "text": "", "platforms": ["ios"] },
          { "text": "A fix.", "platforms": "ios" }
        ]
        """#
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: notesFile(items: brokenItems), version: "3.2.0"))
    }

    func testLoaderShowsNothingWithoutIosItems() {
        let data = notesFile(items: #"[{ "text": "Click your profile picture to open Settings.", "platforms": ["desktop"] }]"#)
        XCTAssertNil(WhatsNewNotesLoader.notes(fromJSON: data, version: "3.2.0"))
    }

    func testLoaderShowsAtMostFourItems() throws {
        let items = (1...6)
            .map { #"{ "text": "Change \#($0).", "platforms": ["ios"] }"# }
            .joined(separator: ", ")
        let notes = try XCTUnwrap(WhatsNewNotesLoader.notes(fromJSON: notesFile(items: "[\(items)]"), version: "3.2.0"))
        XCTAssertEqual(notes.items, ["Change 1.", "Change 2.", "Change 3.", "Change 4."])
    }

    func testLoaderFindsNoFileForAnUnknownVersion() {
        XCTAssertNil(WhatsNewNotesLoader.bundledNotes(version: "0.0.1", bundle: Bundle(for: WhatsNewPresentation.self)))
    }

    /// project.yml copies release-notes/ into the app; the release check
    /// requires a file for every version, so the app's own one is there.
    func testTheAppBundlesTheNotesForItsOwnVersion() throws {
        let bundle = Bundle(for: WhatsNewPresentation.self)
        let version = try XCTUnwrap(bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String)
        XCTAssertNotNil(
            bundle.url(forResource: "v\(version)", withExtension: "json", subdirectory: WhatsNewNotesLoader.directory),
            "release-notes/v\(version).json is not in the app bundle"
        )
        let notes = try XCTUnwrap(WhatsNewNotesLoader.bundledNotes(version: version, bundle: bundle))
        XCTAssertEqual(notes.version, version)
        XCTAssertFalse(notes.items.isEmpty)
        XCTAssertLessThanOrEqual(notes.items.count, WhatsNewNotesLoader.maxItems)
    }

    // MARK: Version order

    func testVersionsCompareNumerically() {
        XCTAssertGreaterThan(WhatsNewPolicy.compareVersions("3.10.0", "3.9.0"), 0)
        XCTAssertLessThan(WhatsNewPolicy.compareVersions("3.1.1", "3.2.0"), 0)
        XCTAssertLessThan(WhatsNewPolicy.compareVersions("2.9.9", "3.0.0"), 0)
        XCTAssertEqual(WhatsNewPolicy.compareVersions("3.2.0", "3.2.0"), 0)
        XCTAssertEqual(WhatsNewPolicy.compareVersions("v3.2", "3.2.0"), 0)
        XCTAssertGreaterThan(WhatsNewPolicy.compareVersions("3.2.0.1", "3.2.0"), 0)
        XCTAssertLessThan(WhatsNewPolicy.compareVersions("", "3.2.0"), 0)
    }

    // MARK: Launch decision

    func testFreshInstallRecordsSilently() {
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: nil, currentVersion: "3.2.0", session: .signedOut, hasNotes: true),
            .record
        )
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: "", currentVersion: "3.2.0", session: .signedOut, hasNotes: false),
            .record
        )
    }

    func testFirstVersionWithTheFeatureShowsToSignedInPeople() {
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: nil, currentVersion: "3.2.0", session: .signedIn, hasNotes: true),
            .show
        )
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: nil, currentVersion: "3.2.0", session: .signedIn, hasNotes: false),
            .none
        )
    }

    func testUnknownSessionDoesNothingThisLaunch() {
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: nil, currentVersion: "3.2.0", session: .unknown, hasNotes: true),
            .none
        )
    }

    func testAnUpdateShowsTheCurrentNotes() {
        for session in [WhatsNewSessionState.signedIn, .signedOut, .unknown] {
            XCTAssertEqual(
                WhatsNewPolicy.decideAtStartup(seenVersion: "3.1.1", currentVersion: "3.2.0", session: session, hasNotes: true),
                .show
            )
        }
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: "3.9.0", currentVersion: "3.10.0", session: .signedIn, hasNotes: true),
            .show
        )
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: "3.1.1", currentVersion: "3.2.0", session: .signedIn, hasNotes: false),
            .none
        )
    }

    func testSameVersionOrDowngradeShowsNothing() {
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: "3.2.0", currentVersion: "3.2.0", session: .signedIn, hasNotes: true),
            .none
        )
        XCTAssertEqual(
            WhatsNewPolicy.decideAtStartup(seenVersion: "3.10.0", currentVersion: "3.9.0", session: .signedIn, hasNotes: true),
            .none
        )
    }

    // MARK: Presentation and recording

    func testLaunchAfterAnUpdateShowsTheCardAndDismissRecords() {
        let store = InMemoryWhatsNewSeenVersionStore(seenVersion: "3.1.1")
        let presentation = WhatsNewPresentation(currentVersion: "3.2.0", notes: sampleNotes, store: store)
        presentation.evaluateAtLaunch(session: .signedIn)
        XCTAssertEqual(presentation.pending, sampleNotes)
        XCTAssertEqual(store.seenVersion, "3.1.1", "showing alone records nothing")

        presentation.dismiss()
        XCTAssertNil(presentation.pending)
        XCTAssertEqual(store.seenVersion, "3.2.0")
    }

    func testFreshInstallRecordsWithoutACardAndLaterSignInDoesNotShowIt() {
        let store = InMemoryWhatsNewSeenVersionStore()
        let presentation = WhatsNewPresentation(currentVersion: "3.2.0", notes: sampleNotes, store: store)
        presentation.evaluateAtLaunch(session: .signedOut)
        XCTAssertNil(presentation.pending)
        XCTAssertEqual(store.seenVersion, "3.2.0")

        // Decided once per process, from the session restored at launch.
        presentation.evaluateAtLaunch(session: .signedIn)
        XCTAssertNil(presentation.pending)
    }

    func testUnknownSessionRecordsNothing() {
        let store = InMemoryWhatsNewSeenVersionStore()
        let presentation = WhatsNewPresentation(currentVersion: "3.2.0", notes: sampleNotes, store: store)
        presentation.evaluateAtLaunch(session: .unknown)
        XCTAssertNil(presentation.pending)
        XCTAssertNil(store.seenVersion)
    }

    func testBackgroundRecordsOnlyAfterTheCardWasVisible() {
        let store = InMemoryWhatsNewSeenVersionStore(seenVersion: "3.1.1")
        let presentation = WhatsNewPresentation(currentVersion: "3.2.0", notes: sampleNotes, store: store)
        presentation.evaluateAtLaunch(session: .signedIn)

        presentation.sceneDidEnterBackground()
        XCTAssertEqual(presentation.pending, sampleNotes, "never on screen, so it waits")
        XCTAssertEqual(store.seenVersion, "3.1.1")

        XCTAssertTrue(presentation.cardDidAppear(), "the first appearance is announced")
        XCTAssertFalse(presentation.cardDidAppear(), "scrolling back is not")
        presentation.sceneDidEnterBackground()
        XCTAssertNil(presentation.pending)
        XCTAssertEqual(store.seenVersion, "3.2.0")
    }

    func testNoNotesOrDisabledShowsNothing() {
        let store = InMemoryWhatsNewSeenVersionStore(seenVersion: "3.1.1")
        let withoutNotes = WhatsNewPresentation(currentVersion: "3.2.0", notes: nil, store: store)
        withoutNotes.evaluateAtLaunch(session: .signedIn)
        XCTAssertNil(withoutNotes.pending)
        XCTAssertFalse(withoutNotes.cardDidAppear())

        let disabled = WhatsNewPresentation(currentVersion: "3.2.0", notes: sampleNotes, store: store, isEnabled: false)
        disabled.evaluateAtLaunch(session: .signedIn)
        XCTAssertNil(disabled.pending)
        XCTAssertEqual(store.seenVersion, "3.1.1")
    }

    func testUITestsShowTheCardOnlyWhenAsked() {
        let bundle = Bundle(for: WhatsNewPresentation.self)
        let quiet = WhatsNewPresentation.launch(environment: ["SYNARA_UI_TESTS": "1"], bundle: bundle)
        quiet.evaluateAtLaunch(session: .signedIn)
        XCTAssertNil(quiet.pending)

        let asked = WhatsNewPresentation.launch(
            environment: ["SYNARA_UI_TESTS": "1", "SYNARA_UI_TEST_WHATS_NEW_SEEN_VERSION": "0.0.1"],
            bundle: bundle
        )
        asked.evaluateAtLaunch(session: .signedIn)
        XCTAssertNotNil(asked.pending)
        XCTAssertEqual(asked.pending, asked.currentNotes)
    }

    func testUserDefaultsStoreUsesItsKey() throws {
        let suiteName = "WhatsNewTests-\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let store = UserDefaultsWhatsNewSeenVersionStore(defaults: defaults)
        XCTAssertNil(store.seenVersion)
        store.recordSeenVersion("3.2.0")
        XCTAssertEqual(defaults.string(forKey: "synara.whatsNewSeenVersion"), "3.2.0")
        XCTAssertEqual(UserDefaultsWhatsNewSeenVersionStore(defaults: defaults).seenVersion, "3.2.0")
    }
}
