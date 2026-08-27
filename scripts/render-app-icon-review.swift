#!/usr/bin/env swift

import AppKit
import Foundation

struct IconConcept {
    let label: String
    let path: String
    let isCurrent: Bool
}

let fileManager = FileManager.default
let root = fileManager.currentDirectoryPath
let concepts = [
    IconConcept(label: "Current · Desktop", path: "src-tauri/icons/icon.png", isCurrent: true),
    IconConcept(label: "Current · iOS", path: "synara-ios/Synara/Resources/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png", isCurrent: false),
    IconConcept(label: "E · Faithful Balanced", path: "docs/design/app-icon-refresh/refinement-e-faithful-balanced.png", isCurrent: false),
    IconConcept(label: "F · Bold Cleanup", path: "docs/design/app-icon-refresh/refinement-f-bold-cleanup.png", isCurrent: false),
    IconConcept(label: "H · Conservative", path: "docs/design/app-icon-refresh/refinement-h-conservative-recomposition.png", isCurrent: false),
]

func load(_ relativePath: String) -> NSImage {
    let absolutePath = URL(fileURLWithPath: root).appendingPathComponent(relativePath).path
    guard let image = NSImage(contentsOfFile: absolutePath) else {
        fatalError("Unable to load \(absolutePath)")
    }
    return image
}

let images = concepts.map { load($0.path) }
let canvasSize = NSSize(width: 1680, height: 1040)
let canvas = NSImage(size: canvasSize)
canvas.lockFocusFlipped(true)

NSColor(calibratedRed: 0.055, green: 0.063, blue: 0.082, alpha: 1).setFill()
NSBezierPath(rect: NSRect(origin: .zero, size: canvasSize)).fill()

func drawText(_ text: String, at point: NSPoint, size: CGFloat, color: NSColor, weight: NSFont.Weight = .regular) {
    let attributes: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: size, weight: weight),
        .foregroundColor: color,
    ]
    text.draw(at: point, withAttributes: attributes)
}

func drawIcon(_ image: NSImage, in rect: NSRect, appleMask: Bool, current: Bool) {
    NSGraphicsContext.saveGraphicsState()
    if appleMask && !current {
        let mask = NSBezierPath(roundedRect: rect, xRadius: rect.width * 0.225, yRadius: rect.height * 0.225)
        mask.addClip()
    }
    image.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1)
    NSGraphicsContext.restoreGraphicsState()
}

drawText("Synara app icon study", at: NSPoint(x: 64, y: 46), size: 38, color: .white, weight: .semibold)
drawText("Actual raster masters shown at platform-relevant sizes", at: NSPoint(x: 64, y: 96), size: 18, color: NSColor(white: 0.68, alpha: 1))

let columnWidth: CGFloat = 304
let left: CGFloat = 64
for (index, concept) in concepts.enumerated() {
    let x = left + CGFloat(index) * columnWidth
    drawText(concept.label, at: NSPoint(x: x, y: 150), size: 15, color: NSColor(white: 0.86, alpha: 1), weight: .medium)
}

drawText("macOS Dock · 96 px · system rounded-square mask", at: NSPoint(x: 64, y: 205), size: 21, color: NSColor(white: 0.94, alpha: 1), weight: .semibold)
let dockRect = NSRect(x: 44, y: 245, width: 1590, height: 152)
NSColor(calibratedWhite: 0.22, alpha: 0.72).setFill()
NSBezierPath(roundedRect: dockRect, xRadius: 38, yRadius: 38).fill()
NSColor(calibratedWhite: 1, alpha: 0.10).setStroke()
NSBezierPath(roundedRect: dockRect, xRadius: 38, yRadius: 38).stroke()

for index in concepts.indices {
    let x = left + CGFloat(index) * columnWidth + 82
    drawIcon(images[index], in: NSRect(x: x, y: 273, width: 96, height: 96), appleMask: true, current: concepts[index].isCurrent)
}

drawText("Linux application grid · 64 px · proposed pre-rounded color asset", at: NSPoint(x: 64, y: 445), size: 21, color: NSColor(white: 0.94, alpha: 1), weight: .semibold)
let gridRect = NSRect(x: 44, y: 485, width: 1590, height: 128)
NSColor(calibratedRed: 0.105, green: 0.118, blue: 0.145, alpha: 1).setFill()
NSBezierPath(roundedRect: gridRect, xRadius: 18, yRadius: 18).fill()

for index in concepts.indices {
    let x = left + CGFloat(index) * columnWidth + 98
    drawIcon(images[index], in: NSRect(x: x, y: 517, width: 64, height: 64), appleMask: !concepts[index].isCurrent, current: concepts[index].isCurrent)
}

drawText("Small-size proof · 16 / 24 / 32 px", at: NSPoint(x: 64, y: 664), size: 21, color: NSColor(white: 0.94, alpha: 1), weight: .semibold)
let proofRect = NSRect(x: 44, y: 705, width: 1590, height: 220)
NSColor(calibratedRed: 0.075, green: 0.084, blue: 0.103, alpha: 1).setFill()
NSBezierPath(roundedRect: proofRect, xRadius: 18, yRadius: 18).fill()

let sizes: [CGFloat] = [16, 24, 32]
for index in concepts.indices {
    let baseX = left + CGFloat(index) * columnWidth + 62
    for (sizeIndex, size) in sizes.enumerated() {
        let y = 748 + CGFloat(sizeIndex) * 52
        drawIcon(images[index], in: NSRect(x: baseX, y: y, width: size, height: size), appleMask: !concepts[index].isCurrent, current: concepts[index].isCurrent)
        drawText("\(Int(size)) px", at: NSPoint(x: baseX + 50, y: y - 1), size: 14, color: NSColor(white: 0.68, alpha: 1))
    }
}

drawText("Evaluation target: distinct silhouette, centered visual mass, no disappearing detail", at: NSPoint(x: 64, y: 968), size: 17, color: NSColor(white: 0.62, alpha: 1))

canvas.unlockFocus()
guard let tiff = canvas.tiffRepresentation,
      let bitmap = NSBitmapImageRep(data: tiff),
      let png = bitmap.representation(using: .png, properties: [:]) else {
    fatalError("Unable to encode review board")
}

let output = URL(fileURLWithPath: root).appendingPathComponent("docs/design/app-icon-refresh/refinement-platform-preview.png")
try png.write(to: output)
print(output.path)
