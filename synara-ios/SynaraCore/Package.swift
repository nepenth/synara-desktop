// swift-tools-version: 5.9
import PackageDescription

// P4-1: This is intentionally a bindings package, not an iOS service adapter.
// `scripts/generate-synara-core-swift.sh` writes project-owned UniFFI Swift
// sources into Sources/SynaraCore/Generated and the target compiles them as
// part of this module. P4 migration slices add typed facade APIs only when
// their matching Rust command/sink surface is ready.
let package = Package(
    name: "SynaraCore",
    platforms: [
        .iOS(.v16),
        .macOS(.v13),
    ],
    products: [
        .library(name: "SynaraCore", targets: ["SynaraCore"]),
    ],
    targets: [
        .target(
            name: "SynaraCore",
            path: "Sources/SynaraCore"
        ),
    ]
)
