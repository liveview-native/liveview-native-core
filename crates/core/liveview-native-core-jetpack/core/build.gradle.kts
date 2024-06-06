plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
    id("org.mozilla.rust-android-gradle.rust-android")
    // TODO: Add generated sources to dokka sourcesets
    id("org.jetbrains.dokka") version "1.9.10" apply true
}
subprojects {
    apply(plugin = "org.jetbrains.dokka")
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.6.4")
    implementation("net.java.dev.jna:jna:5.14.0@aar")

    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.7.3")
    testImplementation("net.java.dev.jna:jna:5.14.0")
    coreLibraryDesugaring("com.android.tools:desugar_jdk_libs:2.0.4")
}
val uniffiPath = "${buildDir}/generated/source/uniffi/java"
val os_name = System.getProperty("os.name").lowercase()
val is_linux = os_name.contains("linux")
val is_mac = os_name.contains("mac")
val lvn_version = "0.4.0-alpha-3"

android {
    namespace = "org.phoenixframework.liveview_native_core_jetpack"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
        /*
        create("releaseDesktop") {
        }
        */
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
        isCoreLibraryDesugaringEnabled = true
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    publishing {
        singleVariant("release") {
            withSourcesJar()
            withJavadocJar()
        }
        /*
        singleVariant("releaseDesktop") {
            withSourcesJar()
            withJavadocJar()
        }
        */
    }
    ndkVersion = "26.3.11579264"

    sourceSets {
        getByName("main") {
            java.srcDir(uniffiPath)
        }
        getByName("test") {
            resources.srcDirs("${buildDir}/rustJniLibs/desktop")
        }
    }

    libraryVariants.all {
        tasks.register<Exec>("build${name.capitalize()}StaticLib") {
            workingDir("${project.projectDir}")
            commandLine(
                "cargo",
                "build",
                "--lib",
                "-p",
                "liveview-native-core",
            )
        }
        val generateUniffi = tasks.register<Exec>("generate${name.capitalize()}UniFFIBindings") {
            workingDir("${project.projectDir}")
            var dylib_file = rootProject.file("../../../target/debug/libliveview_native_core.dylib")

            if (is_linux) {
                dylib_file = rootProject.file("../../../target/debug/libliveview_native_core.so")
            }

            commandLine(
                "cargo",
                "run",
                "-p",
                "uniffi-bindgen",
                "--",
                "generate",
                "--library",
                dylib_file,
                "--language",
                "kotlin",
                "--out-dir",
                uniffiPath
            )
        }
        javaCompileProvider.get().dependsOn(generateUniffi)
    }
}

var cargo_targets = listOf(
    "arm", // rust - armv7-linux-androideabi
    "arm64", // rust - aarch64-linux-android
    "x86", // rust - i686-linux-android
    "x86_64", // rust - x86_64-linux-android
)
if (is_linux) {
    cargo_targets += "linux-x86-64"  // x86_64-unknown-linux-gnu
}
if (is_mac) {
    cargo_targets += "darwin-aarch64" // rust - aarch64-apple-darwin
    cargo_targets += "darwin-x86-64" // rust - x86_64-apple-darwin
}

// Configuring Rust Cargo build
// https://github.com/mozilla/rust-android-gradle
cargo {
    verbose = true

    apiLevel = 21

    module = "../../../../"

    libname = "liveview_native_core"
    // In case you need to run the unit tests, install the respective toolchain and add the target below.
    targets = cargo_targets
}

// Running cargo command before build
tasks.configureEach {
    if ((name == "javaPreCompileDebug" || name == "javaPreCompileRelease")) {
        dependsOn("cargoBuild")
        dependsOn("generateDebugUniFFIBindings")
    }
    if (name == "cargoBuild") {
        dependsOn("generateDebugUniFFIBindings")
    }
    if (name == "generateDebugUniFFIBindings") {
        dependsOn("buildDebugStaticLib")
    }
}

publishing {
    publications {
        register<MavenPublication>("release")  {
            groupId = "org.phoenixframework"
            artifactId = "liveview-native-core-jetpack"
            version = lvn_version

            afterEvaluate {
                from(components["release"])
            }
        }
    }
    /*
    publications {
        register<MavenPublication>("releaseDesktop")  {
            groupId = "org.phoenixframework"
            artifactId = "liveview-native-core-jetpack-desktop"
            version = lvn_version

            afterEvaluate {
                from(components["releaseDesktop"])
            }
        }
    }
    */
}
