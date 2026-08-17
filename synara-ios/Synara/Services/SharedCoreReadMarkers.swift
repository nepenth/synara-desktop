import Foundation

/// P4-S24 map of privacy-safe SharedCore timeline read-state to product
/// read markers.
///
/// Uses existing open / set-read-state / close only. Acknowledged ids
/// must be server event ids. This is not iOS-on-engine and not P4
/// acceptance.
enum SharedCoreReadMarkers {
    static func acknowledgedEventID(ownReadEventID: String?, rowEventIDs: [String]) -> String? {
        if let ownReadEventID, MatrixServerEventIDPolicy.canAcknowledge(ownReadEventID) {
            return ownReadEventID
        }
        return rowEventIDs.reversed().first(where: MatrixServerEventIDPolicy.canAcknowledge)
    }
}
