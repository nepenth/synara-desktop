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
            return [Color(synaraHex: "#2a3350"), Color(synaraHex: "#1e2438")]
        }
        if isAgentRoom {
            return [Color(synaraHex: "#163a40"), Color(synaraHex: "#122e33")]
        }
        if isSecureRoom {
            return [Color(synaraHex: "#1c3d32"), Color(synaraHex: "#163129")]
        }
        if name.localizedCaseInsensitiveContains("alert") || name.localizedCaseInsensitiveContains("incident") {
            return [Color(synaraHex: "#3b2b22"), Color(synaraHex: "#30231c")]
        }
        if name.localizedCaseInsensitiveContains("design") || name.localizedCaseInsensitiveContains("creative") {
            return [Color(synaraHex: "#3d2a4d"), Color(synaraHex: "#322240")]
        }
        if name.localizedCaseInsensitiveContains("ops") || name.localizedCaseInsensitiveContains("infra") {
            return [Color(synaraHex: "#163a40"), Color(synaraHex: "#1c3d32")]
        }

        let palettes: [[Color]] = [
            [Color(synaraHex: "#1e3a5f"), Color(synaraHex: "#16304d")],
            [Color(synaraHex: "#163a40"), Color(synaraHex: "#122e33")],
            [Color(synaraHex: "#3d2a4d"), Color(synaraHex: "#322240")],
            [Color(synaraHex: "#1c3d32"), Color(synaraHex: "#163129")],
            [Color(synaraHex: "#3b2b22"), Color(synaraHex: "#30231c")],
            [Color(synaraHex: "#2a3350"), Color(synaraHex: "#222943")]
        ]
        let seed = "\(id)|\(name)".unicodeScalars.reduce(0) { partial, scalar in
            (partial &* 31 &+ Int(scalar.value)) & 0x7fffffff
        }
        let index = seed % palettes.count
        return palettes[index]
    }
}