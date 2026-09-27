import SwiftUI

// Companion image transport: the `ReadAttachmentChunk` loop from
// crates/ui/src/attachments.rs (`read_attachment_image`: 45KB base64 chunks,
// bounded, stuck-offset guarded, size-capped) with a decoded-image NSCache.
// The loader is injected through the environment so CompanionTranscript can
// stay model-free.

/// One `ReadAttachmentChunk` reply.
struct CompanionAttachmentChunk: Decodable {
    var name: String
    var mimeType: String
    var data: String
    var nextOffset: UInt64
    var done: Bool
}

@MainActor
final class CompanionImageLoader {
    typealias Fetch = @MainActor (_ path: String, _ offset: UInt64) async -> CompanionAttachmentChunk?
    /// The host owning the connection, resolved per key so a path served by
    /// two computers never cross-hits the decoded-image cache.
    typealias HostIdentity = @MainActor () -> String?

    /// 24 MiB, the desktop's `MAX_ATTACHMENT_BYTES` / generated-image cap.
    static let maxImageBytes = 24 * 1024 * 1024
    private static let maxReadChunks = 1_000

    private let fetch: Fetch
    private let hostIdentity: HostIdentity
    private let cache = NSCache<NSString, UIImage>()

    init(fetch: @escaping Fetch, hostIdentity: @escaping HostIdentity = { nil }) {
        self.fetch = fetch
        self.hostIdentity = hostIdentity
        // Same 64 MiB decoded-image budget as the cloud attachment cache.
        cache.totalCostLimit = 64 * 1024 * 1024
        cache.countLimit = 24
    }

    /// The companion session's connection, resolved lazily so reconnects are
    /// picked up per read.
    convenience init(model: CompanionModel) {
        self.init(fetch: { [weak model] path, offset in
            guard let model, model.online else { return nil }
            return try? await CompanionModel.decode(CompanionAttachmentChunk.self,
                model.connection.call("ReadAttachmentChunk", ["path": path, "offset": offset]))
        }, hostIdentity: { [weak model] in model?.selected?.deviceId })
    }

    func cached(path: String, mimeType: String?) -> UIImage? {
        cache.object(forKey: key(path: path, mimeType: mimeType) as NSString)
    }

    func image(path: String, mimeType: String?) async -> UIImage? {
        let cacheKey = key(path: path, mimeType: mimeType)
        if let hit = cache.object(forKey: cacheKey as NSString) { return hit }
        guard let mimeType, GeneratedImageReference.supportedMimeTypes.contains(mimeType) else { return nil }
        var b64 = ""
        var offset: UInt64 = 0
        var done = false
        for _ in 0..<Self.maxReadChunks {
            guard let chunk = await fetch(path, offset),
                  chunk.mimeType == mimeType,
                  b64.utf8.count + chunk.data.utf8.count <= Self.maxImageBytes / 3 * 4 else { return nil }
            b64 += chunk.data
            done = chunk.done
            if done { break }
            guard chunk.nextOffset > offset else { return nil }
            offset = chunk.nextOffset
        }
        guard done, let data = Data(base64Encoded: b64),
              let decoded = await Task.detached(priority: .utility, operation: {
                  AttachmentImageCache.decodeGeneratedImage(data, mimeType: mimeType)
              }).value else { return nil }
        cache.setObject(decoded.image, forKey: cacheKey as NSString, cost: decoded.bytes)
        return decoded.image
    }

    private func key(path: String, mimeType: String?) -> String {
        "\(hostIdentity() ?? "")|\(mimeType ?? "")|\(path)"
    }
}

private struct CompanionImageLoaderKey: EnvironmentKey {
    static let defaultValue: CompanionImageLoader? = nil
}

extension EnvironmentValues {
    /// Injected by the session screen (`CompanionImageLoader(model:)`).
    var companionImageLoader: CompanionImageLoader? {
        get { self[CompanionImageLoaderKey.self] }
        set { self[CompanionImageLoaderKey.self] = newValue }
    }
}

// MARK: - Inline generated image

/// Inline output image: content-width, aspect fit, 14pt continuous corners,
/// hairline border; taps open the zoom/share viewer.
struct CompanionGeneratedImageView: View {
    let reference: GeneratedImageReference
    var onResize: () -> Void = {}
    @Environment(\.companionImageLoader) private var loader
    @State private var image: UIImage?
    @State private var failed = false
    @State private var preview: AttachmentPreview?

