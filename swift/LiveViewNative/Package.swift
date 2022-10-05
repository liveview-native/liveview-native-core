// swift-tools-version: 5.6
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "LiveViewNative",
    products: [
        .library(
            name: "LiveViewNative",
            targets: ["LiveViewNative"]),
    ],
    dependencies: [
        // Dependencies declare other packages that this package depends on.
        // .package(url: /* package url */, from: "1.0.0"),
    ],
    targets: [
        .binaryTarget(name: "LiveViewNativeCore", path: "LiveViewNativeCore.xcframework"),
        .target(
            name: "LiveViewNative",
            dependencies: [
                .target(name: "LiveViewNativeCore")
            ]),
        .testTarget(
            name: "LiveViewNativeTests",
            dependencies: ["LiveViewNative"]),
    ]
)
