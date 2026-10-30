import AVKit
import SwiftUI
import UIKit

/// A Klipy or GIPHY link played in place, loaded straight from the provider
/// as its terms require. If it fails to load, `onFailure` lets the row show
/// the link instead.
struct InlineGifView: View {
    let link: InlineGifLink
    let onFailure: () -> Void
    @State private var aspectRatio: CGFloat?

    private let maxSide: CGFloat = 280

    var body: some View {
        RemoteAnimatedImage(
            url: link.resolvedURL,
            contentMode: .scaleAspectFit,
            onLoad: { image in
                if image.size.height > 0 {
                    aspectRatio = image.size.width / image.size.height
                }
            },
            onFailure: onFailure
        ) {
            ZStack {
                SynaraColor.secondarySurface
                ProgressView()
            }
        }
        .frame(width: size.width, height: size.height)
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card * 1.5, style: .continuous))
        .accessibilityElement()
        .accessibilityLabel(Text("GIF"))
        .accessibilityAddTraits(.isImage)
        .accessibilityIdentifier("InlineGif")
    }

    private var size: CGSize {
        guard let aspectRatio, aspectRatio > 0 else {
            return CGSize(width: 220, height: 160)
        }
        let width = min(maxSide, maxSide * aspectRatio)
        return CGSize(width: width, height: width / aspectRatio)
    }
}

/// The X card for an x.com or twitter.com status link; nothing while it
/// loads or when X has no post, so the link in the message stands alone.
struct XPostEmbedView: View {
    let url: String
    @State private var post: XPost?

    var body: some View {
        // A zero-size stand-in, so the load task has a view to run on.
        ZStack {
            if let post {
                XPostCard(post: post)
            } else {
                Color.clear.frame(width: 0, height: 0)
            }
        }
        .accessibilityHidden(post == nil)
        .task(id: url) {
            post = nil
            let loaded = await XPostStore.shared.post(for: url)
            if Task.isCancelled == false {
                post = loaded
            }
        }
    }
}

/// The preview card for a message's first link (Settings → Link previews),
/// fetched by Core on this device; nothing while it loads or when the page
/// has no title.
struct LinkPreviewEmbedView: View {
    let url: URL
    @Environment(\.appEnvironment) private var environment
    @State private var preview: LinkPreview?

    var body: some View {
        ZStack {
            if let preview {
                LinkPreviewCard(preview: preview)
            } else {
                Color.clear.frame(width: 0, height: 0)
            }
        }
        .accessibilityHidden(preview == nil)
        .task(id: url) {
            preview = nil
            let loaded = await LinkPreviewStore.shared.preview(for: url, using: environment.linkPreviews)
            if Task.isCancelled == false {
                preview = loaded
            }
        }
    }
}

/// A flat card on a quiet inset, like the restyled agent cards: the host (and
/// the page's own site name after it), title and description beside the
/// page's image.
struct LinkPreviewCard: View {
    let preview: LinkPreview
    @Environment(\.openURL) private var openURL

    var body: some View {
        Button {
            openURL(preview.url)
        } label: {
            HStack(alignment: .top, spacing: SynaraSpacing.small) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(verbatim: preview.source)
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .lineLimit(1)
                    Text(verbatim: preview.title)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(SynaraColor.primaryText)
                        .multilineTextAlignment(.leading)
                        .lineLimit(2)
                    if let description = preview.description {
                        Text(verbatim: description)
                            .font(SynaraTypography.messageMeta)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .multilineTextAlignment(.leading)
                            .lineLimit(2)
                    }
                }
                Spacer(minLength: 0)
                if let image = preview.image {
                    Image(uiImage: image)
                        .resizable()
                        .scaledToFill()
                        .frame(width: 64, height: 64)
                        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
                        .accessibilityHidden(true)
                }
            }
            .padding(SynaraSpacing.small)
            .frame(maxWidth: 460, minHeight: SynaraHitTarget.minimum, alignment: .leading)
            .synaraAccessibleSurfaceFill(
                SynaraColor.mutedControl.opacity(0.45),
                opaqueFill: SynaraColor.surface
            )
            .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityElement(children: .combine)
        .accessibilityLabel(Text("Link preview: \(preview.title), \(preview.source)"))
        .accessibilityHint(Text("Opens the link in your browser"))
        .accessibilityIdentifier("LinkPreviewCard")
    }
}

