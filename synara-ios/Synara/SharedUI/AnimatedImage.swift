import ImageIO
import SwiftUI
import UIKit

/// Decodes GIF, animated WebP and APNG data into an animated `UIImage`
/// (`UIImage(data:)` keeps only the first frame). Frames are downsampled to
/// the size they are shown at, and a large animation drops frames to stay
/// inside a decoded-memory budget instead of holding every frame.
enum AnimatedImageDecoder {
    struct Limits: Equatable {
        /// Longest side of a decoded frame, in pixels.
        var maxPixelSize: Int = 640
        var maxFrames: Int = 200
        /// Bytes all decoded frames may take together (4 bytes per pixel).
        var maxDecodedBytes: Int = 48 * 1024 * 1024

        static let inline = Limits()
        static let fullScreen = Limits(maxPixelSize: 1600, maxFrames: 300, maxDecodedBytes: 96 * 1024 * 1024)
    }

    /// Browsers treat shorter GIF delays as 0.1 s; so does this decoder.
    private static let minimumFrameDelay = 0.011
    private static let defaultFrameDelay = 0.1
    /// The longest frame list built to honour uneven delays.
    private static let maxTimelineEntries = 1200

    /// `image(from:limits:)` off the main thread.
    static func decode(_ data: Data, limits: Limits = .inline) async -> UIImage? {
        await Task.detached(priority: .userInitiated) {
            image(from: data, limits: limits)
        }.value
    }

    static func image(from data: Data, limits: Limits = .inline) -> UIImage? {
        let options = [kCGImageSourceShouldCache: false] as CFDictionary
        guard let source = CGImageSourceCreateWithData(data as CFData, options) else {
            return nil
        }
        let count = CGImageSourceGetCount(source)
        guard count > 0 else {
            return nil
        }
        guard count > 1 else {
            return frame(source, at: 0, maxPixelSize: limits.maxPixelSize).map { UIImage(cgImage: $0) }
        }

        let frameBytes = decodedBytesPerFrame(source, maxPixelSize: limits.maxPixelSize)
        let budgetFrames = max(1, limits.maxDecodedBytes / max(1, frameBytes))
        let keptFrames = max(1, min(limits.maxFrames, budgetFrames, count))
        // Keep every `step`th frame and give it the delays of those dropped.
        let step = Int((Double(count) / Double(keptFrames)).rounded(.up))

        var frames: [UIImage] = []
        var delays: [Double] = []
        var index = 0
        while index < count {
            let end = min(count, index + step)
            if let cgImage = frame(source, at: index, maxPixelSize: limits.maxPixelSize) {
                frames.append(UIImage(cgImage: cgImage))
                delays.append((index ..< end).reduce(0) { $0 + frameDelay(source, at: $1) })
            }
            index = end
        }
        guard let first = frames.first else {
            return nil
        }
        guard frames.count > 1 else {
            return first
        }
        return animatedImage(frames: frames, delays: delays) ?? first
    }

    /// `UIImage.animatedImage` shows each frame for the same time, so a frame
    /// with a longer delay is repeated (by reference) to keep the GIF's timing.
    private static func animatedImage(frames: [UIImage], delays: [Double]) -> UIImage? {
        let total = delays.reduce(0, +)
        let centiseconds = delays.map { max(1, Int(($0 * 100).rounded())) }
        let unit = centiseconds.reduce(0) { gcd($0, $1) }
        let entries = centiseconds.reduce(0) { $0 + $1 / max(1, unit) }
        guard unit > 0, entries <= maxTimelineEntries else {
            return UIImage.animatedImage(with: frames, duration: total)
        }
        var timeline: [UIImage] = []
        timeline.reserveCapacity(entries)
        for (frame, length) in zip(frames, centiseconds) {
            timeline.append(contentsOf: repeatElement(frame, count: length / unit))
        }
        return UIImage.animatedImage(with: timeline, duration: total)
    }

    private static func frame(_ source: CGImageSource, at index: Int, maxPixelSize: Int) -> CGImage? {
        let options = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: maxPixelSize,
        ] as CFDictionary
        return CGImageSourceCreateThumbnailAtIndex(source, index, options)
    }

    private static func decodedBytesPerFrame(_ source: CGImageSource, maxPixelSize: Int) -> Int {
        let properties = CGImageSourceCopyPropertiesAtIndex(source, 0, nil) as? [CFString: Any]
        let width = (properties?[kCGImagePropertyPixelWidth] as? NSNumber)?.doubleValue ?? Double(maxPixelSize)
        let height = (properties?[kCGImagePropertyPixelHeight] as? NSNumber)?.doubleValue ?? Double(maxPixelSize)
        let longest = max(width, height, 1)
        let scale = min(1, Double(maxPixelSize) / longest)
        return Int((width * scale).rounded(.up) * (height * scale).rounded(.up) * 4)
    }

    static func frameDelay(_ source: CGImageSource, at index: Int) -> Double {
        guard let properties = CGImageSourceCopyPropertiesAtIndex(source, index, nil) as? [CFString: Any] else {
            return defaultFrameDelay
        }
        let containers: [(CFString, CFString, CFString)] = [
            (kCGImagePropertyGIFDictionary, kCGImagePropertyGIFUnclampedDelayTime, kCGImagePropertyGIFDelayTime),
            (kCGImagePropertyWebPDictionary, kCGImagePropertyWebPUnclampedDelayTime, kCGImagePropertyWebPDelayTime),
            (kCGImagePropertyPNGDictionary, kCGImagePropertyAPNGUnclampedDelayTime, kCGImagePropertyAPNGDelayTime),
        ]
        for (dictionaryKey, unclampedKey, clampedKey) in containers {
            guard let dictionary = properties[dictionaryKey] as? [CFString: Any] else {
                continue
            }
            let delay = (dictionary[unclampedKey] as? NSNumber)?.doubleValue
                ?? (dictionary[clampedKey] as? NSNumber)?.doubleValue
                ?? defaultFrameDelay
            return delay < minimumFrameDelay ? defaultFrameDelay : delay
        }
        return defaultFrameDelay
    }

    private static func gcd(_ a: Int, _ b: Int) -> Int {
        b == 0 ? a : gcd(b, a % b)
    }
}

