import Combine
import SwiftUI
#if canImport(UIKit)
import UIKit
#endif

enum SynaraSpacing {
    static let xSmall: CGFloat = 4
    static let small: CGFloat = 8
    static let medium: CGFloat = 12
    static let large: CGFloat = 16
    static let xLarge: CGFloat = 24
}

/// The floating system tab bar intentionally floats over edge-to-edge content.
/// A clear scroll tail lets the final actionable row move fully above that glass
/// without painting an opaque footer or duplicating screen-specific estimates.
enum SynaraTabRootContentReachability {
    static let minimumSeparation: CGFloat = SynaraSpacing.small

    static func scrollTailHeight(
        windowBounds: CGRect,
        tabBarFrame: CGRect,
        isVisible: Bool
    ) -> CGFloat {
        guard isVisible else {
            return 0
        }
        let visibleTabBar = tabBarFrame.intersection(windowBounds)
        guard visibleTabBar.isNull == false, visibleTabBar.height > 0 else {
            return 0
        }
        return max(0, windowBounds.maxY - visibleTabBar.minY) + minimumSeparation
    }
}

extension View {
    func synaraTabRootContentReachability(scrollTailHeight: CGFloat) -> some View {
        safeAreaInset(edge: .bottom, spacing: 0) {
            Color.clear
                .frame(height: max(0, scrollTailHeight))
                .accessibilityHidden(true)
        }
    }
}

struct SynaraThemeTokens: Equatable {
    let groupedSurface: String
    let secondarySurface: String
    let surface: String
    let elevatedSurface: String
    let headingText: String
    let primaryText: String
    let secondaryText: String
    let tertiaryText: String
    let separator: String
    let mutedControl: String
    let agentReviewBackground: String
    let agentReviewSurface: String
}

enum SynaraThemeRamp {
    static let defaultBaseHex = "#2b2d31"
    static let storageKey = SynaraSharedConstants.themeBaseColorKey
    static let presets: [(label: String, hex: String)] = [
        ("Graphite", "#2b2d31"),
        ("Blurple", "#5865f2"),
        ("Teal", "#0d9488"),
        ("Slate", "#64748b"),
        ("Amber", "#b45309"),
        ("Rose", "#be123c"),
    ]


    static func normalize(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines), trimmed.isEmpty == false else {
            return nil
        }
        guard trimmed.count == 7,
              trimmed.hasPrefix("#"),
              trimmed.unicodeScalars.dropFirst().allSatisfy({ CharacterSet(charactersIn: "0123456789abcdefABCDEF").contains($0) })
        else {
            return nil
        }
        return trimmed.lowercased()
    }

    static func resolve(_ value: String?) -> String {
        normalize(value) ?? defaultBaseHex
    }

    static func storedBaseHex(
        defaults: UserDefaults = SynaraSharedConstants.appGroupDefaults() ?? .standard
    ) -> String {
        resolve(defaults.string(forKey: storageKey))
    }

    static func persist(
        _ value: String?,
        defaults: UserDefaults = SynaraSharedConstants.appGroupDefaults() ?? .standard
    ) {
        if let hex = normalize(value) {
            defaults.set(hex, forKey: storageKey)
        } else if value == nil {
            defaults.removeObject(forKey: storageKey)
        } else {
            return
        }
        SynaraThemePaint.shared.reload()
    }

    static func tokens(
        baseHex: String,
        dark: Bool,
        increasedContrast: Bool = false
    ) -> SynaraThemeTokens {
        let resolved = resolve(baseHex)
        let hsl = synaraHSL(from: resolved)
        let hue = hsl.hue.isNaN ? 220.0 : hsl.hue

        if dark {
            let saturation = min(0.145, max(0.045, hsl.saturation * 0.45))
            let mixRatio = min(0.22, max(0.1, 0.1 + hsl.saturation * 0.12))
            return SynaraThemeTokens(
                groupedSurface: synaraMix(synaraHex(hue: hue, saturation: saturation, lightness: 0.075), resolved, mixRatio),
                secondarySurface: synaraMix(synaraHex(hue: hue, saturation: saturation * 0.92, lightness: 0.10), resolved, mixRatio),
                surface: synaraMix(synaraHex(hue: hue, saturation: saturation * 0.88, lightness: 0.104), resolved, mixRatio),
                elevatedSurface: synaraMix(synaraHex(hue: hue, saturation: saturation * 0.80, lightness: 0.135), resolved, mixRatio),
                headingText: synaraHex(hue: hue, saturation: saturation * 0.18, lightness: increasedContrast ? 0.99 : 0.97),
                primaryText: synaraHex(hue: hue, saturation: saturation * 0.22, lightness: increasedContrast ? 0.94 : 0.80),
                secondaryText: synaraHex(hue: hue, saturation: saturation * 0.18, lightness: increasedContrast ? 0.80 : 0.66),
                tertiaryText: synaraHex(hue: hue, saturation: saturation * 0.14, lightness: increasedContrast ? 0.72 : 0.60),
                separator: synaraMix(synaraHex(hue: hue, saturation: saturation * 0.80, lightness: 0.31), resolved, mixRatio),
                mutedControl: synaraMix(synaraHex(hue: hue, saturation: saturation * 0.80, lightness: 0.24), resolved, mixRatio),
                agentReviewBackground: synaraHex(hue: hue, saturation: min(0.22, saturation + 0.06), lightness: 0.08),
                agentReviewSurface: synaraHex(hue: hue, saturation: min(0.24, saturation + 0.08), lightness: 0.13)
            )
        }

        // Light appearance needs a much quieter chrome stack than Dark Mode.
        // Mixing the raw theme base into these stops made the default graphite
        // theme look like a translucent gray veil over entire screens. Keep the
        // chosen hue, but build near-white opaque surfaces so hierarchy comes
        // from small luminance steps instead of a dark cast.
        let saturation = min(0.045, max(0.012, hsl.saturation * 0.14))
        return SynaraThemeTokens(
            groupedSurface: synaraHex(hue: hue, saturation: saturation, lightness: 0.955),
            secondarySurface: synaraHex(hue: hue, saturation: saturation * 0.70, lightness: 0.975),
            surface: synaraHex(hue: hue, saturation: saturation * 0.40, lightness: 0.995),
            elevatedSurface: synaraHex(hue: hue, saturation: saturation * 0.55, lightness: 0.985),
            headingText: synaraHex(hue: hue, saturation: min(saturation * 1.4, 0.12), lightness: increasedContrast ? 0.01 : 0.06),
            primaryText: synaraHex(hue: hue, saturation: min(saturation * 1.4, 0.12), lightness: increasedContrast ? 0.04 : 0.12),
            secondaryText: synaraHex(hue: hue, saturation: saturation, lightness: increasedContrast ? 0.20 : 0.34),
            tertiaryText: synaraHex(hue: hue, saturation: saturation, lightness: increasedContrast ? 0.27 : 0.40),
            separator: synaraHex(hue: hue, saturation: saturation * 0.70, lightness: 0.86),
            mutedControl: synaraHex(hue: hue, saturation: saturation * 0.70, lightness: 0.93),
            agentReviewBackground: synaraHex(hue: 196, saturation: 0.12, lightness: 0.94),
            agentReviewSurface: synaraHex(hue: 196, saturation: 0.14, lightness: 0.90)
        )
    }

    static func relativeLuminance(hex: String) -> Double {
        guard let color = synaraRGB(from: hex) else { return 0 }
        func channel(_ value: Double) -> Double {
            value <= 0.03928 ? value / 12.92 : pow((value + 0.055) / 1.055, 2.4)
        }
        return 0.2126 * channel(color.red) + 0.7152 * channel(color.green) + 0.0722 * channel(color.blue)
    }

    static func contrastRatio(foreground: String, background: String) -> Double {
        let first = relativeLuminance(hex: foreground)
        let second = relativeLuminance(hex: background)
        let lighter = max(first, second)
        let darker = min(first, second)
        return (lighter + 0.05) / (darker + 0.05)
    }

    static func colorHex(
        _ keyPath: KeyPath<SynaraThemeTokens, String>,
        baseHex: String,
        dark: Bool
    ) -> String {
        tokens(baseHex: baseHex, dark: dark)[keyPath: keyPath]
    }
}