struct XPostCard: View {
    let post: XPost
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            HStack(spacing: SynaraSpacing.small) {
                RemoteAnimatedImage(url: post.avatarURL.flatMap(URL.init(string:))) {
                    Circle().fill(SynaraColor.secondarySurface)
                }
                .frame(width: 32, height: 32)
                .clipShape(Circle())
                .accessibilityHidden(true)

                VStack(alignment: .leading, spacing: 1) {
                    Text(post.authorName)
                        .font(SynaraTypography.emphasis)
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(1)
                    Text(verbatim: "@\(post.authorHandle)")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.secondaryText)
                        .lineLimit(1)
                }
                Spacer(minLength: 0)
            }
            .accessibilityElement(children: .combine)

            if post.text.isEmpty == false {
                Text(verbatim: post.text)
                    .font(SynaraTypography.messageBody)
                    .foregroundStyle(SynaraColor.primaryText)
                    .fixedSize(horizontal: false, vertical: true)
                    .textSelection(.enabled)
            }

            let media = post.cardMedia
            if media.count == 1, let item = media.first {
                XPostMediaView(media: item, index: 1, count: 1)
                    .aspectRatio(CGFloat(item.aspectRatio ?? 16.0 / 9.0), contentMode: .fit)
                    .frame(maxWidth: .infinity, maxHeight: 360)
            } else if media.count > 1 {
                LazyVGrid(
                    columns: [GridItem(.flexible(), spacing: 4), GridItem(.flexible(), spacing: 4)],
                    spacing: 4
                ) {
                    ForEach(Array(media.enumerated()), id: \.element) { index, item in
                        XPostMediaView(media: item, index: index + 1, count: media.count)
                            .frame(height: 120)
                    }
                }
            }

            if let source = URL(string: post.url) {
                Button {
                    openURL(source)
                } label: {
                    Text("View on X")
                        .font(SynaraTypography.messageMeta)
                        .foregroundStyle(SynaraColor.accentText)
                        .frame(minHeight: SynaraHitTarget.minimum, alignment: .leading)
                        .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityHint(Text("Opens the post in your browser"))
                .accessibilityIdentifier("XPostSourceLink")
            }
        }
        .padding(SynaraSpacing.medium)
        .frame(maxWidth: 460, alignment: .leading)
        .synaraCard(fill: SynaraColor.surface)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("Post by \(post.authorName) on X"))
        .accessibilityIdentifier("XPostCard")
    }
}

/// A photo, or a video or GIF that plays inline once tapped.
private struct XPostMediaView: View {
    let media: XPostMedia
    let index: Int
    let count: Int
    @State private var player: AVPlayer?
    @State private var looper: AVPlayerLooper?

    var body: some View {
        ZStack {
            Color.black
            switch media {
            case let .photo(url, _, _):
                RemoteAnimatedImage(url: URL(string: url), contentMode: .scaleAspectFill) {
                    SynaraColor.secondarySurface
                }
                .accessibilityElement()
                .accessibilityLabel(Text(count > 1 ? "Photo \(index) of \(count)" : "Photo"))
                .accessibilityAddTraits(.isImage)
            case let .video(src, poster, _, _):
                playable(src: src, poster: poster, isGif: false)
            case let .gif(src, poster, _, _):
                playable(src: src, poster: poster, isGif: true)
            }
        }
        .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous))
        .onDisappear {
            player?.pause()
        }
    }

    @ViewBuilder
    private func playable(src: String, poster: String?, isGif: Bool) -> some View {
        if let player {
            if isGif {
                LoopingVideoView(player: player)
                    .accessibilityElement()
                    .accessibilityLabel(Text("GIF from the post"))
                    .accessibilityAddTraits(.startsMediaSession)
            } else {
                VideoPlayer(player: player)
                    .accessibilityLabel(Text("Video from the post"))
            }
        } else {
            Button {
                start(src: src, isGif: isGif)
            } label: {
                ZStack {
                    if let poster, let posterURL = URL(string: poster) {
                        RemoteAnimatedImage(url: posterURL, contentMode: .scaleAspectFill) {
                            Color.black
                        }
                    }
                    Image(systemName: "play.circle.fill")
                        .font(.system(size: 44, weight: .regular))
                        .symbolRenderingMode(.palette)
                        .foregroundStyle(Color.white, Color.black.opacity(0.55))
                }
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text(isGif ? "Play GIF from the post" : "Play video from the post"))
            .accessibilityAddTraits(.startsMediaSession)
            .accessibilityIdentifier(isGif ? "XPostPlayGif" : "XPostPlayVideo")
        }
    }

    private func start(src: String, isGif: Bool) {
        guard let url = URL(string: src), RemoteContentURL.safeHost(src) == XPostEmbed.videoHost else {
            return
        }
        // No cookies travel with X media requests.
        let asset = AVURLAsset(url: url, options: [AVURLAssetHTTPCookiesKey: [HTTPCookie]()])
        let item = AVPlayerItem(asset: asset)
        if isGif {
            // X's GIFs are short MP4s: they loop silently, as a GIF would.
            let queue = AVQueuePlayer()
            queue.isMuted = true
            looper = AVPlayerLooper(player: queue, templateItem: item)
            player = queue
            queue.play()
        } else {
            let player = AVPlayer(playerItem: item)
            self.player = player
            player.play()
        }
    }
}

/// A player layer without controls, for X's looping GIFs.
private struct LoopingVideoView: UIViewRepresentable {
    let player: AVPlayer

    final class PlayerView: UIView {
        override static var layerClass: AnyClass { AVPlayerLayer.self }
        var playerLayer: AVPlayerLayer { layer as! AVPlayerLayer }
    }

    func makeUIView(context: Context) -> PlayerView {
        let view = PlayerView()
        view.playerLayer.videoGravity = .resizeAspect
        view.playerLayer.player = player
        return view
    }

    func updateUIView(_ view: PlayerView, context: Context) {
        if view.playerLayer.player !== player {
            view.playerLayer.player = player
        }
    }
}
