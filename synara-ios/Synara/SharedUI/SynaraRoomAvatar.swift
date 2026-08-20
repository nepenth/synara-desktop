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