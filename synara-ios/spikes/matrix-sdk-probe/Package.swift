// swift-tools-version: 5.10

import PackageDescription

let package = Package(
    name: "MatrixSDKProbe",
    platforms: [
        .iOS(.v16),
        .macOS(.v12)
    ],
    products: [
        .executable(name: "MatrixSDKProbe", targets: ["MatrixSDKProbe"])
    ],
    dependencies: [
        .package(url: "https://github.com/matrix-org/matrix-rust-components-swift.git", exact: "26.05.13")
    ],
    targets: [
        .executableTarget(
            name: "MatrixSDKProbe",
            dependencies: [
                .product(name: "MatrixRustSDK", package: "matrix-rust-components-swift")
            ]
        )
    ]
)
