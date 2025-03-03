allprojects {
    configurations.all {
        resolutionStrategy.eachDependency {
            if (this.requested.group == "org.jetbrains.kotlin") {
                this.useVersion("1.9.21")
                because("compatibility with client version")
            }
        }
    }
}

plugins {
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.rust.android.gradle) apply false
    alias(libs.plugins.dokka) apply true
}
