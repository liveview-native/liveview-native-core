// swift-tools-version: 5.6
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "LiveViewNativeCore",
    products: [
        .library(
            name: "LiveViewNativeCore",
            targets: ["LiveViewNativeCore"]),
    ],
    dependencies: [
        // This is used to generate documentation vio `swift package generate-documentation`
        // This doesn't work because of:
        // https://github.com/apple/swift-docc-plugin/issues/50 will hopefully resolve it
        .package(url: "https://github.com/apple/swift-docc-plugin", from: "1.0.0"),
    ],
    targets: [
        .binaryTarget(name: "RustFramework", path: "../../../target/uniffi/swift/RustFramework.xcframework"),
        .target(
            name: "LiveViewNativeCore",
            dependencies: [
                .target(name: "RustFramework")
            ]
        ),
        .testTarget(
            name: "LiveViewNativeCoreTests",
            dependencies: ["LiveViewNativeCore"]),
    ]
)
