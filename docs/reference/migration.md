# Migration Guide

Step-by-step instructions for upgrading between Soldatik3D versions.

## Migrating to 0.1.0 (From Pre-Release)

If you were using a development version before 0.1.0, here are the required changes.

### 1. Update Cargo.toml

**Before:**
```toml
[dependencies]
runtime = { path = "../soldatik3D/crates/runtime" }
```

**After:**
```toml
[dependencies]
soldatik3d = { path = "../../../.." }  # Or version from crates.io

[features]
default = ["vulkan", "opengl"]
vulkan = ["soldatik3d/vulkan"]
opengl = ["soldatik3d/opengl"]
```

**Crate Type (Required for Android):**
```toml
[lib]
crate-type = ["cdylib", "rlib"]  # Was just "rlib" or "bin"
```

### 2. Update Game Entry Point

**Before (Desktop only):**
```rust
use runtime::{run, Game, GameConfig, UpdateContext, World};

fn main() -> anyhow::Result<()> {
    run(MyGame)
}
```

**After (Cross-platform):**
```rust
use soldatik3d::{run, run_android, Game, GameConfig, UpdateContext, World};

fn main() -> anyhow::Result<()> {
    run(MyGame)
}

// Android entry point - REQUIRED for Android builds
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    if let Err(e) = run_android(MyGame, app) {
        eprintln!("Game failed: {e:?}");
    }
}
```

### 3. Update GameConfig Usage

**Before:**
```rust
fn config(&self) -> GameConfig {
    GameConfig {
        title: "My Game".into(),
        width: 1280,
        height: 720,
        fullscreen: false,
        vsync: true,
        backend: BackendKind::Vulkan,
        window_hints: WindowHints::default(),
        max_frames: None,
        startup_commands: vec![],
        open_console: false,
    }
}
```

**After (Builder Pattern):**
```rust
fn config(&self) -> GameConfig {
    GameConfig::default()
        .with_title("My Game")
        .with_window_size(1280, 720)
        .with_fullscreen(false)
        .with_vsync(true)
        .with_backend(BackendKind::Vulkan)
        .with_window_hints(WindowHints {
            resizable: true,
            cursor_visible: false,
            cursor_locked: true,
            ..Default::default()
        })
        .with_max_frames(None)
        .with_startup_commands(vec!["vsync.off".into()])
        .with_open_console(true)
}
```

### 4. Update Imports

**Before:**
```rust
use runtime::{Game, GameConfig, UpdateContext, World, Camera, Light, Material, Mesh};
use runtime::platform::{Input, KeyCode, MouseButton};
use runtime::render::{Renderer, DebugView, VsyncMode};
```

**After:**
```rust
use soldatik3d::{
    run,
    #[cfg(target_os = "android")]
    run_android,
    Game,
    GameConfig,
    UpdateContext,
    World,
    Camera,
    Light,
    Material,
    Mesh,
    Input,
    KeyCode,
    MouseButton,
    Renderer,
    DebugView,
    VsyncMode,
    BackendKind,
    WindowHints,
    Vec3, Vec2, Color, Mat4, Quat,
};
```

### 5. Update Android Build

**build.rs** - Add Android NDK setup:
```rust
fn main() {
    // ... existing code ...
    
    if std::env::var("TARGET").unwrap_or_default().contains("android") {
        setup_android_ndk();
    }
}

fn setup_android_ndk() {
    // See templates/game-project/build.rs for full implementation
}
```

**Add `build_apk.sh`** - Copy from template.

**Add Android Gradle Template** - Copy from `scripts/apk/gradle/` (engine repo) or use the template's included `android/gradle/` (if created with `s3d new`).

### 6. Update .cargo/config.toml

**Before:**
```toml
# May have had target-specific linkers
```

**After:**
```toml
# Clean config - cross-compilation via environment
[net]
git-fetch-with-cli = true

[profile.dev]
opt-level = 3
debug = true
split-debuginfo = "unpacked"

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "debuginfo"
debug = false
```

Cross-compilation now handled via:
- `cross_build.sh` script
- `soldatik3d build --target <platform>`
- CI/CD environment variables

### 7. Update Shader Paths

**Before:**
```rust
include_str!("shaders/world.vert")
```

**After:**
```rust
// Vulkan
include_str!("../../crates/vulkan/shaders/world.vert")

// OpenGL
include_str!("../../crates/opengl/shaders/world.vert")
```

Or use the engine's built-in shaders (recommended).

### 8. Update Console Commands

**Before:**
```rust
// Direct console manipulation
ctx.console.print("Hello");
```

