import Foundation

/// "What's New" after an update: one notes file per release,
/// `release-notes/v<version>.json` at the repository root (format and limits:
/// docs/whats-new-notes.md, checked by scripts/check-whats-new.mjs). The app
/// bundles every file (project.yml) and shows the running version's iOS items
/// once, in a card at the top of Rooms; Settings › About › What's New shows
/// them again. Same rules as desktop's `whatsNewNotes.ts`.
struct WhatsNewNotes: Equatable {
    let version: String
    let headline: String
    let items: [String]

    var releaseURL: URL {
        WhatsNewNotes.releaseURL(for: version)
    }

    static let releasesURL = URL(string: "https://github.com/nepenth/synara-desktop/releases")!

    static func releaseURL(for version: String) -> URL {
        guard let tag = "v\(version)".addingPercentEncoding(withAllowedCharacters: .urlPathAllowed),
              let url = URL(string: "\(releasesURL.absoluteString)/tag/\(tag)")
        else {
            return releasesURL
        }
        return url
    }
}

enum WhatsNewNotesLoader {
    /// The bundle folder that holds `v<version>.json` (project.yml).
    static let directory = "release-notes"
    /// The checker allows at most this many items per platform; the card never shows more.
    static let maxItems = 4
    static let platform = "ios"

    /// The iOS notes in one notes file for `version`, or nil when the file is
    /// for another version, is not in the expected shape, or has no iOS
    /// items. The release check rejects such files; this keeps a bad one
    /// from drawing a broken card.
    static func notes(fromJSON data: Data, version: String) -> WhatsNewNotes? {
        guard let file = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
              file["version"] as? String == version,
              let headline = (file["headline"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines),
              headline.isEmpty == false,
              let rawItems = file["items"] as? [Any]
        else {
            return nil
        }
        let items = rawItems
            .compactMap { rawItem -> String? in
                guard let item = rawItem as? [String: Any],
                      let text = (item["text"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines),
                      text.isEmpty == false,
                      let platforms = item["platforms"] as? [Any],
                      platforms.contains(where: { $0 as? String == platform })
                else {
                    return nil
                }
                return text
            }
            .prefix(maxItems)
        guard items.isEmpty == false else {
            return nil
        }
        return WhatsNewNotes(version: version, headline: headline, items: Array(items))
    }

    /// The bundled notes for `version`, or nil when the app has no file for it.
    static func bundledNotes(version: String, bundle: Bundle = .main) -> WhatsNewNotes? {
        guard let url = bundle.url(forResource: "v\(version)", withExtension: "json", subdirectory: directory),
              let data = try? Data(contentsOf: url)
        else {
            return nil
        }
        return notes(fromJSON: data, version: version)
    }
}

/// Whether a session was restored at launch; `unknown` when the session
/// store could not be read.
enum WhatsNewSessionState: Equatable {
    case signedIn
    case signedOut
    case unknown
}

/// - `record`: a fresh install; note the current version without showing anything.
/// - `show`: show the current version's notes once the Rooms list is ready.
/// - `none`: nothing to do.
enum WhatsNewStartupDecision: Equatable {
    case record
    case show
    case none
}

enum WhatsNewPolicy {
    /// What to do with "What's New" when the app launches.
    ///
    /// There is no record on a fresh install, and also on the first launch
    /// of the first version with this feature. A restored session tells them
    /// apart: someone signed in has used an earlier version, so they see the
    /// notes; with no session it is a first run, and the version is recorded
    /// silently. When the session store could not be read, nothing is
    /// recorded, so the question is asked again on the next launch. Skipped
    /// versions show only the current notes, and a downgrade or the same
    /// version shows nothing.
    static func decideAtStartup(
        seenVersion: String?,
        currentVersion: String,
        session: WhatsNewSessionState,
        hasNotes: Bool
    ) -> WhatsNewStartupDecision {
        guard let seenVersion, seenVersion.isEmpty == false else {
            switch session {
            case .signedOut:
                return .record
            case .unknown:
                return .none
            case .signedIn:
                return hasNotes ? .show : .none
            }
        }
        guard compareVersions(seenVersion, currentVersion) < 0 else {
            return .none
        }
        return hasNotes ? .show : .none
    }

    /// Numeric comparison of dotted versions ("3.10.0" > "3.9.0"); a leading
    /// "v" and non-numeric parts are ignored, missing parts count as 0.
    /// Negative when `left` is older, 0 when equal, positive when newer.
    static func compareVersions(_ left: String, _ right: String) -> Int {
        let leftParts = versionParts(left)
        let rightParts = versionParts(right)
        for index in 0..<max(leftParts.count, rightParts.count) {
            let leftPart = index < leftParts.count ? leftParts[index] : 0
            let rightPart = index < rightParts.count ? rightParts[index] : 0
            if leftPart != rightPart {
                return leftPart < rightPart ? -1 : 1
            }
        }
        return 0
    }

    private static func versionParts(_ version: String) -> [Int] {
        var trimmed = version.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.first == "v" || trimmed.first == "V" {
            trimmed.removeFirst()
        }
        return trimmed
            .split(whereSeparator: { $0 == "." || $0 == "-" })
            .compactMap { part in Int(part.prefix(while: \.isNumber)) }
    }
}

/// The last version whose notes were shown (or skipped on a fresh install)
/// on this device.
protocol WhatsNewSeenVersionStoring: AnyObject {
    var seenVersion: String? { get }
    func recordSeenVersion(_ version: String)
}

/// Device-local, in the standard defaults next to `synara.roomListSort`.
final class UserDefaultsWhatsNewSeenVersionStore: WhatsNewSeenVersionStoring {
    static let storageKey = "synara.whatsNewSeenVersion"
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    var seenVersion: String? {
        defaults.string(forKey: Self.storageKey)
    }

    func recordSeenVersion(_ version: String) {
        defaults.set(version, forKey: Self.storageKey)
    }
}

final class InMemoryWhatsNewSeenVersionStore: WhatsNewSeenVersionStoring {
    private(set) var seenVersion: String?

    init(seenVersion: String? = nil) {
        self.seenVersion = seenVersion
    }

    func recordSeenVersion(_ version: String) {
        seenVersion = version
    }
}

/// The card's state for this process. The decision is made once, at launch,
/// from the session restored then; signing in later never shows the card.
/// Dismissing records the version. So does going to the background after
/// the card was on screen, which also takes it away, so it never nags.
final class WhatsNewPresentation: ObservableObject {
    static let shared = WhatsNewPresentation.launch()

    /// The notes the card shows; nil when there is no card.
    @Published private(set) var pending: WhatsNewNotes?
    /// The running version's notes, for Settings › About.
    let currentNotes: WhatsNewNotes?
    private let currentVersion: String?
    private let store: WhatsNewSeenVersionStoring
    private let isEnabled: Bool
    private var didEvaluate = false
    private(set) var wasVisible = false

    init(
        currentVersion: String?,
        notes: WhatsNewNotes?,
        store: WhatsNewSeenVersionStoring,
        isEnabled: Bool = true
    ) {
        self.currentVersion = currentVersion
        self.currentNotes = notes
        self.store = store
        self.isEnabled = isEnabled
    }

    /// UI tests never see the card unless a test asks for it with
    /// `SYNARA_UI_TEST_WHATS_NEW_SEEN_VERSION` (an older version "last
    /// seen"); they never touch the simulator's real defaults.
    static func launch(
        environment: [String: String] = ProcessInfo.processInfo.environment,
        bundle: Bundle = .main
    ) -> WhatsNewPresentation {
        let version = bundle.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
        let notes = version.flatMap { WhatsNewNotesLoader.bundledNotes(version: $0, bundle: bundle) }
        guard environment["SYNARA_UI_TESTS"] == "1" else {
            return WhatsNewPresentation(currentVersion: version, notes: notes, store: UserDefaultsWhatsNewSeenVersionStore())
        }
        let seenVersion = environment["SYNARA_UI_TEST_WHATS_NEW_SEEN_VERSION"]
        return WhatsNewPresentation(
            currentVersion: version,
            notes: notes,
            store: InMemoryWhatsNewSeenVersionStore(seenVersion: seenVersion),
            isEnabled: seenVersion != nil
        )
    }

    /// Runs once per process, when the shell first appears.
    func evaluateAtLaunch(session: WhatsNewSessionState) {
        guard didEvaluate == false else {
            return
        }
        didEvaluate = true
        guard isEnabled, let currentVersion else {
            return
        }
        switch WhatsNewPolicy.decideAtStartup(
            seenVersion: store.seenVersion,
            currentVersion: currentVersion,
            session: session,
            hasNotes: currentNotes != nil
        ) {
        case .record:
            store.recordSeenVersion(currentVersion)
        case .show:
            pending = currentNotes
        case .none:
            break
        }
    }

    /// True on the card's first appearance, when VoiceOver should hear it.
    @discardableResult
    func cardDidAppear() -> Bool {
        guard pending != nil, wasVisible == false else {
            return false
        }
        wasVisible = true
        return true
    }

    func dismiss() {
        guard let notes = pending else {
            return
        }
        store.recordSeenVersion(notes.version)
        pending = nil
    }

    func sceneDidEnterBackground() {
        guard wasVisible else {
            return
        }
        dismiss()
    }
}

extension WhatsNewSessionState {
    init(_ session: AppSessionStore) {
        if case .signedIn = session.currentState {
            self = .signedIn
        } else if session.restoreFailureLogDescription != nil {
            self = .unknown
        } else {
            self = .signedOut
        }
    }
}