/// Animated (and still) images for third-party links, fetched with
/// `RemoteFetch` and kept decoded for this session so rows scrolling back
/// in do not refetch or redecode.
actor RemoteImageStore {
    static let shared = RemoteImageStore()

    /// GIF links and X media; Klipy's md renditions are well under this.
    static let maxDownloadBytes = 16 * 1024 * 1024

    private let cache: NSCache<NSString, UIImage> = {
        let cache = NSCache<NSString, UIImage>()
        cache.totalCostLimit = 96 * 1024 * 1024
        cache.countLimit = 160
        return cache
    }()
    private var inFlight: [String: Task<UIImage?, Never>] = [:]

    func image(for url: URL, limits: AnimatedImageDecoder.Limits = .inline) async -> UIImage? {
        let key = "\(limits.maxPixelSize)|\(url.absoluteString)"
        if let cached = cache.object(forKey: key as NSString) {
            return cached
        }
        if let task = inFlight[key] {
            return await task.value
        }
        let task = Task.detached(priority: .utility) { () -> UIImage? in
            // No content-type check: ImageIO decodes only real images, and
            // CDNs label GIFs inconsistently.
            guard let data = try? await RemoteFetch.data(from: url, maxBytes: Self.maxDownloadBytes) else {
                return nil
            }
            return AnimatedImageDecoder.image(from: data, limits: limits)
        }
        inFlight[key] = task
        let image = await task.value
        inFlight[key] = nil
        if let image {
            cache.setObject(image, forKey: key as NSString, cost: image.decodedByteCost)
        }
        return image
    }
}

extension UIImage {
    /// Approximate decoded size: frames repeated for timing share storage.
    var decodedByteCost: Int {
        let frames = images ?? [self]
        var seen = Set<ObjectIdentifier>()
        var bytes = 0
        for frame in frames where seen.insert(ObjectIdentifier(frame)).inserted {
            bytes += Int(frame.size.width * frame.scale * frame.size.height * frame.scale * 4)
        }
        return bytes
    }

    var isAnimated: Bool {
        (images?.count ?? 0) > 1
    }
}

/// Shows an animated `UIImage`; SwiftUI's `Image` draws only the first frame.
/// With Reduce Motion on, the first frame stays still.
struct AnimatedImageView: UIViewRepresentable {
    let image: UIImage
    var contentMode: UIView.ContentMode = .scaleAspectFill
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func makeUIView(context: Context) -> UIImageView {
        let view = UIImageView()
        view.clipsToBounds = true
        view.isAccessibilityElement = false
        for axis in [NSLayoutConstraint.Axis.horizontal, .vertical] {
            view.setContentCompressionResistancePriority(.defaultLow, for: axis)
            view.setContentHuggingPriority(.defaultLow, for: axis)
        }
        return view
    }

    func updateUIView(_ view: UIImageView, context: Context) {
        view.contentMode = contentMode
        let shown = reduceMotion ? (image.images?.first ?? image) : image
        if view.image !== shown {
            view.image = shown
        }
    }
}

/// An image loaded from a third-party URL through `RemoteImageStore`.
/// `placeholder` shows until the image loads, and stays when the load
/// fails (reported through `onFailure`).
struct RemoteAnimatedImage<Placeholder: View>: View {
    let url: URL?
    var limits: AnimatedImageDecoder.Limits = .inline
    var contentMode: UIView.ContentMode = .scaleAspectFill
    /// Drops the decoded image when the view scrolls away (it stays in
    /// `RemoteImageStore` while memory allows), so a long grid holds only
    /// what is on screen.
    var releasesOnDisappear = true
    var onLoad: (UIImage) -> Void = { _ in }
    var onFailure: () -> Void = {}
    @ViewBuilder let placeholder: () -> Placeholder
    @State private var image: UIImage?

    var body: some View {
        // One container, so swapping placeholder and image neither restarts
        // the load nor counts as the view disappearing.
        ZStack {
            if let image {
                AnimatedImageView(image: image, contentMode: contentMode)
            } else {
                placeholder()
            }
        }
        .task(id: url) {
            await load()
        }
        .onDisappear {
            if releasesOnDisappear {
                image = nil
            }
        }
    }

    @MainActor
    private func load() async {
        guard let url else {
            image = nil
            onFailure()
            return
        }
        let loaded = await RemoteImageStore.shared.image(for: url, limits: limits)
        guard Task.isCancelled == false else {
            return
        }
        image = loaded
        if let loaded {
            onLoad(loaded)
        } else {
            onFailure()
        }
    }
}