private struct SynaraThemeBaseHexKey: EnvironmentKey {
    static let defaultValue = SynaraThemeRamp.defaultBaseHex
}

extension EnvironmentValues {
    var synaraThemeBaseHex: String {
        get { self[SynaraThemeBaseHexKey.self] }
        set { self[SynaraThemeBaseHexKey.self] = newValue }
    }
}

final class SynaraThemePaint: ObservableObject {
    static let shared = SynaraThemePaint()

    @Published private(set) var baseHex: String

    private init() {
        baseHex = SynaraThemeRamp.storedBaseHex()
    }

    func reload() {
        let next = SynaraThemeRamp.storedBaseHex()
        if next != baseHex {
            baseHex = next
        }
    }
}

struct SynaraTokenFill: View {
    @Environment(\.synaraThemeBaseHex) private var baseHex
    @Environment(\.colorScheme) private var colorScheme
    let keyPath: KeyPath<SynaraThemeTokens, String>

    init(_ keyPath: KeyPath<SynaraThemeTokens, String>) {
        self.keyPath = keyPath
    }

    var body: some View {
        Color(
            synaraHex: SynaraThemeRamp.colorHex(
                keyPath,
                baseHex: baseHex,
                dark: colorScheme == .dark
            )
        )
    }
}

enum SynaraChrome {
    static let railToken = \SynaraThemeTokens.groupedSurface
    static let roomListToken = \SynaraThemeTokens.secondarySurface
    static let chatToken = \SynaraThemeTokens.surface
    static let composerToken = \SynaraThemeTokens.elevatedSurface
    static let settingsToken = \SynaraThemeTokens.groupedSurface
    static let agentReviewToken = \SynaraThemeTokens.agentReviewBackground

    static var rail: SynaraTokenFill { SynaraTokenFill(railToken) }
    static var roomList: SynaraTokenFill { SynaraTokenFill(roomListToken) }
    static var chat: SynaraTokenFill { SynaraTokenFill(chatToken) }
    static var composer: SynaraTokenFill { SynaraTokenFill(composerToken) }
    static var settings: SynaraTokenFill { SynaraTokenFill(settingsToken) }
    static var agentReview: SynaraTokenFill { SynaraTokenFill(agentReviewToken) }
}

enum SynaraColor {
    static var surface: Color { synaraAdaptive(\.surface) }
    static var secondarySurface: Color { synaraAdaptive(\.secondarySurface) }
    static var elevatedSurface: Color { synaraAdaptive(\.elevatedSurface) }
    static var groupedSurface: Color { synaraAdaptive(\.groupedSurface) }
    static var headingText: Color { synaraAdaptive(\.headingText) }
    static var primaryText: Color { synaraAdaptive(\.primaryText) }
    static var secondaryText: Color { synaraAdaptive(\.secondaryText) }
    static var tertiaryText: Color { synaraAdaptive(\.tertiaryText) }
    static let accent = Color.accentColor
    static let agent = Color(.systemTeal)
    static let success = Color(.systemGreen)
    static let warning = Color(.systemOrange)
    static let critical = Color(.systemRed)
    static var separator: Color { synaraAdaptive(\.separator) }
    static let secure = Color(.systemOrange)
    static let design = Color(.systemPurple)
    static let ops = Color(.systemTeal)
    static var mutedControl: Color { synaraAdaptive(\.mutedControl) }
    static var agentReviewBackground: Color { synaraAdaptive(\.agentReviewBackground) }
    static var agentReviewSurface: Color { synaraAdaptive(\.agentReviewSurface) }
}

