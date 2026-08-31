#!/usr/bin/env swift

import AppKit
import Foundation

let fileManager = FileManager.default
let root = URL(fileURLWithPath: fileManager.currentDirectoryPath, isDirectory: true)
let brandingDirectory = root.appendingPathComponent("assets/branding", isDirectory: true)
let desktopIconDirectory = root.appendingPathComponent("src-tauri/icons", isDirectory: true)
let iosIconDirectory = root.appendingPathComponent(
    "synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset",
    isDirectory: true
)

func loadImage(_ url: URL) throws -> NSImage {
    guard let image = NSImage(contentsOf: url) else {
        throw NSError(
            domain: "SynaraIconGenerator",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Unable to load \(url.path)"]
        )
    }
    return image
}

let master = try loadImage(brandingDirectory.appendingPathComponent("synara-app-icon-master.png"))
let small = try loadImage(brandingDirectory.appendingPathComponent("synara-app-icon-small.png"))

func render(
    _ image: NSImage,
    size: Int,
    roundedDesktop: Bool,
    opaque: Bool
) throws -> Data {
    guard let colorSpace = CGColorSpace(name: CGColorSpace.sRGB) else {
        throw NSError(
            domain: "SynaraIconGenerator",
            code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Unable to create sRGB color space"]
        )
    }
    let alphaInfo = opaque ? CGImageAlphaInfo.noneSkipLast : CGImageAlphaInfo.premultipliedLast
    let bitmapInfo = CGBitmapInfo(rawValue: alphaInfo.rawValue)
        .union(.byteOrder32Big)
    guard let cgContext = CGContext(
        data: nil,
        width: size,
        height: size,
        bitsPerComponent: 8,
        bytesPerRow: size * 4,
        space: colorSpace,
        bitmapInfo: bitmapInfo.rawValue
    ) else {
        throw NSError(
            domain: "SynaraIconGenerator",
            code: 3,
            userInfo: [NSLocalizedDescriptionKey: "Unable to allocate \(size)×\(size) bitmap"]
        )
    }
    let context = NSGraphicsContext(cgContext: cgContext, flipped: false)

    NSGraphicsContext.saveGraphicsState()
    NSGraphicsContext.current = context
    context.imageInterpolation = NSImageInterpolation.high

    let rect = NSRect(x: 0, y: 0, width: size, height: size)
    if !opaque {
        NSColor.clear.setFill()
        rect.fill()
    }
    if roundedDesktop {
        let radius = CGFloat(size) * 0.225
        NSBezierPath(roundedRect: rect, xRadius: radius, yRadius: radius).addClip()
    }
    image.draw(in: rect, from: .zero, operation: .copy, fraction: 1)

    context.flushGraphics()
    NSGraphicsContext.restoreGraphicsState()

    guard let cgImage = cgContext.makeImage() else {
        throw NSError(
            domain: "SynaraIconGenerator",
            code: 4,
            userInfo: [NSLocalizedDescriptionKey: "Unable to create \(size)×\(size) CGImage"]
        )
    }
    let bitmap = NSBitmapImageRep(cgImage: cgImage)
    guard let data = bitmap.representation(
        using: NSBitmapImageRep.FileType.png,
        properties: [NSBitmapImageRep.PropertyKey.compressionFactor: 0.9]
    ) else {
        throw NSError(
            domain: "SynaraIconGenerator",
            code: 5,
            userInfo: [NSLocalizedDescriptionKey: "Unable to encode \(size)×\(size) PNG"]
        )
    }
    return data
}

func source(for size: Int) -> NSImage {
    size <= 87 ? small : master
}

func writePNG(
    _ image: NSImage,
    _ destination: URL,
    size: Int,
    roundedDesktop: Bool,
    opaque: Bool
) throws {
    let data = try render(
        image,
        size: size,
        roundedDesktop: roundedDesktop,
        opaque: opaque
    )
    try data.write(to: destination, options: .atomic)
    print("generated \(destination.path) (\(size)×\(size))")
}

func writePNG(
    _ destination: URL,
    size: Int,
    roundedDesktop: Bool,
    opaque: Bool
) throws {
    try writePNG(
        source(for: size),
        destination,
        size: size,
        roundedDesktop: roundedDesktop,
        opaque: opaque
    )
}

try fileManager.createDirectory(at: brandingDirectory, withIntermediateDirectories: true)
try fileManager.createDirectory(at: desktopIconDirectory, withIntermediateDirectories: true)
try fileManager.createDirectory(at: iosIconDirectory, withIntermediateDirectories: true)

let desktopMaster = brandingDirectory.appendingPathComponent("synara-app-icon-desktop.png")
try writePNG(desktopMaster, size: 1024, roundedDesktop: true, opaque: false)

