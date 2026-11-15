plugins {
    id("com.android.application")
}

// The native library name is supplied by build_apk.sh / CI. It must match the
// `[lib] name` Cargo uses for the game crate (with `-` replaced by `_`).
val nativeLibName = System.getenv("SOLDATIK3D_ANDROID_LIB_NAME") ?: "demo"
val appId = System.getenv("SOLDATIK3D_ANDROID_APP_ID") ?: "com.soldatik3d.game"
val gameTitle = System.getenv("SOLDATIK3D_ANDROID_APP_NAME") ?: "Soldatik3D Game"

android {
    namespace = "com.soldatik3d.game"
    compileSdk = 34

    defaultConfig {
        applicationId = appId
        minSdk = 29
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        manifestPlaceholders["soldatik3dLibName"] = nativeLibName
        resValue("string", "app_name", gameTitle)
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // A release build must be installable out of the box. When a
            // dedicated keystore is configured, use it; otherwise fall back to
            // the debug key (which is what CI does for unsigned projects).
            val keystore = System.getenv("SOLDATIK3D_KEYSTORE")
            signingConfig = if (keystore.isNullOrBlank()) {
                signingConfigs.getByName("debug")
            } else {
                signingConfigs.create("release") {
                    storeFile = file(keystore)
                    storePassword = System.getenv("SOLDATIK3D_KEYSTORE_PASSWORD")
                    keyAlias = System.getenv("SOLDATIK3D_KEY_ALIAS")
                    keyPassword = System.getenv("SOLDATIK3D_KEY_PASSWORD")
                }
            }
        }
        debug {
            isDebuggable = true
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    // AGP already includes app/src/main/jniLibs, which is where build_apk.sh
    // and CI place the native library.

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

// This is a pure native-activity app: there is no Java/Kotlin code and no
// AndroidX runtimes to pull in. Keeping the dependency list empty also keeps
// the APK small and avoids manifest-merger surprises.
dependencies {
}
