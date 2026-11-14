//! Build script for Soldatik3D game projects
//!
//! This script handles platform-specific setup:
//! - Copies assets to the correct location for each platform
//! - Sets up Android NDK linker environment variables
//! - Generates platform-specific configuration

use std::env;
use std::path::{Path, PathBuf};
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");

    let target = env::var("TARGET").unwrap_or_default();
    let profile = env::var("PROFILE").unwrap_or_default();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap_or_default());

    // Handle Android-specific setup
    if target.contains("android") {
        setup_android(&target);
    }

    // Copy assets to output directory
    copy_assets(&target, &profile, &out_dir);

    // Print helpful information
    println!("cargo:warning=Building for target: {target}, profile: {profile}");
}

fn setup_android(target: &str) {
    // Check for ANDROID_NDK_HOME
    if env::var("ANDROID_NDK_HOME").is_err() {
        println!("cargo:warning=ANDROID_NDK_HOME not set. Android build may fail.");
        println!("cargo:warning=Set it to your NDK path, e.g.:");
        println!("cargo:warning=  export ANDROID_NDK_HOME=$HOME/android-ndk-r26c");
    } else {
        let ndk_home = env::var("ANDROID_NDK_HOME").unwrap();
        let api = "29";

        let (linker, cc, ar) = match target {
            "aarch64-linux-android" => (
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar", ndk_home),
            ),
            "armv7-linux-androideabi" => (
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar", ndk_home),
            ),
            "x86_64-linux-android" => (
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android{}-clang", ndk_home, api),
                format!("{}/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar", ndk_home),
            ),
            _ => return,
        };

        // Set linker environment variables
        let target_upper = target.replace('-', "_").to_uppercase();
        println!("cargo:rustc-env=CARGO_TARGET_{}_LINKER={}", target_upper, linker);
        println!("cargo:rustc-env=CC_{}={}", target_upper, cc);
        println!("cargo:rustc-env=AR_{}={}", target_upper, ar);
    }
}

fn copy_assets(target: &str, profile: &str, out_dir: &Path) {
    let assets_src = Path::new("assets");
    if !assets_src.exists() {
        return;
    }

    // Determine output directory based on target and profile
    let mut assets_dst = if target.contains("android") {
        // For Android, assets go in the APK's assets folder (handled by Gradle)
        // But we also copy to target for local testing
        PathBuf::from("target").join(&target).join(&profile).join("assets")
    } else {
        PathBuf::from("target").join(&profile).join("assets")
    };

    if let Err(e) = copy_dir_all(assets_src, &assets_dst) {
        println!("cargo:warning=Failed to copy assets: {}", e);
    } else {
        println!("cargo:warning=Assets copied to: {}", assets_dst.display());
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}