#if canImport(UIKit)
extension SynaraThemeRamp {
    static func uiColor(
        _ keyPath: KeyPath<SynaraThemeTokens, String>,
        baseHex: String,
        dark: Bool
    ) -> UIColor {
        UIColor(synaraHex: colorHex(keyPath, baseHex: baseHex, dark: dark)) ?? .systemBackground
    }
}

private func synaraAdaptive(_ keyPath: KeyPath<SynaraThemeTokens, String>) -> Color {
    synaraAdaptive(keyPath, baseHex: SynaraThemePaint.shared.baseHex)
}

private func synaraAdaptive(_ keyPath: KeyPath<SynaraThemeTokens, String>, baseHex: String) -> Color {
    Color(uiColor: UIColor { traits in
        let tokens = SynaraThemeRamp.tokens(
            baseHex: baseHex,
            dark: traits.userInterfaceStyle == .dark,
            increasedContrast: traits.accessibilityContrast == .high
        )
        return UIColor(synaraHex: tokens[keyPath: keyPath]) ?? .systemBackground
    })
}

private extension UIColor {
    convenience init?(synaraHex: String) {
        guard let rgb = synaraRGB(from: synaraHex) else { return nil }
        self.init(red: rgb.red, green: rgb.green, blue: rgb.blue, alpha: 1)
    }
}

extension Color {
    init(synaraHex: String) {
        self.init(uiColor: UIColor(synaraHex: synaraHex) ?? .label)
    }

    func synaraHexString() -> String? {
        let uiColor = UIColor(self)
        let srgb = uiColor.cgColor.converted(
            to: CGColorSpaceCreateDeviceRGB(),
            intent: .defaultIntent,
            options: nil
        ).map(UIColor.init(cgColor:)) ?? uiColor
        var red: CGFloat = 0
        var green: CGFloat = 0
        var blue: CGFloat = 0
        var alpha: CGFloat = 0
        guard srgb.getRed(&red, green: &green, blue: &blue, alpha: &alpha) else {
            return nil
        }
        let clampedRed = min(max(red, 0), 1)
        let clampedGreen = min(max(green, 0), 1)
        let clampedBlue = min(max(blue, 0), 1)
        return String(
            format: "#%02x%02x%02x",
            Int((clampedRed * 255).rounded()),
            Int((clampedGreen * 255).rounded()),
            Int((clampedBlue * 255).rounded())
        )
    }
}
#else
private func synaraAdaptive(_ keyPath: KeyPath<SynaraThemeTokens, String>) -> Color {
    Color(
        synaraHex: SynaraThemeRamp.colorHex(
            keyPath,
            baseHex: SynaraThemePaint.shared.baseHex,
            dark: false
        )
    )
}
#endif

private struct SynaraRGB {
    let red: Double
    let green: Double
    let blue: Double
}

private struct SynaraHSL {
    let hue: Double
    let saturation: Double
    let lightness: Double
}

private func synaraRGB(from hex: String) -> SynaraRGB? {
    var value = hex.trimmingCharacters(in: .whitespacesAndNewlines)
    if value.hasPrefix("#") {
        value.removeFirst()
    }
    guard value.count == 6, let int = UInt32(value, radix: 16) else {
        return nil
    }
    return SynaraRGB(
        red: Double((int >> 16) & 0xff) / 255,
        green: Double((int >> 8) & 0xff) / 255,
        blue: Double(int & 0xff) / 255
    )
}

private func synaraHSL(from hex: String) -> SynaraHSL {
    guard let rgb = synaraRGB(from: hex) else {
        return SynaraHSL(hue: 220, saturation: 0.06, lightness: 0.18)
    }
    let maxChannel = max(rgb.red, rgb.green, rgb.blue)
    let minChannel = min(rgb.red, rgb.green, rgb.blue)
    let lightness = (maxChannel + minChannel) / 2
    let delta = maxChannel - minChannel
    guard delta > 0 else {
        return SynaraHSL(hue: 220, saturation: 0, lightness: lightness)
    }
    let saturation = delta / (1 - abs(2 * lightness - 1))
    let hue: Double
    if maxChannel == rgb.red {
        hue = 60 * (((rgb.green - rgb.blue) / delta).truncatingRemainder(dividingBy: 6))
    } else if maxChannel == rgb.green {
        hue = 60 * (((rgb.blue - rgb.red) / delta) + 2)
    } else {
        hue = 60 * (((rgb.red - rgb.green) / delta) + 4)
    }
    return SynaraHSL(hue: hue < 0 ? hue + 360 : hue, saturation: saturation, lightness: lightness)
}

private func synaraMix(_ left: String, _ right: String, _ ratio: Double) -> String {
    guard let first = synaraRGB(from: left), let second = synaraRGB(from: right) else {
        return left
    }
    let amount = min(max(ratio, 0), 1)
    return String(
        format: "#%02x%02x%02x",
        Int(((first.red * (1 - amount) + second.red * amount) * 255).rounded()),
        Int(((first.green * (1 - amount) + second.green * amount) * 255).rounded()),
        Int(((first.blue * (1 - amount) + second.blue * amount) * 255).rounded())
    )
}

