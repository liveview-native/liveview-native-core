# LiveView Native Core Jetpack

This library provides an abstraction layer on top of the [LiveView Native Core](https://github.com/liveview-native/liveview-native-core) library.

## Pre-requisites

In order to build this library, it's necessary to do the following steps:
- Use the latest version of [Android Studio](https://developer.android.com/studio) with [NDK](https://developer.android.com/studio/projects/install-ndk).
- This project contains Rust files which depends on **LiveView Core library** and exposes functionality to the Kotlin layer via [JNI](https://docs.oracle.com/javase/7/docs/technotes/guides/jni/spec/jniTOC.html). Therefore, you need to [install Rust](https://www.rust-lang.org/tools/install).
- After installing Rust, you'll need to install the toolchains for each platform which the library will be generated *(arm, arm64, x86, x86_64, darwin-x86-64, darwin-aarch64)*. This project is using [Rust Gradle Plugin](https://github.com/mozilla/rust-android-gradle), therefore follow the steps described in the corresponding section in their website. For instance:
- Ensure your classpath points to both the proper NDK and kotlinx-coroutines e.g. `export CLASSPATH = "../path/to/jna-5.14.0.jar:/..path/to/kotlinx-coroutines-core-jvm.jar"`, typically on MacOs the kotlinx coroutines are located in `opt/homebrew/opt/kotlin/libexec/lib` if you installed with homebrew.
- Ensure that you have a python version installed between 3.10 and 3.12 accessible as `python` in your environment, or use the override below.
```
rustup target add armv7-linux-androideabi   # for arm
rustup target add i686-linux-android        # for x86
rustup target add aarch64-linux-android     # for aarch64
rustup target add x86_64-linux-android      # for x86
export RUST_ANDROID_GRADLE_PYTHON_COMMAND=python3.12 # or some other version less than 3.13
```

## Building the library

In order to generate the [Android Archive](https://developer.android.com/studio/projects/android-library) (`*.aar`) file, use the command below:
```
./gradlew assembleRelease
```

## Releasing a new version of the library

This library is hosted in [Jitpack](https://jitpack.io/) and the whole build process is automated.
In order to generate a new version of the library, you just need to open a PR containing the changes and update the library version in the [build.gradle.kts](core/build.gradle.kts) file (see the `version` field in `publishing` task).
After approved and merged, [create a new release](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository) here in GitHub.
