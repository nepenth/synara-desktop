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
        // The generator places the generated `synara_coreFFI` C module and the
        // matching Rust static libraries in this XCFramework. The generated
        // Swift imports that module; modeling it as a binary target ensures an
        // app linking SynaraCore also links the real project-owned FFI.
        .binaryTarget(
            name: "synara_coreFFI",
            path: "Artifacts/SynaraCore.xcframework"
        ),
        .target(
            name: "SynaraCore",
            dependencies: ["synara_coreFFI"],
            path: "Sources/SynaraCore",
            // Keep the tracked generation instructions out of the module while
            // accepting generated .swift files in this directory at build time.
            exclude: ["Generated/README.md"]
        ),
    ]
)