private func synaraHex(hue: Double, saturation: Double, lightness: Double) -> String {
    let clampedSaturation = min(max(saturation, 0), 1)
    let clampedLightness = min(max(lightness, 0), 1)
    let chroma = (1 - abs(2 * clampedLightness - 1)) * clampedSaturation
    let huePrime = (hue.isNaN ? 220 : hue) / 60
    let x = chroma * (1 - abs(huePrime.truncatingRemainder(dividingBy: 2) - 1))
    let match = clampedLightness - chroma / 2
    let rgb: SynaraRGB
    switch huePrime {
    case 0..<1:
        rgb = SynaraRGB(red: chroma, green: x, blue: 0)
    case 1..<2:
        rgb = SynaraRGB(red: x, green: chroma, blue: 0)
    case 2..<3:
        rgb = SynaraRGB(red: 0, green: chroma, blue: x)
    case 3..<4:
        rgb = SynaraRGB(red: 0, green: x, blue: chroma)
    case 4..<5:
        rgb = SynaraRGB(red: x, green: 0, blue: chroma)
    default:
        rgb = SynaraRGB(red: chroma, green: 0, blue: x)
    }
    return String(
        format: "#%02x%02x%02x",
        Int(((rgb.red + match) * 255).rounded()),
        Int(((rgb.green + match) * 255).rounded()),
        Int(((rgb.blue + match) * 255).rounded())
    )
}

enum SynaraRadius {
    static let small: CGFloat = 6
    static let card: CGFloat = 8
    static let control: CGFloat = 8
    static let composer: CGFloat = 22
}

enum SynaraDepthLevel: String, CaseIterable {
    case content
    case layered
    case raised
    case floating
    case avatar
    case critical

    func metrics(
        dark: Bool,
        increasedContrast: Bool,
        reduceTransparency: Bool
    ) -> SynaraDepthMetrics {
        if self == .content {
            return SynaraDepthMetrics(
                shadowOpacity: 0,
                shadowRadius: 0,
                shadowOffsetY: 0,
                edgeOpacity: 0,
                boundaryOpacity: 0,
                boundaryWidth: 0
            )
        }

        if increasedContrast {
            switch self {
            case .content:
                preconditionFailure("Content depth is handled before increased-contrast metrics")
            case .layered:
                return SynaraDepthMetrics(
                    shadowOpacity: 0,
                    shadowRadius: 0,
                    shadowOffsetY: 0,
                    edgeOpacity: 0,
                    boundaryOpacity: 0.78,
                    boundaryWidth: 0.8
                )
            case .avatar, .raised:
                return SynaraDepthMetrics(
                    shadowOpacity: 0,
                    shadowRadius: 0,
                    shadowOffsetY: 0,
                    edgeOpacity: 0,
                    boundaryOpacity: 0.88,
                    boundaryWidth: 1
                )
            case .floating:
                return SynaraDepthMetrics(
                    shadowOpacity: 0,
                    shadowRadius: 0,
                    shadowOffsetY: 0,
                    edgeOpacity: 0,
                    boundaryOpacity: 0.96,
                    boundaryWidth: 1.25
                )
            case .critical:
                return SynaraDepthMetrics(
                    shadowOpacity: 0,
                    shadowRadius: 0,
                    shadowOffsetY: 0,
                    edgeOpacity: 0,
                    boundaryOpacity: 1,
                    boundaryWidth: 1.5
                )
            }
        }

        let transparencyScale = reduceTransparency ? 0.72 : 1.0
        let boundaryScale = reduceTransparency ? 1.18 : 1.0
        switch self {
        case .content:
            preconditionFailure("Content depth is handled before standard metrics")
        case .layered:
            return SynaraDepthMetrics(
                shadowOpacity: (dark ? 0.24 : 0.055) * transparencyScale,
                shadowRadius: dark ? 3 : 2,
                shadowOffsetY: 1,
                edgeOpacity: dark ? 0.13 : 0.12,
                boundaryOpacity: (dark ? 0.46 : 0.26) * boundaryScale,
                boundaryWidth: 0.6
            )
        case .raised:
            return SynaraDepthMetrics(
                shadowOpacity: (dark ? 0.30 : 0.08) * transparencyScale,
                shadowRadius: dark ? 7 : 5,
                shadowOffsetY: dark ? 2 : 1.5,
                edgeOpacity: dark ? 0.17 : 0.14,
                boundaryOpacity: (dark ? 0.52 : 0.30) * boundaryScale,
                boundaryWidth: 0.7
            )
        case .floating:
            return SynaraDepthMetrics(
                shadowOpacity: (dark ? 0.38 : 0.12) * transparencyScale,
                shadowRadius: dark ? 14 : 11,
                shadowOffsetY: dark ? 5 : 4,
                edgeOpacity: dark ? 0.21 : 0.18,
                boundaryOpacity: (dark ? 0.60 : 0.36) * boundaryScale,
                boundaryWidth: 0.8
            )
        case .avatar:
            return SynaraDepthMetrics(
                shadowOpacity: (dark ? 0.30 : 0.08) * transparencyScale,
                shadowRadius: dark ? 5 : 4,
                shadowOffsetY: dark ? 2 : 1.5,
                edgeOpacity: dark ? 0.18 : 0.12,
                boundaryOpacity: (dark ? 0.50 : 0.28) * boundaryScale,
                boundaryWidth: 0.8
            )
        case .critical:
            return SynaraDepthMetrics(
                shadowOpacity: (dark ? 0.44 : 0.16) * transparencyScale,
                shadowRadius: dark ? 18 : 15,
                shadowOffsetY: dark ? 7 : 5,
                edgeOpacity: dark ? 0.24 : 0.22,
                boundaryOpacity: (dark ? 0.68 : 0.44) * boundaryScale,
                boundaryWidth: 0.9
            )
        }
    }
}

