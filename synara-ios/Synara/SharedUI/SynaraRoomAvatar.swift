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
            .shadow(color: room.avatarShadow.opacity(0.22), radius: 5, x: 0, y: 2)
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
                RoundedRectangle(cornerRadius: 11, style: .continuous)
                    .fill(room.avatarGradient)
                    .overlay(alignment: .topLeading) {
                        RoundedRectangle(cornerRadius: 11, style: .continuous)
                            .fill(Color.white.opacity(0.18))
                            .blendMode(.softLight)
                    }

                if let systemImage = room.avatarSystemImage {
                    Image(systemName: systemImage)
                        .font(.system(size: size * 0.43, weight: .bold))
                        .foregroundStyle(.white)
                        .symbolRenderingMode(.hierarchical)
                } else {
                    Text(room.avatarInitials)
                        .font(.system(size: size * 0.34, weight: .bold))
                        .foregroundStyle(.white)
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
        if isAgentRoom {
            return "sparkles"
        }
        if isSecureRoom {
            return "lock.fill"
        }
        if name.localizedCaseInsensitiveContains("alert") || name.localizedCaseInsensitiveContains("incident") {
            return "bell.badge.fill"
        }
        if name.localizedCaseInsensitiveContains("ops") || name.localizedCaseInsensitiveContains("infra") {
            return "briefcase.fill"
        }
        if name.localizedCaseInsensitiveContains("design") || name.localizedCaseInsensitiveContains("creative") {
            return "paintpalette.fill"
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

    var avatarGradient: LinearGradient {
        let palette = avatarPalette
        return LinearGradient(colors: palette, startPoint: .topLeading, endPoint: .bottomTrailing)
    }

    var avatarShadow: Color {
        avatarPalette.last ?? SynaraColor.accent
    }

    var isSecureRoom: Bool {
        name.localizedCaseInsensitiveContains("security")
            || name.localizedCaseInsensitiveContains("secure")
            || name.localizedCaseInsensitiveContains("e2e")
    }

    private var avatarPalette: [Color] {
        if kind == .directMessage {
            return [Color(red: 0.35, green: 0.42, blue: 0.53), Color(red: 0.12, green: 0.16, blue: 0.24)]
        }
        if isAgentRoom {
            return [Color(red: 0.58, green: 0.32, blue: 0.94), Color(red: 0.20, green: 0.63, blue: 0.86)]
        }
        if isSecureRoom {
            return [Color(red: 0.05, green: 0.55, blue: 0.43), Color(red: 0.02, green: 0.26, blue: 0.25)]
        }
        if name.localizedCaseInsensitiveContains("alert") || name.localizedCaseInsensitiveContains("incident") {
            return [Color(red: 0.97, green: 0.42, blue: 0.22), Color(red: 0.78, green: 0.12, blue: 0.24)]
        }
        if name.localizedCaseInsensitiveContains("design") || name.localizedCaseInsensitiveContains("creative") {
            return [Color(red: 0.49, green: 0.29, blue: 0.95), Color(red: 0.95, green: 0.32, blue: 0.58)]
        }
        if name.localizedCaseInsensitiveContains("ops") || name.localizedCaseInsensitiveContains("infra") {
            return [Color(red: 0.04, green: 0.48, blue: 0.46), Color(red: 0.05, green: 0.23, blue: 0.38)]
        }

        let palettes: [[Color]] = [
            [Color(red: 0.12, green: 0.45, blue: 0.91), Color(red: 0.26, green: 0.24, blue: 0.77)],
            [Color(red: 0.04, green: 0.58, blue: 0.74), Color(red: 0.02, green: 0.31, blue: 0.58)],
            [Color(red: 0.80, green: 0.25, blue: 0.43), Color(red: 0.48, green: 0.20, blue: 0.74)],
            [Color(red: 0.12, green: 0.60, blue: 0.38), Color(red: 0.08, green: 0.35, blue: 0.42)],
            [Color(red: 0.90, green: 0.45, blue: 0.16), Color(red: 0.68, green: 0.20, blue: 0.34)]
        ]
        let seed = "\(id)|\(name)".unicodeScalars.reduce(0) { partial, scalar in
            (partial &* 31 &+ Int(scalar.value)) & 0x7fffffff
        }
        let index = seed % palettes.count
        return palettes[index]
    }
}