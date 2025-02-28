plugins {
    alias(libs.plugins.rust.android.gradle)
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    id("maven-publish")
    // TODO: Add generated sources to dokka sourcesets
    alias(libs.plugins.dokka) apply true
}

subprojects { apply(plugin = "org.jetbrains.dokka") }

dependencies {
    implementation(libs.org.jetbrains.kotlinx.coroutines.core)
    compileOnly(libs.net.java.dev.jna) { artifact { type = "aar" } }
    testImplementation(libs.net.java.dev.jna)

    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.espresso.core)

    testImplementation(libs.junit)
    testImplementation(libs.org.jetbrains.kotlinx.coroutines.test)
    coreLibraryDesugaring(libs.com.android.tools.desugar)
}

val uniffiPath = "${layout.buildDirectory}/generated/source/uniffi/java"
val os_name = System.getProperty("os.name").lowercase()
val is_linux = os_name.contains("linux")
val is_mac = os_name.contains("mac")
val lvn_version = "0.4.1-rc-1"

android {
    namespace = "org.phoenixframework.liveview_native_core_jetpack"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        getByName("release") { isMinifyEnabled = false }
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
    kotlinOptions { jvmTarget = "1.8" }
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
        getByName("main") { java.srcDir(uniffiPath) }
        getByName("test") { resources.srcDirs("${layout.buildDirectory}/rustJniLibs/desktop") }
    }

    libraryVariants.all {
        tasks.register<Exec>("build${name.replaceFirstChar { c -> c.uppercase() }}StaticLib") {
            workingDir("${project.projectDir}")
            commandLine(
                    "cargo",
                    "build",
                    "--lib",
                    "-p",
                    "liveview-native-core",
            )
        }
        val generateUniffi =
                tasks.register<Exec>(
                        "generate${name.replaceFirstChar { c -> c.uppercase() }}UniFFIBindings"
                ) {
                    workingDir("${project.projectDir}")
                    var dylib_file =
                            rootProject.file("../../../target/debug/libliveview_native_core.dylib")

                    if (is_linux) {
                        dylib_file =
                                rootProject.file("../../../target/debug/libliveview_native_core.so")
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

var cargo_targets =
        listOf(
                "arm", // rust - armv7-linux-androideabi
                "arm64", // rust - aarch64-linux-android
                "x86", // rust - i686-linux-android
                "x86_64", // rust - x86_64-linux-android
        )

if (is_linux) {
    cargo_targets += "linux-x86-64" // x86_64-unknown-linux-gnu
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
    // In case you need to run the unit tests, install the respective toolchain and add the target
    // below.
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
// https://github.com/mozilla/rust-android-gradle/issues/118#issuecomment-1569407058
tasks.whenObjectAdded {
    if ((this.name == "mergeDebugJniLibFolders" || this.name == "mergeReleaseJniLibFolders")) {
        this.dependsOn("cargoBuild")
        // fix mergeDebugJniLibFolders  UP-TO-DATE
        val dir = layout.buildDirectory.asFile.get()
        this.inputs.dir(dir.resolve("rustJniLibs/android"))
    }
}

publishing {
    publications {
        register<MavenPublication>("release") {
            groupId = "org.phoenixframework"
            artifactId = "liveview-native-core-jetpack"
            version = lvn_version

            afterEvaluate { from(components["release"]) }
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