let desktopPNGs: [(String, Int)] = [
    ("16x16.png", 16),
    ("24x24.png", 24),
    ("32x32.png", 32),
    ("48x48.png", 48),
    ("64x64.png", 64),
    ("128x128.png", 128),
    ("128x128@2x.png", 256),
    ("256x256.png", 256),
    ("512x512.png", 512),
    ("icon.png", 512),
    ("StoreLogo.png", 50),
    ("Square30x30Logo.png", 30),
    ("Square44x44Logo.png", 44),
    ("Square71x71Logo.png", 71),
    ("Square89x89Logo.png", 89),
    ("Square107x107Logo.png", 107),
    ("Square142x142Logo.png", 142),
    ("Square150x150Logo.png", 150),
    ("Square284x284Logo.png", 284),
    ("Square310x310Logo.png", 310),
]
for (name, size) in desktopPNGs {
    try writePNG(
        desktopIconDirectory.appendingPathComponent(name),
        size: size,
        roundedDesktop: true,
        opaque: false
    )
}

let iosSizes = [20, 29, 40, 58, 60, 76, 80, 87, 120, 152, 167, 180, 1024]
for size in iosSizes {
    try writePNG(
        iosIconDirectory.appendingPathComponent("AppIcon-\(size).png"),
        size: size,
        roundedDesktop: false,
        opaque: true
    )
}

let temporaryDirectory = fileManager.temporaryDirectory
    .appendingPathComponent("synara-tauri-icons-\(UUID().uuidString)", isDirectory: true)
try fileManager.createDirectory(at: temporaryDirectory, withIntermediateDirectories: true)
defer { try? fileManager.removeItem(at: temporaryDirectory) }

// macOS applies its own app-icon mask. Build each ICNS representation from an
// opaque, unmasked square, using the reinforced source for compact sizes.
let iconsetDirectory = temporaryDirectory.appendingPathComponent("Synara.iconset", isDirectory: true)
try fileManager.createDirectory(at: iconsetDirectory, withIntermediateDirectories: true)
let macRepresentations: [(String, Int)] = [
    ("icon_16x16.png", 16),
    ("icon_16x16@2x.png", 32),
    ("icon_32x32.png", 32),
    ("icon_32x32@2x.png", 64),
    ("icon_128x128.png", 128),
    ("icon_128x128@2x.png", 256),
    ("icon_256x256.png", 256),
    ("icon_256x256@2x.png", 512),
    ("icon_512x512.png", 512),
    ("icon_512x512@2x.png", 1024),
]
for (name, size) in macRepresentations {
    try writePNG(
        iconsetDirectory.appendingPathComponent(name),
        size: size,
        roundedDesktop: false,
        opaque: true
    )
}

let generatedICNS = temporaryDirectory.appendingPathComponent("icon.icns")
let iconutil = Process()
iconutil.executableURL = URL(fileURLWithPath: "/usr/bin/iconutil")
iconutil.arguments = ["-c", "icns", "--output", generatedICNS.path, iconsetDirectory.path]
try iconutil.run()
iconutil.waitUntilExit()
guard iconutil.terminationStatus == 0 else {
    throw NSError(
        domain: "SynaraIconGenerator",
        code: 6,
        userInfo: [NSLocalizedDescriptionKey: "iconutil failed with status \(iconutil.terminationStatus)"]
    )
}
try Data(contentsOf: generatedICNS).write(
    to: desktopIconDirectory.appendingPathComponent("icon.icns"),
    options: .atomic
)
print("generated \(desktopIconDirectory.appendingPathComponent("icon.icns").path)")

// Windows owns no equivalent system mask. Use the stronger compact master for
// the ICO family so 16–64 px taskbar and launcher representations remain clear.
let windowsMaster = temporaryDirectory.appendingPathComponent("windows-master.png")
try writePNG(small, windowsMaster, size: 1024, roundedDesktop: true, opaque: false)

let tauriIcon = Process()
tauriIcon.executableURL = URL(fileURLWithPath: "/usr/bin/env")
tauriIcon.currentDirectoryURL = root
tauriIcon.arguments = [
    "npm", "exec", "tauri", "icon", "--",
    windowsMaster.path,
    "--output", temporaryDirectory.path,
]
try tauriIcon.run()
tauriIcon.waitUntilExit()
guard tauriIcon.terminationStatus == 0 else {
    throw NSError(
        domain: "SynaraIconGenerator",
        code: 7,
        userInfo: [NSLocalizedDescriptionKey: "tauri icon failed with status \(tauriIcon.terminationStatus)"]
    )
}
for name in ["icon.ico"] {
    let generated = temporaryDirectory.appendingPathComponent(name)
    let destination = desktopIconDirectory.appendingPathComponent(name)
    try Data(contentsOf: generated).write(to: destination, options: .atomic)
    print("generated \(destination.path)")
}
