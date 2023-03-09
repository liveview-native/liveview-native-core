# Build On Android

To build on android:

1. Specify your linkers in the [config.toml](../../.cargo/config.toml) file. For instance:

```toml
[target.aarch64-linux-android]
linker = "AndroidSdk\\ndk-bundle\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\aarch64-linux-android30-clang++.cmd" #windows
linker = "AndroidSdk/ndk-bundle/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android30-clang++" # linux/ mac

[target.armv7-linux-androideabi]
linker = "AndroidSdk\\ndk-bundle\\toolchains\\llvm\\prebuilt\\windows-x86_64\\bin\\armv7a-linux-androideabi30-clang++.cmd" #windows
linker = "AndroidSdk/ndk-bundle/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi30-clang++" # linux/ mac
```

2. ### Build the library

   Run the following commands in the [core](../../../core) directory:

```shell
$ RUSTC_BOOTSTRAP=1 cargo build --release --target=aarch64-linux-android 
$ RUSTC_BOOTSTRAP=1 cargo build --release --target=armv7-linux-androideabi
```

The **.so** file generated is found in the `target/{arc}/release` folder. Replace `{arch}` with
the specified target. In this case `aarch64-linux-android` or `aarch64-linux-android`
e.g `target/aarch64-linux-android/release`

3. Move both **.so** files to the `jniLibs` (liveview-android/src/main/jniLibs) folder of your project and use.