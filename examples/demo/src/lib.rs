//! Soldatik3D demo game.
//!
//! This crate depends only on the public `runtime` facade. It owns all gameplay
//! code: the procedural level, first-person controller, camera presets and
//! debug interface. Camera spawning and selection are engine features exposed
//! through [`runtime::World`] and the console.

pub mod debug_ui;
pub mod demo;
pub mod level;
pub mod physics;
pub mod player;

pub use demo::DemoGame;

/// Runs the demo through the engine runtime.
pub fn run() -> anyhow::Result<()> {
    runtime::run(DemoGame::new())
}

/// Native-activity entry point for Android.
///
/// `android-activity` requires a Rust function with this exact name and
/// signature; it is called on the dedicated Android main thread. The demo is
/// built as a `cdylib` (`libdemo.so`) and loaded by `android.app.NativeActivity`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    use runtime::winit::platform::android::activity::WindowManagerFlags;

    // winit 0.30 cannot toggle Android fullscreen through the window API, so
    // the native activity flags are the only way to hide the system bars and
    // keep the display awake during play.
    app.set_window_flags(
        WindowManagerFlags::FULLSCREEN | WindowManagerFlags::KEEP_SCREEN_ON,
        WindowManagerFlags::FORCE_NOT_FULLSCREEN,
    );
    if let Err(error) = runtime::run_android(DemoGame::new(), app) {
        eprintln!("[demo] android_main failed: {error:?}");
    }
}
