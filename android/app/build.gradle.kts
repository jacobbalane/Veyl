plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "io.github.jacobbalane.veyl"
    compileSdk {
        version = release(37) { minorApiLevel = 2 }
    }
    defaultConfig {
        applicationId = "io.github.jacobbalane.veyl"
        minSdk = 26
        targetSdk = 37
        versionCode = providers.gradleProperty("VERSION_CODE").get().toInt()
        versionName = providers.gradleProperty("VERSION_NAME").get()
    }
    signingConfigs {
        create("release") {
            val path = System.getenv("VEYL_KEYSTORE_PATH")
            if (!path.isNullOrBlank()) {
                storeFile = file(path)
                storePassword = System.getenv("VEYL_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("VEYL_KEY_ALIAS")
                keyPassword = System.getenv("VEYL_KEY_PASSWORD")
                storeType = "pkcs12"
            }
        }
    }
    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            resValue("string", "app_name", "Veyl (debug)")
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = if (System.getenv("VEYL_KEYSTORE_PATH").isNullOrBlank()) null
                else signingConfigs.getByName("release")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
        buildConfig = true
        resValues = true
    }
    lint {
        abortOnError = true
        warningsAsErrors = true
        baseline = file("lint-baseline.xml")
    }
}

dependencies {
    implementation(project(":core"))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.ui.tooling.preview)
    debugImplementation(libs.androidx.compose.ui.tooling)
    testImplementation(libs.junit)
}
