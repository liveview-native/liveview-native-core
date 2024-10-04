# LiveView Native Core

This repository contains an implementation of the LiveView Native core library,
which is intended to handle all the details which are common across the various
platforms on which LiveView Native is used.

This project is primarily meant for use as a static library via [`uniffi`]
generated client libraries. This project can also be used as a crate dependency
but it is not the first priority.

## Features

Currently, the features planned to be provided by the Core library consists of:

1. Connection management for the LiveView socket
2. Initializing a virtual DOM from the page received from the LiveView server,
   and associated parsing out of components, live views, and statics
3. Receipt, decoding, and application of changes sent by the LiveView server to
   the virtual DOM managed by Core
4. Management of callbacks/event handlers registered from the host language in
   response to virtual DOM changes

Host languages (e.g. Swift) can then bind to this library, and focus on the
details of rendering the UI, rather than on how to interact with the
LiveView server.

## Status

This library is not quite ready for production use yet, as there are still some
details of the API being finalized, and much testing remains to be done. We'll
plan to announce the library more publicly when it is ready for the community
to build with.

## Client libraries

This project generates bindings for Kotlin and Swift via [`uniffi`]. The Swift
package is bundled in with an xcframework with the pre-compiled static
libraries for all supported Apple targets. The Kotlin package is similar.


`C#` bindings with the goal of supporting [`WinUI3`] could be in the future of
this project.

[`WinUI3`]: https://learn.microsoft.com/en-us/windows/apps/winui/winui3/
[`uniffi`]: https://github.com/mozilla/uniffi-rs/

## Development

Building liveview-native-core for `watchOS`, `tvOS`, and `visionOS` depends on
the [`build-std`] unstable rust feature because they are [tier-3 rust targets]
thus the nightly rust compiler is required.  Various language nightly features
have been added at times but it's a goal to reduce those and only [use stable
language features].

To run tests, you will want to start the [test server] via:
```
mix deps.get && mix compile && mix phx.server
```

To run [`cargo test`, you need to have `kotlinx-coroutines-core-jvm.jar` and
`jna-5.14.0.jar` (or similar) in the
`CLASSPATH`](https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/.github/workflows/ci.yml#L334-L345).

[use stable language features]: https://github.com/liveview-native/phoenix-channels-client/issues/134
[test server]: https://github.com/liveview-native/liveview-native-core/tree/d15a0e1457a17c2f719d80f1d233b8648eb62904/tests/support/test_server.
[tier-3 rust targets]: https://doc.rust-lang.org/nightly/rustc/platform-support.html#tier-3

Rust integration tests are ran in various simulators via [`cargo-dinghy`].
Dinghy can be described as a _more or less cursed way_ to run tests. It
attaches to the simulator process via lldb to get the status code of the
program - this requires sudo in some cases. The current project setup uses
[custom target
runners](https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/.cargo/config.toml#L1-L26)
to enable the rust tests to be packaged into a apple simulator app. Annoyingly
this can be flakey in CI.


[`cargo-dinghy`]: https://github.com/sonos/dinghy

### Android/Kotlin Notes

For `cargo test` to succeed, you will need the [`kotlin` brew
package](https://formulae.brew.sh/formula/kotlin) or equivalent and possibly
[`ktlint`](https://formulae.brew.sh/formula/ktlint).

In the [`liveview-native-core-jetpack`
directory](https://github.com/liveview-native/liveview-native-core/tree/d15a0e1457a17c2f719d80f1d233b8648eb62904/crates/core/liveview-native-core-jetpack)
you'll find instructions/requirements for building the jetpack client.

### Apple/Swift Notes

Installation of the xcode and iOS/watchOS/tvOS/visionOS platforms is required
to build for various platforms.

This project currently relies heavily on [`cargo-make`] for the Apple target
builds (xcframework, swift package, etc.). It is subject to change but `cargo 
make uniffi-swift-test` will build the `xcframework` 
(`cargo make uniffi-xcframework`), package up the swift package (`cargo make 
uniffi-swift-package`) and test all the supported apple platforms - Swift,
Apple TV, Apple Watch, Apple Vision Pro, iPhone, MacOS.

At the moment, [`cargo make uniffi-swift-test` requires the
`CARGO_MAKE_TOOLCHAIN` to be
set](https://github.com/liveview-native/liveview-native-core/issues/185). This
is useful should you need have a custom toolchain of the rust compiler. Tier-3
rust targets can break from time to time.

The `Package.swift` is at the root of the project but the [package contents are
located in the `./crates/core/liveview-native-core-swift`
subdirectory](https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/Package.swift#L49-L65).
This is because swift packages are used as git repos and possibly do not
support subdirectory packages.

The various sub-tests `cargo make uniffi-swift-test-{package, macos, etc}` do
not build the swift package or the xcframework. This is is because the
[`swift_integration_tests`] run on an action matrix and depend on
[`swift_build_xcframework`].

[`cargo-make`]: https://github.com/sagiegurari/cargo-make
[`build-std`]: https://doc.rust-lang.org/cargo/reference/unstable.html#build-std
[`swift_integration_tests`]:
https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/.github/workflows/ci.yml#L209-L284
[`swift_build_xcframework`]:
https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/.github/workflows/ci.yml#L164-L207


## Releases

To create a Release, one must navigate to the Releases pane of the GitHub UI
and create a new pre-release with a tag. This will kick off the [release
CI](https://github.com/liveview-native/liveview-native-core/blob/d15a0e1457a17c2f719d80f1d233b8648eb62904/.github/workflows/release.yml#L3-L6)
which compiles the rust into many static libraries, generates swift and bundles
it all into a swift package that is consumable by
[`liveview-client-swiftui`](https://github.com/liveview-native/liveview-client-swiftui/).

## License

MIT. See `LICENSE.md`.
