//! Build script for Soldatik3D game projects.
//!
//! Android APKs need game assets inside the Gradle project. Desktop builds
//! load assets from the working directory, so they need no build-time copy.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/");

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("android") {
        return;
    }

    copy_assets_to_android();
}

fn copy_assets_to_android() {
    let assets = Path::new("assets");
    if !assets.is_dir() {
        return;
    }

    let destination = Path::new("android/gradle/app/src/main/assets");
    if let Err(error) = copy_dir_all(assets, destination) {
        println!("cargo:warning=Failed to copy assets into the Android project: {error}");
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }
    Ok(())
}
