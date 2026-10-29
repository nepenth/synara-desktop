import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

struct SynaraRoomAvatarTile: View {
    let room: RoomSummary
    let size: CGFloat
    @Environment(\.appEnvironment) private var environment
    @State private var avatarImage: UIImage?

    var body: some View {
        avatarContent
            .frame(width: size, height: size)
            .task(id: avatarTaskID) {
                await loadAvatar()
            }
            .accessibilityHidden(true)
    }

    @ViewBuilder
    private var avatarContent: some View {
        if let avatarImage {
            Image(uiImage: avatarImage)
                .resizable()
                .scaledToFill()
                .frame(width: size, height: size)
                .clipShape(RoundedRectangle(cornerRadius: 11, style: .continuous))
        } else {
            ZStack {
                RoundedRectangle(cornerRadius: size * 0.28, style: .continuous)
                    .fill(SynaraColor.mutedControl)

                if let systemImage = room.avatarSystemImage {
                    Image(systemName: systemImage)
                        .font(.system(size: size * 0.4, weight: .medium))
                        .foregroundStyle(SynaraColor.secondaryText)
                        .symbolRenderingMode(.hierarchical)
                } else {
                    Text(room.avatarInitials)
                        .font(.system(size: size * 0.32, weight: .medium))
                        .foregroundStyle(SynaraColor.primaryText)
                        .minimumScaleFactor(0.72)
                }
            }
        }
    }

    @MainActor
    private func loadAvatar() async {
        avatarImage = nil

        guard let avatarURL = room.avatarURL,
              avatarURL.scheme == "mxc" else {
            return
        }

        let resource = MediaResource(
            id: avatarURL.absoluteString,
            filename: "\(room.id)-avatar",
            authenticatedURL: avatarURL,
            requiresAuthentication: true
        )
        if let data = await environment.mediaLoader.loadThumbnailData(
            for: resource,
            width: UInt64(max(1, Int(size * 3))),
            height: UInt64(max(1, Int(size * 3)))
        ),
           let image = UIImage(data: data) {
            avatarImage = image
        }
    }

    private var avatarTaskID: String {
        "\(room.id)|\(room.avatarURL?.absoluteString ?? "profile")"
    }
}

/// The glyph in front of a room's name in the room list and the
/// conversation header: "#" for channels, the person for direct messages.
enum SynaraRoomGlyph: Equatable {
    case channel
    case person(initial: String)

    static func glyph(for room: RoomSummary?) -> SynaraRoomGlyph {
        guard let room, room.kind == .directMessage else {
            return .channel
        }
        return .person(initial: initial(for: room.name, fallbackID: room.directUserId))
    }

    /// First letter or digit of the name, else of the peer's ID localpart.
    static func initial(for name: String, fallbackID: String?) -> String {
        let candidates = [name, fallbackID ?? ""]
        for candidate in candidates {
            if let character = candidate.first(where: { $0.isLetter || $0.isNumber }) {
                return String(character).uppercased()
            }
        }
        return "?"
    }
}

/// A direct message's person glyph: the room avatar (the peer's) when it
/// loads, else their initial, drawn like the timeline's sender avatars.
struct SynaraPersonGlyph: View {
    let initial: String
    let avatarURL: URL?
    let size: CGFloat
    /// Picks the fallback colour; the peer's Matrix ID when known.
    var seed: String? = nil
    @Environment(\.appEnvironment) private var environment
    @State private var avatarImage: UIImage?

    var body: some View {
        Group {
            if let avatarImage {
                Image(uiImage: avatarImage)
                    .resizable()
                    .scaledToFill()
            } else {
                Circle()
                    .fill(SynaraAvatarPalette.color(for: seed ?? initial))
                    .overlay {
                        Text(initial)
                            .font(.system(size: size * 0.5, weight: .semibold))
                            .foregroundStyle(Color.white)
                            .minimumScaleFactor(0.7)
                    }
            }
        }
        .frame(width: size, height: size)
        .clipShape(Circle())
        .task(id: avatarURL?.absoluteString) {
            await loadAvatar()
        }
        .accessibilityHidden(true)
    }

    @MainActor
    private func loadAvatar() async {
        avatarImage = nil
        guard let avatarURL, avatarURL.scheme == "mxc" else {
            return
        }
        let resource = MediaResource(
            id: avatarURL.absoluteString,
            filename: "dm-avatar",
            authenticatedURL: avatarURL,
            requiresAuthentication: true
        )
        if let data = await environment.mediaLoader.loadThumbnailData(
            for: resource,
            width: UInt64(max(1, Int(size * 3))),
            height: UInt64(max(1, Int(size * 3)))
        ),
           let image = UIImage(data: data) {
            avatarImage = image
        }
    }
}

extension RoomSummary {
    var avatarSystemImage: String? {
        if kind == .directMessage {
            return "person.fill"
        }
        return nil
    }

    var avatarInitials: String {
        let cleaned = name
            .replacingOccurrences(of: "#", with: " ")
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: "-", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        let ignoredWords: Set<String> = ["the", "and", "room", "channel", "chat"]
        let words = cleaned
            .split(separator: " ")
            .map(String.init)
            .filter { ignoredWords.contains($0.lowercased()) == false }

        if words.count >= 2 {
            return words.prefix(2).compactMap(\.first).map(String.init).joined().uppercased()
        }

        if let word = words.first, word.count > 1 {
            let letters = word.filter(\.isLetter)
            let first = letters.first.map(String.init) ?? String(word.prefix(1))
            let second = letters.dropFirst().first(where: { !"AEIOUaeiou".contains($0) }).map(String.init)
                ?? letters.dropFirst().first.map(String.init)
                ?? ""
            return "\(first)\(second)".uppercased()
        }

        return cleaned.first.map { String($0).uppercased() } ?? "S"
    }

    var isSecureRoom: Bool {
        name.localizedCaseInsensitiveContains("security")
            || name.localizedCaseInsensitiveContains("secure")
            || name.localizedCaseInsensitiveContains("e2e")
    }
}