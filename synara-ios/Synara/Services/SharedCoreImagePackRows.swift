import Foundation

/// P4-S37 map of privacy-safe SharedCore image-pack JSON to sticker rows.
///
/// Names, ids, and `mxc://` metadata only. No image bytes. Send stays on
/// Incoming image-pack projection only. This is not iOS-on-engine and not P4 acceptance.
struct SharedCoreSticker: Equatable, Identifiable {
    let id: String
    let packId: String
    let packName: String
    let body: String
    let mxc: String
    let width: UInt64?
    let height: UInt64?
    let mimetype: String?
    let size: UInt64?
}

enum SharedCoreImagePackRows {
    static func stickers(packId: String, packName: String?, contentJSON: String) -> [SharedCoreSticker] {
        guard let data = contentJSON.data(using: .utf8),
              let object = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any]
        else {
            return []
        }
        let name = displayName(packName: packName, object: object, fallback: packId)
        guard let images = object["images"] as? [String: Any] else {
            return []
        }
        return images.keys.sorted().compactMap { key in
            guard let image = images[key] as? [String: Any] else {
                return nil
            }
            let url = string(image["url"]) ?? ""
            guard url.hasPrefix("mxc://") else {
                return nil
            }
            let info = image["info"] as? [String: Any]
            let body = string(image["body"]) ?? key
            return SharedCoreSticker(
                id: "\(packId):\(key)",
                packId: packId,
                packName: name,
                body: body,
                mxc: url,
                width: uint64(info?["w"] ?? image["w"]),
                height: uint64(info?["h"] ?? image["h"]),
                mimetype: string(info?["mimetype"] ?? image["mimetype"]),
                size: uint64(info?["size"] ?? image["size"])
            )
        }
    }

    static func displayName(packName: String?, object: [String: Any], fallback: String) -> String {
        if let packName, packName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            return packName
        }
        if let pack = object["pack"] as? [String: Any],
           let name = string(pack["display_name"]),
           name.isEmpty == false
        {
            return name
        }
        return fallback
    }

    private static func string(_ value: Any?) -> String? {
        guard let value = value as? String else {
            return nil
        }
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }

    private static func uint64(_ value: Any?) -> UInt64? {
        if let number = value as? NSNumber {
            return number.uint64Value
        }
        if let int = value as? Int, int >= 0 {
            return UInt64(int)
        }
        if let int = value as? UInt64 {
            return int
        }
        return nil
    }
}
