import SynaraCore

enum ComposerMatrixFormatting {
    static func formattedBody(for body: String) -> String? {
        SynaraCore.markdownToHtml(body: body)
    }

    static func plainBody(for draft: String, formattedBody: String?) -> String {
        guard let formattedBody else { return draft }
        let projected = MatrixHTMLRenderer.selectionProjection(
            body: draft,
            html: formattedBody,
            revealingSpoilers: false
        ).richText.plainText
        let plain = projected.replacingOccurrences(
            of: #"\[Spoiler[^\]]*\]"#,
            with: "[spoiler]",
            options: .regularExpression
        ).trimmingCharacters(in: .whitespacesAndNewlines)
        return plain.isEmpty ? draft : plain
    }
}