/// Semantic depth roles keep core surfaces consistent when visual metrics evolve.
enum SynaraSurfaceDepthRole {
    static let roomRow: SynaraDepthLevel = .layered
    static let standardMessage: SynaraDepthLevel = .layered
    static let emphasizedMessage: SynaraDepthLevel = .raised
    static let standardMessageShowsBackground = true
}

struct SynaraDepthMetrics: Equatable {
    let shadowOpacity: Double
    let shadowRadius: CGFloat
    let shadowOffsetY: CGFloat
    let edgeOpacity: Double
    let boundaryOpacity: Double
    let boundaryWidth: CGFloat
}

private struct SynaraDepthModifier<Outline: Shape>: ViewModifier {
    let level: SynaraDepthLevel
    let outline: Outline
    let boundaryColor: Color
    let shadowVerticalDirection: CGFloat
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    @ViewBuilder
    func body(content: Content) -> some View {
        if level == .content {
            content
        } else {
            let metrics = level.metrics(
                dark: colorScheme == .dark,
                increasedContrast: colorSchemeContrast == .increased,
                reduceTransparency: reduceTransparency
            )

            content
                .overlay {
                    outline
                        .stroke(boundaryColor.opacity(metrics.boundaryOpacity), lineWidth: metrics.boundaryWidth)
                        .allowsHitTesting(false)
                }
                .overlay {
                    outline
                        .stroke(
                            LinearGradient(
                                colors: [
                                    Color.white.opacity(metrics.edgeOpacity),
                                    Color.clear,
                                    Color.black.opacity(metrics.edgeOpacity * 0.72),
                                ],
                                startPoint: .top,
                                endPoint: .bottom
                            ),
                            lineWidth: max(0.75, metrics.boundaryWidth)
                        )
                        .allowsHitTesting(false)
                }
                .shadow(
                    color: Color.black.opacity(metrics.shadowOpacity),
                    radius: metrics.shadowRadius,
                    x: 0,
                    y: metrics.shadowOffsetY * shadowVerticalDirection
                )
        }
    }
}

private struct SynaraDockedDepthModifier: ViewModifier {
    let level: SynaraDepthLevel
    let boundaryColor: Color
    @Environment(\.colorScheme) private var colorScheme
    @Environment(\.colorSchemeContrast) private var colorSchemeContrast
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    @ViewBuilder
    func body(content: Content) -> some View {
        if level == .content {
            content
        } else {
            let metrics = level.metrics(
                dark: colorScheme == .dark,
                increasedContrast: colorSchemeContrast == .increased,
                reduceTransparency: reduceTransparency
            )

            content
                .overlay(alignment: .top) {
                    Rectangle()
                        .fill(boundaryColor.opacity(metrics.boundaryOpacity))
                        .frame(height: metrics.boundaryWidth)
                        .allowsHitTesting(false)
                }
                .overlay(alignment: .top) {
                    Rectangle()
                        .fill(Color.white.opacity(metrics.edgeOpacity))
                        .frame(height: 0.5)
                        .allowsHitTesting(false)
                }
                .shadow(
                    color: Color.black.opacity(metrics.shadowOpacity),
                    radius: metrics.shadowRadius,
                    x: 0,
                    y: -metrics.shadowOffsetY
                )
        }
    }
}

struct SynaraTactileButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed && reduceMotion == false ? 0.985 : 1)
            .offset(y: configuration.isPressed && reduceMotion == false ? 1 : 0)
            .opacity(configuration.isPressed ? 0.88 : 1)
            .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: configuration.isPressed)
    }
}

private struct SynaraAccessibleSurfaceFillModifier: ViewModifier {
    let standardFill: Color
    let opaqueFill: Color
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    func body(content: Content) -> some View {
        if reduceTransparency {
            content.background {
                ZStack {
                    opaqueFill
                    standardFill
                }
            }
        } else {
            content.background(standardFill)
        }
    }
}

extension View {
    /// Preserves a surface's semantic color while replacing alpha-composited
    /// fills with an opaque plane when Reduce Transparency is enabled.
    func synaraAccessibleSurfaceFill(
        _ standardFill: Color,
        opaqueFill: Color
    ) -> some View {
        modifier(
            SynaraAccessibleSurfaceFillModifier(
                standardFill: standardFill,
                opaqueFill: opaqueFill
            )
        )
    }

    func synaraDepth<Outline: Shape>(
        _ level: SynaraDepthLevel,
        shape: Outline,
        boundaryColor: Color = SynaraColor.separator,
        shadowVerticalDirection: CGFloat = 1
    ) -> some View {
        modifier(
            SynaraDepthModifier(
                level: level,
                outline: shape,
                boundaryColor: boundaryColor,
                shadowVerticalDirection: shadowVerticalDirection
            )
        )
    }

    func synaraDepth(
        _ level: SynaraDepthLevel,
        cornerRadius: CGFloat,
        boundaryColor: Color = SynaraColor.separator,
        shadowVerticalDirection: CGFloat = 1
    ) -> some View {
        synaraDepth(
            level,
            shape: RoundedRectangle(cornerRadius: cornerRadius, style: .continuous),
            boundaryColor: boundaryColor,
            shadowVerticalDirection: shadowVerticalDirection
        )
    }

