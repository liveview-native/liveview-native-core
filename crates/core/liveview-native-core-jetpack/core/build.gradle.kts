plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
    id("org.mozilla.rust-android-gradle.rust-android")
    //id("idea")
    id("org.jetbrains.dokka") version "1.9.10"
}

val uniffiPath = "${buildDir}/generated/source/uniffi/java"
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
    ndkVersion = "25.1.8937393"
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
        //val sourceSet = sourceSets.find { it.name == this.name }
        //sourceSet.java.srcDir(uniffiPath)
        //java.sourceSets.java.srcDir(File(uniffiPath))
        // XXX: I've been trying to make this work but I can't, so the compiled bindings will show as "regular sources" in Android Studio.
        //idea.module.generatedSourceDirs.add(file("${uniffiPath}/uniffi"))
    }
}


dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.4.0")
    implementation("net.java.dev.jna:jna:5.7.0@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.6.4")
}

// Configuring Rust Cargo build
// https://github.com/mozilla/rust-android-gradle
cargo {
    verbose = true
    module = "../../"
    libname = "liveview_native_core"
    // In case you need to run the unit tests, install the respective toolchain and add the target below.
    targets = listOf("arm", "arm64", "x86", "x86_64", "darwin-aarch64")
    //targets = listOf("darwin-aarch64")
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
