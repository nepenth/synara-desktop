import Foundation
import SynaraCore

/// One `io.element.recent_emoji` entry.
struct SharedCoreRecentEmojiEntry: Equatable {
    let emoji: String
    let count: UInt64
}

/// Typed recent-emoji read and record-use. Uses an already-constructed
/// SharedCore; the caller owns the core so UniFFI does not free the
/// retained Client. Core applies desktop's rule (front, count + 1, at most
/// 100 entries), so iOS and desktop share one list.
enum SharedCoreRecentEmoji {
    static func snapshot(core: SharedCore) async throws -> [SharedCoreRecentEmojiEntry] {
        entries(from: try await core.recentEmojiSnapshot())
    }

    static func record(core: SharedCore, emoji: String) async throws -> [SharedCoreRecentEmojiEntry] {
        entries(from: try await core.recentEmojiRecord(emoji: emoji))
    }

    static func entries(from dto: RecentEmojiSnapshotDto) -> [SharedCoreRecentEmojiEntry] {
        dto.entries.map { SharedCoreRecentEmojiEntry(emoji: $0.emoji, count: $0.count) }
    }
}

/// The quick-reaction choices in a message's long-press menu: the user's
/// most-used emoji (as desktop's `getRecentEmojis` ranks them), padded with
/// a fixed set. The full picker comes later.
enum TimelineQuickReactions {
    static let limit = 6
    static let fallback = ["👍", "❤️", "😂", "😮", "😢", "🙏"]

    /// Most-used first; ties keep the stored most-recent-first order.
    /// Padded from `fallback`, skipping emoji already listed.
    static func keys(from recent: [SharedCoreRecentEmojiEntry]) -> [String] {
        let ranked = recent.enumerated()
            .sorted { lhs, rhs in
                lhs.element.count != rhs.element.count
                    ? lhs.element.count > rhs.element.count
                    : lhs.offset < rhs.offset
            }
            .map(\.element.emoji)
        var keys: [String] = []
        for key in ranked + fallback where keys.contains(key) == false {
            keys.append(key)
            if keys.count == limit {
                break
            }
        }
        return keys
    }
}

/// The quick-reaction keys for open conversations. Rows observe it, so the
/// menu follows a recorded use without rebuilding timeline rows.
@MainActor
final class TimelineQuickReactionStore: ObservableObject {
    @Published private(set) var keys = TimelineQuickReactions.fallback

    func refresh(load: () async -> [SharedCoreRecentEmojiEntry]?) async {
        if let recent = await load() {
            keys = TimelineQuickReactions.keys(from: recent)
        }
    }

    /// Records a reaction the user added; removing one is not a use.
    func recordUse(_ emoji: String, record: (String) async -> [SharedCoreRecentEmojiEntry]?) async {
        if let recent = await record(emoji) {
            keys = TimelineQuickReactions.keys(from: recent)
        }
    }
}