    /// Adds upward occlusion and a top separator to a full-width bottom
    /// surface without outlining the screen edges like a floating card.
    func synaraDockedDepth(
        _ level: SynaraDepthLevel,
        boundaryColor: Color = SynaraColor.separator
    ) -> some View {
        modifier(SynaraDockedDepthModifier(level: level, boundaryColor: boundaryColor))
    }
}

enum SynaraTypography {
    static let screenTitle = Font.title2.weight(.semibold)
    static let sectionTitle = Font.headline
    static let body = Font.body
    static let supporting = Font.callout
    static let emphasis = Font.body.weight(.semibold)
    static let messageBody = Font.body
    static let messageMeta = Font.caption
    static let roomPreview = Font.subheadline
    static let chipLabel = Font.caption.weight(.semibold)
    static let fineMeta = Font.caption2
    static let fineMetaBold = Font.caption2.weight(.bold)
    static let monoBody = Font.system(.callout, design: .monospaced)
    static let composerPlaceholder = Font.callout
    static let composerMetric = Font.caption2
}

struct SynaraAvatar: View {
    let title: String
    let systemImage: String?
    let tint: Color
    var size: CGFloat

    init(title: String, systemImage: String? = nil, tint: Color = SynaraColor.accent, size: CGFloat = 38) {
        self.title = title
        self.systemImage = systemImage
        self.tint = tint
        self.size = size
    }

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)

        ZStack {
            shape
                .fill(tint.opacity(0.18))

            if let systemImage {
                Image(systemName: systemImage)
                    .font(.system(size: size * 0.44, weight: .semibold))
                    .foregroundStyle(tint)
            } else {
                Text(initials)
                    .font(.system(size: size * 0.34, weight: .semibold))
                    .foregroundStyle(tint)
                    .minimumScaleFactor(0.7)
            }
        }
        .frame(width: size, height: size)
        .clipShape(shape)
        .synaraDepth(.avatar, shape: shape)
        .accessibilityHidden(true)
    }

    private var initials: String {
        let trimmed = title.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return "#"
        }

        let words = trimmed.replacingOccurrences(of: "#", with: "").split(separator: " ")
        if words.count >= 2 {
            return words.prefix(2).compactMap(\.first).map(String.init).joined().uppercased()
        }

        return String(trimmed.prefix(2)).uppercased()
    }
}

struct SynaraBrandMark: View {
    var assetName = "SynaraLogo"
    var size: CGFloat = 48
    var hasBackground = true

    var body: some View {
        Image(assetName)
            .resizable()
            .interpolation(.high)
            .scaledToFit()
            .padding(size * 0.12)
            .frame(width: size, height: size)
            .background {
                if hasBackground {
                    RoundedRectangle(cornerRadius: SynaraRadius.card)
                        .fill(SynaraColor.secondarySurface)
                }
            }
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card))
            .accessibilityHidden(true)
    }
}

struct SynaraStatusChip: View {
    let title: String
    let tint: Color
    var systemImage: String?

    var body: some View {
        Label {
            Text(title)
                .font(SynaraTypography.chipLabel)
                .lineLimit(1)
                .minimumScaleFactor(0.8)
        } icon: {
            if let systemImage {
                Image(systemName: systemImage)
                    .font(SynaraTypography.chipLabel)
            }
        }
        .labelStyle(.titleAndIcon)
        .padding(.horizontal, SynaraSpacing.small)
        .padding(.vertical, SynaraSpacing.xSmall)
        .background(tint.opacity(0.14))
        .foregroundStyle(tint)
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.small))
    }
}

struct SynaraUnreadBadge: View {
    let count: Int
    let highlighted: Bool

    var body: some View {
        if count > 0 {
            Text(count > 99 ? "99+" : "\(count)")
                .font(SynaraTypography.fineMetaBold)
                .monospacedDigit()
                .padding(.horizontal, SynaraSpacing.small)
                .frame(minWidth: 24, minHeight: 20)
                .background(highlighted ? SynaraColor.accent : SynaraColor.accent.opacity(0.16))
                .foregroundStyle(highlighted ? Color.white : SynaraColor.accent)
                .clipShape(Capsule())
                .overlay {
                    Capsule()
                        .stroke(
                            highlighted ? Color.clear : SynaraColor.accent.opacity(0.24),
                            lineWidth: 0.5
                        )
                        .allowsHitTesting(false)
                }
                .accessibilityLabel("\(count) unread")
        }
    }
}

struct SynaraListRowButtonStyle: ButtonStyle {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .background(
                RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                    .fill(configuration.isPressed ? SynaraColor.secondaryText.opacity(0.10) : Color.clear)
            )
            .scaleEffect(configuration.isPressed && reduceMotion == false ? 0.995 : 1)
            .animation(reduceMotion ? nil : .easeOut(duration: 0.14), value: configuration.isPressed)
    }
}

struct SynaraFilterChip: View {
    let title: String
    var badgeCount: Int? = nil
    let isSelected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(SynaraTypography.chipLabel.weight(.medium))
                    .lineLimit(1)