    var body: some View {
        Group {
            if let image {
                Button {
                    preview = AttachmentPreview(name: reference.name, image: image)
                } label: {
                    Image(uiImage: image)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
                        .overlay(RoundedRectangle(cornerRadius: 14, style: .continuous)
                            .strokeBorder(Theme.wash(0.11), lineWidth: 1))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Preview generated image")
                .accessibilityHint(reference.name)
            } else if failed {
                Label(reference.name, systemImage: "photo")
                    .font(Theme.sans(13))
                    .foregroundStyle(Theme.textFaint)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity)
                    .frame(height: 120)
                    .background(Theme.wash(0.035), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                    .accessibilityLabel("Image on your computer")
            } else {
                CompanionImageShimmer()
            }
        }
        .frame(maxWidth: .infinity)
        .onGeometryChange(for: CGSize.self) { $0.size } action: { _ in onResize() }
        .task(id: "\(reference.path)|\(reference.mimeType)") {
            guard image == nil, !failed else { return }
            guard let loader else { failed = true; return }
            if let hit = loader.cached(path: reference.path, mimeType: reference.mimeType) { image = hit; onResize(); return }
            guard let loaded = await loader.image(path: reference.path, mimeType: reference.mimeType) else {
                failed = true
                onResize()
                return
            }
            image = loaded
            onResize()
        }
        .fullScreenCover(item: $preview) { CompanionImageViewer(preview: $0) }
    }
}

/// Quiet loading placeholder with a 4:3 footprint and a sweeping sheen.
struct CompanionImageShimmer: View {
    @State private var sweeping = false

    var body: some View {
        RoundedRectangle(cornerRadius: 14, style: .continuous)
            .fill(Theme.wash(0.035))
            .aspectRatio(4 / 3, contentMode: .fit)
            .overlay {
                GeometryReader { geo in
                    LinearGradient(colors: [.clear, Theme.wash(0.09), .clear], startPoint: .leading, endPoint: .trailing)
                        .frame(width: geo.size.width * 0.6)
                        .offset(x: sweeping ? geo.size.width : -geo.size.width * 0.6)
                        .animation(.linear(duration: 1.4).repeatForever(autoreverses: false), value: sweeping)
                }
                .clipShape(RoundedRectangle(cornerRadius: 14, style: .continuous))
            }
            .frame(maxWidth: .infinity)
            .onAppear { sweeping = true }
            .accessibilityLabel("Loading image")
    }
}

// MARK: - Full-screen viewer (pinch zoom + share)

struct CompanionImageViewer: View {
    let preview: AttachmentPreview
    @Environment(\.dismiss) private var dismiss

    private var shareImage: Image { Image(uiImage: preview.image) }

    var body: some View {
        ZStack {
            Color.black.opacity(0.94).ignoresSafeArea()
            ZoomableImage(image: preview.image)
                .ignoresSafeArea(edges: .horizontal)
            VStack {
                HStack(spacing: 12) {
                    Spacer()
                    ShareLink(item: shareImage, preview: SharePreview(preview.name, image: shareImage)) {
                        Image(systemName: "square.and.arrow.up")
                            .font(.system(size: 17, weight: .medium))
                            .foregroundStyle(.white)
                            .frame(width: 44, height: 44)
                            .background(.black.opacity(0.6), in: Circle())
                    }
                    .accessibilityLabel("Share image")
                    Button { dismiss() } label: {
                        Image(systemName: "xmark")
                            .font(.system(size: 17, weight: .medium))
                            .foregroundStyle(.white)
                            .frame(width: 44, height: 44)
                            .background(.black.opacity(0.6), in: Circle())
                    }
                    .accessibilityLabel("Close image preview")
                }
                Spacer()
                Text(preview.name)
                    .font(Theme.sans(11))
                    .foregroundStyle(Theme.textMuted)
                    .lineLimit(1)
                    .padding(.horizontal, 16)
            }
            .padding(12)
        }
        .presentationBackground(.clear)
    }
}

/// UIKit pinch zoom; SwiftUI's magnify gesture would fight the full-screen
/// cover's swipe-to-dismiss.
struct ZoomableImage: UIViewRepresentable {
    let image: UIImage

    func makeUIView(context: Context) -> UIScrollView {
        let scroll = UIScrollView()
        scroll.delegate = context.coordinator
        scroll.minimumZoomScale = 1
        scroll.maximumZoomScale = 6
        scroll.bouncesZoom = true
        scroll.showsVerticalScrollIndicator = false
        scroll.showsHorizontalScrollIndicator = false
        scroll.backgroundColor = .clear
        let imageView = UIImageView(image: image)
        imageView.contentMode = .scaleAspectFit
        imageView.frame = scroll.bounds
        imageView.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        scroll.addSubview(imageView)
        context.coordinator.imageView = imageView
        return scroll
    }

    func updateUIView(_ scroll: UIScrollView, context: Context) {
        context.coordinator.imageView?.image = image
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    final class Coordinator: NSObject, UIScrollViewDelegate {
        weak var imageView: UIImageView?

        func viewForZooming(in scrollView: UIScrollView) -> UIView? { imageView }
    }
}
