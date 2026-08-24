// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SynaraNseCore",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "SynaraNseCore", targets: ["SynaraNseCore"]),
    ],
    targets: [
        .binaryTarget(
            name: "synara_nse_coreFFI",
            path: "Artifacts/SynaraNseCore.xcframework"
        ),
        .target(
            name: "SynaraNseCore",
            dependencies: ["synara_nse_coreFFI"],
            path: "Sources/SynaraNseCore",
            exclude: ["Generated/README.md"]
        ),
    ]
)