                if let badgeCount, badgeCount > 0 {
                    Text(badgeCount > 99 ? "99+" : "\(badgeCount)")
                        .font(SynaraTypography.fineMetaBold)
                        .monospacedDigit()
                        .padding(.horizontal, SynaraSpacing.xSmall)
                        .frame(minWidth: 18, minHeight: 18)
                        .background(isSelected ? Color.white.opacity(0.22) : SynaraColor.accent.opacity(0.14))
                        .foregroundStyle(isSelected ? Color.white : SynaraColor.accent)
                        .clipShape(Capsule())
                }
            }
            .padding(.horizontal, SynaraSpacing.medium)
            .frame(height: 32)
            .background(isSelected ? SynaraColor.accent : SynaraColor.secondarySurface)
            .foregroundStyle(isSelected ? Color.white : SynaraColor.secondaryText)
            .clipShape(Capsule())
            .overlay {
                if isSelected == false {
                    Capsule()
                        .stroke(SynaraColor.separator.opacity(0.55), lineWidth: 0.5)
                        .allowsHitTesting(false)
                }
            }
            .synaraDepth(isSelected ? .raised : .content, cornerRadius: 16)
        }
        .buttonStyle(SynaraTactileButtonStyle())
        .accessibilityLabel(accessibilityLabel)
        .accessibilityAddTraits(isSelected ? .isSelected : [])
    }

    private var accessibilityLabel: String {
        if let badgeCount, badgeCount > 0 {
            return "\(title), \(badgeCount) unread"
        }
        return title
    }
}

struct SynaraActionIconButton: View {
    let systemImage: String
    let accessibilityLabel: String
    let tint: Color
    let action: () -> Void

    init(
        systemImage: String,
        accessibilityLabel: String,
        tint: Color = SynaraColor.accent,
        action: @escaping () -> Void
    ) {
        self.systemImage = systemImage
        self.accessibilityLabel = accessibilityLabel
        self.tint = tint
        self.action = action
    }

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)

        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 17, weight: .semibold))
                .frame(width: 44, height: 44)
                .background(tint.opacity(0.12))
                .foregroundStyle(tint)
                .clipShape(shape)
                .synaraDepth(.raised, shape: shape)
        }
        .buttonStyle(SynaraTactileButtonStyle())
        .contentShape(Rectangle())
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

struct SynaraIconTile: View {
    let title: String
    let systemImage: String
    let tint: Color
    var size: CGFloat = 46

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: SynaraRadius.card)
                .fill(tint)
            Image(systemName: systemImage)
                .font(.system(size: size * 0.44, weight: .semibold))
                .foregroundStyle(.white)
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

struct SynaraProductHeader: View {
    let title: String
    let subtitle: String
    var systemImage: String = "sparkles"
    var assetName: String = "SynaraLogo"

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.medium) {
            SynaraBrandMark(assetName: assetName, size: 56)
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(.title2.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                Text(subtitle)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(nil)
            }
        }
        .padding(.vertical, SynaraSpacing.small)
        .accessibilityElement(children: .combine)
    }
}

struct SynaraCardModifier: ViewModifier {
    var fill: Color = SynaraColor.secondarySurface
    var opaqueFill: Color = SynaraColor.secondarySurface
    var stroke: Color = SynaraColor.separator
    var depth: SynaraDepthLevel = .raised

    func body(content: Content) -> some View {
        let shape = RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)

        content
            .synaraAccessibleSurfaceFill(fill, opaqueFill: opaqueFill)
            .clipShape(shape)
            .synaraDepth(depth, shape: shape, boundaryColor: stroke)
    }
}

extension View {
    func synaraCard(
        fill: Color = SynaraColor.secondarySurface,
        opaqueFill: Color = SynaraColor.secondarySurface,
        stroke: Color = SynaraColor.separator,
        depth: SynaraDepthLevel = .raised
    ) -> some View {
        modifier(
            SynaraCardModifier(
                fill: fill,
                opaqueFill: opaqueFill,
                stroke: stroke,
                depth: depth
            )
        )
    }
}

struct SynaraEmptyState: View {
    let title: String
    let systemImage: String
    let message: String?

    init(title: String, systemImage: String, message: String? = nil) {
        self.title = title
        self.systemImage = systemImage
        self.message = message
    }

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            Image(systemName: systemImage)
                .font(.system(size: 42, weight: .semibold))
                .foregroundStyle(.secondary)
                .accessibilityHidden(true)

            VStack(spacing: SynaraSpacing.small) {
                Text(title)
                    .font(SynaraTypography.screenTitle)
                    .foregroundStyle(SynaraColor.primaryText)
                    .multilineTextAlignment(.center)
                    .lineLimit(nil)

                if let message {
                    Text(message)
                        .font(SynaraTypography.supporting)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .multilineTextAlignment(.center)
                        .lineLimit(nil)
                }
            }
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(SynaraColor.surface)
    }
}

struct SynaraLoadingState: View {
    let title: String

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            ProgressView()
            Text(title)
                .font(SynaraTypography.body)
                .foregroundStyle(SynaraColor.secondaryText)
                .multilineTextAlignment(.center)
        }
        .padding(SynaraSpacing.xLarge)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct SynaraShimmerModifier: ViewModifier {
    @State private var phase: CGFloat = -1

    func body(content: Content) -> some View {
        content
            .overlay {
                GeometryReader { geometry in
                    LinearGradient(
                        colors: [
                            .clear,
                            SynaraColor.elevatedSurface.opacity(0.75),
                            .clear
                        ],
                        startPoint: .leading,
                        endPoint: .trailing
                    )
                    .frame(width: geometry.size.width * 1.8)
                    .offset(x: geometry.size.width * phase)
                }
                .mask(content)
            }
            .onAppear {
                withAnimation(.linear(duration: 1.15).repeatForever(autoreverses: false)) {
                    phase = 1.8
                }
            }
    }
}

struct SynaraSkeletonBlock: View {
    var width: CGFloat? = nil
    var height: CGFloat
    var cornerRadius: CGFloat = SynaraRadius.small

    var body: some View {
        RoundedRectangle(cornerRadius: cornerRadius, style: .continuous)
            .fill(SynaraColor.mutedControl.opacity(0.65))
            .frame(width: width, height: height)
            .modifier(SynaraShimmerModifier())
            .accessibilityHidden(true)
    }
}

