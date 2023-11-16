plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
    id("org.mozilla.rust-android-gradle.rust-android")
    id("org.jetbrains.dokka") version "1.9.10" apply true
}
subprojects {
    apply(plugin = "org.jetbrains.dokka")
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.4.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.6.4")
    implementation("net.java.dev.jna:jna:5.13.0")
}
val uniffiPath = "${buildDir}/generated/source/uniffi/java"

/*
dokkaHtml.configure {
    dokkaSourceSets {
        named("main") {
            noAndroidSdkLink.set(false)
            java.srcDir(uniffiPath)
        }
    }
}
kotlin {
    sourceSets {
        named("main") {
            //java.srcDir(uniffiPath)
        }
        configureEach {
            sourceRoots.from(uniffiPath)
        }
    }
}
*/

android {
    namespace = "org.phoenixframework.liveview_native_core_jetpack"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    publishing {
        singleVariant("release") {
            withSourcesJar()
            withJavadocJar()
        }
    }
    ndkVersion = "25.2.9519653"

    sourceSets {
        getByName("main") {
            java.srcDir(uniffiPath)
        }
    }

    libraryVariants.all {
        val t = tasks.register<Exec>("generate${name.capitalize()}UniFFIBindings") {
            workingDir("${project.projectDir}")
            // Runs the bindings generation, note that you must have uniffi-bindgen installed and in your PATH environment variable
            commandLine(
                "cargo",
                "run",
                "--bin",
                "uniffi-bindgen",
                "--",
                "generate",
                rootProject.file("../src/uniffi.udl"),
                "--language",
                "kotlin",
                // TODO: Try out different config options for kotlin with uniffi
                "--config",
                rootProject.file("../uniffi.toml"),
                "--out-dir",
                uniffiPath
            )
        }
        javaCompileProvider.get().dependsOn(t)
    }
}

// Configuring Rust Cargo build
// https://github.com/mozilla/rust-android-gradle
cargo {
    verbose = true

    module = "../../../../"

    libname = "liveview_native_core"
    // In case you need to run the unit tests, install the respective toolchain and add the target below.
    targets = listOf(
        "arm", // rust - armv7-linux-androideabi
        "arm64", // rust - aarch64-linux-android
        "x86", // rust - i686-linux-android
        "x86_64", // rust - x86_64-linux-android
        "darwin-aarch64", // rust - aarch64-apple-darwin
        "darwin-x86-64", // rust - x86_64-apple-darwin
    )
}

// Running cargo command before build
tasks.configureEach {
    if ((name == "javaPreCompileDebug" || name == "javaPreCompileRelease")) {
        dependsOn("cargoBuild")
    }
}

// Configuring Java Lib Path in order to find the native library before running the Unit Tests
tasks.withType<Test>().configureEach {
    doFirst {
        val rustJniLibsForDesktopDir = File("${projectDir}/build/rustJniLibs/desktop")
        val archTypesSubdirs = rustJniLibsForDesktopDir.listFiles()
        for (dir in archTypesSubdirs) {
            // Selecting the proper JNI lib file for run the unit tests
            // in according to the architecture. e.g.: darwin-aarch64, darwin-x86-64
            val arch = System.getProperty("os.arch").replace("_", "-")
            if (dir.isDirectory && dir.name.contains(arch)) {
                systemProperty("java.library.path", dir.absolutePath)
                break
            }
        }
    }
}

publishing {
    publications {
        register<MavenPublication>("release")  {
            groupId = "org.phoenixframework"
            artifactId = "liveview-native-core-jetpack"
            version = "0.1.0-pre-alpha-08"

            afterEvaluate {
                from(components["release"])
            }
        }
    }
}
