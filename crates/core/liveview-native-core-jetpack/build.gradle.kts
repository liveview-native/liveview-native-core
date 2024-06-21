plugins {
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.rust.android.gradle) apply false
    alias(libs.plugins.dokka) apply true
}
