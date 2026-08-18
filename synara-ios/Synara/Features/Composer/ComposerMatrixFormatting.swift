import SynaraCore

enum ComposerMatrixFormatting {
    static func formattedBody(for body: String) -> String? {
        SynaraCore.markdownToHtml(body: body)
    }
}
