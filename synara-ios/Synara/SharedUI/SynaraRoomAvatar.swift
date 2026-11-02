import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

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

/// The glyph column of a list row: `#` for channels, the person glyph for
/// direct messages, both `SynaraMetrics.roomGlyph` wide so names line up
/// across the Rooms list and the Notifications inbox.
struct SynaraRoomGlyphView: View {
    let room: RoomSummary?
    var tint: Color = SynaraColor.secondaryText

    var body: some View {
        switch SynaraRoomGlyph.glyph(for: room) {
        case .channel:
            Text("#")
                .font(SynaraTypography.body.weight(.semibold))
                .foregroundStyle(tint)
                .frame(width: SynaraMetrics.roomGlyph, alignment: .center)
                .accessibilityHidden(true)
        case let .person(initial):
            SynaraPersonGlyph(
                initial: initial,
                avatarURL: room?.avatarURL,
                size: SynaraMetrics.roomGlyph,
                seed: room?.directUserId ?? room?.id
            )
        }
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
