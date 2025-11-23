// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SoksakBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "SoksakBridge",
            type: .static,
            targets: ["SoksakBridge"]),
    ],
    dependencies: [
        .package(url: "https://github.com/argmaxinc/WhisperKit", from: "0.15.0")
    ],
    targets: [
        .target(
            name: "SoksakBridge",
            dependencies: [
                .product(name: "WhisperKit", package: "WhisperKit")
            ]
        ),
    ]
)
