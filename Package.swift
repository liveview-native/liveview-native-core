// swift-tools-version: 5.6
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription
let liveview_native_core_framework: Target

// To relase, toggle this to `false`
let useLocalFramework = true
if useLocalFramework {
    liveview_native_core_framework = .binaryTarget(
        name: "liveview_native_core",
        path: "./target/uniffi/swift/liveview_native_core.xcframework"
    )
} else {
    let releaseTag = "0.3.0-alpha"
    let releaseChecksum = "f3972f4d40732c884c98426b28550376abaff20a3490b73367ad170f1f0bcca9"
    liveview_native_core_framework = .binaryTarget(
        name: "liveview_native_core",
        url: "https://github.com/liveview-native/liveview-native-core/releases/download/\(releaseTag)/liveview_native_core.xcframework.zip",
        checksum: releaseChecksum
    )
}


let package = Package(
    name: "LiveViewNativeCore",
    platforms: [
        .iOS("16.0"),
        .macOS("13.0"),
        .watchOS("9.0"),
        .tvOS("16.0"),
    ],
    products: [
        .library(
            name: "LiveViewNativeCore",
            targets: [
                "liveview_native_core",
                "LiveViewNativeCore"
            ]
        ),
    ],
    dependencies: [
        // This is used to generate documentation vio `swift package generate-documentation`
        // This doesn't work because of:
        // https://github.com/apple/swift-docc-plugin/issues/50 will hopefully resolve it
        .package(url: "https://github.com/apple/swift-docc-plugin", from: "1.0.0"),
    ],
    targets: [
        liveview_native_core_framework,
        .target(
            name: "LiveViewNativeCore",
            dependencies: [
                .target(name: "liveview_native_core")
            ],
            path: "./crates/core/liveview-native-core-swift/Sources/LiveViewNativeCore/"
        ),
        .testTarget(
            name: "LiveViewNativeCoreTests",
            dependencies: [
                "LiveViewNativeCore"
            ],
            path: "./crates/core/liveview-native-core-swift/Tests/LiveViewNativeCoreTests/"
        ),
    ]
)