struct SynaraSkeletonRow: View {
    var showsAvatar = true
    var titleWidth: CGFloat = 148
    var subtitleWidth: CGFloat = 212

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            if showsAvatar {
                SynaraSkeletonBlock(width: 42, height: 42, cornerRadius: 11)
            }

            VStack(alignment: .leading, spacing: SynaraSpacing.small) {
                SynaraSkeletonBlock(width: titleWidth, height: 14)
                SynaraSkeletonBlock(width: subtitleWidth, height: 12)
            }

            Spacer(minLength: SynaraSpacing.small)

            SynaraSkeletonBlock(width: 28, height: 20, cornerRadius: 10)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
        .accessibilityLabel("Loading")
        .accessibilityIdentifier("SynaraSkeletonRow")
    }
}

struct SynaraSkeletonList: View {
    let rowCount: Int
    var showsAvatar = true

    var body: some View {
        VStack(spacing: SynaraSpacing.small) {
            ForEach(0..<rowCount, id: \.self) { index in
                SynaraSkeletonRow(
                    showsAvatar: showsAvatar,
                    titleWidth: index.isMultiple(of: 2) ? 148 : 124,
                    subtitleWidth: index.isMultiple(of: 3) ? 196 : 228
                )
            }
        }
        .accessibilityIdentifier("SynaraSkeletonList")
    }
}

struct SynaraTimelineSkeletonRow: View {
    var isOutgoing = false
    var lineCount = 2

    var body: some View {
        HStack(alignment: .top, spacing: SynaraSpacing.small) {
            if isOutgoing {
                Spacer(minLength: 48)
            } else {
                SynaraSkeletonBlock(width: 34, height: 34, cornerRadius: SynaraRadius.card)
            }

            VStack(alignment: isOutgoing ? .trailing : .leading, spacing: SynaraSpacing.small) {
                if isOutgoing == false {
                    SynaraSkeletonBlock(width: 92, height: 12)
                }

                VStack(alignment: isOutgoing ? .trailing : .leading, spacing: SynaraSpacing.xSmall) {
                    ForEach(0..<lineCount, id: \.self) { line in
                        SynaraSkeletonBlock(
                            width: line == lineCount - 1 ? 132 : 196,
                            height: 12
                        )
                    }
                }
            }
            .frame(maxWidth: .infinity, alignment: isOutgoing ? .trailing : .leading)

            if isOutgoing {
                SynaraSkeletonBlock(width: 34, height: 34, cornerRadius: SynaraRadius.card)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
        .accessibilityLabel("Loading message")
        .accessibilityIdentifier("SynaraTimelineSkeletonRow")
    }
}

struct SynaraTimelineSkeletonList: View {
    let rowCount: Int

    var body: some View {
        VStack(spacing: SynaraSpacing.medium) {
            ForEach(0..<rowCount, id: \.self) { index in
                SynaraTimelineSkeletonRow(
                    isOutgoing: index.isMultiple(of: 3),
                    lineCount: index.isMultiple(of: 2) ? 1 : 2
                )
            }
        }
        .accessibilityIdentifier("SynaraTimelineSkeletonList")
    }
}

struct SynaraErrorState: View {
    let title: String
    let message: String
    let retry: (() -> Void)?

    var body: some View {
        VStack(spacing: SynaraSpacing.large) {
            SynaraEmptyState(title: title, systemImage: "exclamationmark.triangle", message: message)

            if let retry {
                Button("Retry", action: retry)
                    .buttonStyle(.borderedProminent)
            }
        }
    }
}

struct SynaraToolbarIconButton: View {
    let systemImage: String
    let accessibilityLabel: String
    let action: () -> Void

    var body: some View {
        let shape = RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)

        Button(action: action) {
            Image(systemName: systemImage)
                .frame(width: 28, height: 28)
                .background(SynaraColor.elevatedSurface)
                .clipShape(shape)
                .synaraDepth(.raised, shape: shape)
        }
        .buttonStyle(SynaraTactileButtonStyle())
        .accessibilityLabel(Text(accessibilityLabel))
    }
}

struct SynaraDesignTokenGallery: View {
    var body: some View {
        NavigationStack {
            List {
                Section("States") {
                    SynaraEmptyState(title: "No Rooms", systemImage: "bubble.left.and.bubble.right", message: "Joined rooms will appear here.")
                        .frame(minHeight: 220)
                    SynaraLoadingState(title: "Syncing rooms")
                        .frame(minHeight: 120)
                    SynaraSkeletonList(rowCount: 3)
                        .padding(.vertical, SynaraSpacing.small)
                }

                Section("Controls") {
                    HStack(spacing: SynaraSpacing.medium) {
                        SynaraToolbarIconButton(systemImage: "magnifyingglass", accessibilityLabel: "Search") {}
                        SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Account") {}
                        SynaraActionIconButton(systemImage: "paperplane.fill", accessibilityLabel: "Send") {}
                    }
                    SynaraStatusChip(title: "Agent", tint: SynaraColor.agent, systemImage: "sparkles")
                    SynaraAvatar(title: "Project Room")
                    SynaraUnreadBadge(count: 12, highlighted: true)
                }
            }
            .navigationTitle("Design Tokens")
        }
    }
}

struct SynaraDesignTokenGallery_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            SynaraDesignTokenGallery()
                .preferredColorScheme(.light)
                .previewDisplayName("Light")

            SynaraDesignTokenGallery()
                .preferredColorScheme(.dark)
                .previewDisplayName("Dark")
        }
    }
}