**After:**
```rust
// Same, plus command registration
ctx.commands.register(Command::new(
    "mycmd",
    "Description",
    |ctx| {
        ctx.console.print("Hello!");
        CommandResult::Ok(None)
    },
));
```

### 9. Add CLI Tool (Optional)

```bash
cargo install --path tools/soldatik3d-cli
# Or from crates.io when published
# cargo install soldatik3d-cli
```

### 10. Update CI/CD

**GitHub Actions** - Update to new workflow files:
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

Remove old `.github/workflows/android.yml` (merged into ci.yml).

---

## Migrating Between 0.1.x Versions

### 0.1.0 → 0.1.1 (Example)

When patch versions are released:

1. **Update dependency:**
   ```toml
   soldatik3d = "0.1.1"
   ```

2. **Run cargo update:**
   ```bash
   cargo update -p soldatik3d
   ```

3. **Check changelog** for any deprecation warnings.

4. **Re-build:**
   ```bash
   cargo clean && cargo build
   ```

### Breaking Changes in Minor Versions

Since we're in 0.x, minor versions (0.1 → 0.2) MAY contain breaking changes. Always check:

1. **CHANGELOG.md** - Breaking changes section
2. **Migration guide** for that version
3. **Compile errors** after update
4. **Deprecation warnings** - `cargo build` output

### Automated Migration

For large codebases, consider:

```bash
# 1. Create backup
git commit -am "Pre-migration backup"

# 2. Update dependency
cargo update -p soldatik3d

# 3. Fix compile errors
cargo build 2>&1 | grep -E "error\[" | head -20

# 4. Run tests
cargo test

# 5. Run game
cargo run
```

### IDE Assistance

Use `rust-analyzer` for:
- Auto-import updates
- Deprecation warnings
- Type mismatches
- Unused imports

---

## Platform-Specific Migration

### Android

| Aspect | Old | New |
|--------|-----|-----|
| Entry point | `#[no_mangle] fn main()` | `android_main(app: AndroidApp)` |
| Function | `run()` | `run_android()` |
| Crate type | `rlib`/`bin` | `cdylib` + `rlib` |
| Manifest | Manual | Auto via template |
| APK build | Manual | `build_apk.sh` / `soldatik3d apk` |

### Windows

| Aspect | Old | New |
|--------|-----|-----|
| Backend | Vulkan only | Vulkan + OpenGL |
| Cross-compile | Manual | `cross_build.sh` / `soldatik3d build --target windows` |
| Linker | Manual config | Auto via `.cargo/config.toml` |

### Linux

| Aspect | Old | New |
|--------|-----|-----|
| System deps | Manual | Documented in CI |
| shaderc | System only | System + fallback |
| Validation | Manual | `SOLDATIK3D_VALIDATION=1` |

---

## Common Migration Issues

### "crate `runtime` not found"
→ Change `runtime` to `soldatik3d` in Cargo.toml and imports

### "function `run_android` not found"
→ Import from `soldatik3d`: `use soldatik3d::run_android;`

### "trait `Game` not implemented"
→ Check `Game` trait signature: `config`, `create_world`, `update`

### "Android build fails: linker not found"
→ Set `ANDROID_NDK_HOME`, verify NDK r26c+, check `build.rs`

### "Windows cross-compile fails: shaderc"
→ Use `--no-default-features --features opengl` for MinGW

### "crate type `cdylib` required"
→ Add `crate-type = ["cdylib", "rlib"]` to `[lib]` in Cargo.toml

### "Command `soldatik3d` not found"
→ Install CLI: `cargo install --path tools/soldatik3d-cli`

---

## Verification Checklist

After migration, verify:

- [ ] `cargo build` succeeds (all targets)
- [ ] `cargo test` passes
- [ ] `cargo run` works on desktop
- [ ] `soldatik3d build --target android` works
- [ ] `soldatik3d apk arm64-v8a` produces APK
- [ ] `adb install` and run on device
- [ ] Console opens with `` ` ``
- [ ] Built-in commands work (`help`, `vsync.off`, `screenshot`)
- [ ] Custom commands work
- [ ] `soldatik3d export` produces packages
- [ ] CI/CD passes (GitHub Actions)

---

## Getting Help

If migration fails:

1. Check [GitHub Issues](https://github.com/finesoldat1k/soldatik3D/issues)
2. Search existing issues
3. Create new issue with:
   - Soldatik3D version
   - Rust version (`rustc --version`)
   - Platform
   - Error message
   - Minimal reproduction

---

*See [Changelog](changelog.md) for version history, or [Installation](getting-started/installation.md) for fresh install guide.*