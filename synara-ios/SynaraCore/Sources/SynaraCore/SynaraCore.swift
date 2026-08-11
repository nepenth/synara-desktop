import Foundation

/// Project-owned UniFFI module bootstrap.
///
/// Generated sources are deliberately not checked in. Run
/// `scripts/generate-synara-core-swift.sh` from the repository root on a
/// configured Apple build host before linking an iOS application. This P4-1
/// package contains no service adapter or Matrix client fallback.
public enum SynaraCoreBindings {
    /// The stable UniFFI namespace emitted by synara-core's project-owned UDL.
    public static let namespace = "synara_core"
}
