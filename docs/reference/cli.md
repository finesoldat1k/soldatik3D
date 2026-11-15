# `s3d` CLI Reference

`s3d` is the Soldatik3D development tool. It creates projects, invokes Cargo for
cross-target builds, packages APKs and copies artifacts. It is a host tool and
is not a dependency of games.

```bash
cargo build --release -p s3d
./target/release/s3d --help
```

Global option:

| Option | Effect |
|--------|--------|
| `-v`, `--verbose` | Verbose output on commands that support it |

Commands must run in the right directory:

| Command | Working directory |
|---------|-------------------|
| `new` | Any |
| `build`, `run`, `apk`, `export`, `check` | Generated game project root (must contain `Cargo.toml`) |
| `info`, `engine` | Any |

## `s3d new`

```text
Usage: s3d new [OPTIONS] <NAME>

Arguments:
  <NAME>  Project name (directory name)

Options:
  -d, --dir <DIR>            Target directory (default: current directory)
  -t, --template <TEMPLATE>  Template path (default: built-in templates/game)
```

What it does:

1. Normalizes the name to a snake_case Cargo package (`My-Game` → `my_game`)
   and a PascalCase Rust type (`MyGame`).
2. Creates the target directory plus `src/`, `assets/`, `shaders/` and
   `.cargo/`.
3. Copies `templates/game` (or `--template`) with `my_game` and `MyGame`
   replaced throughout text files.
4. Rewrites the template's `runtime` path dependency so it points at the engine
   checkout that created the project (`SOLDATIK3D_ENGINE` overrides this).
5. Runs `git init`, `git add -A` and an initial commit.

Template lookup order:

1. `--template <path>`
2. `$CARGO_MANIFEST_DIR/../../templates/game`
3. `/usr/share/s3d/templates/game`
4. `$XDG_DATA_HOME/s3d/templates/game`
5. `../templates/game` and `../../templates/game` relative to the executable

If no candidate contains `Cargo.toml`, the command fails with
`Template directory not found`.

```bash
s3d new my_game
s3d new "My Game" --dir ~/projects
s3d new custom --template /path/to/template
```

The generated project is standalone: it has its own `[workspace]` table so it
can live inside or outside the engine repository.

## `s3d build`

```text
Usage: s3d build [OPTIONS] [TARGET]

Arguments:
  [TARGET]  linux | windows | android | all   [default: linux]

Options:
  -r, --release              Build in release mode
  -f, --features <FEATURES>  Cargo features        [default: vulkan,opengl]
  -o, --out-dir <OUT_DIR>    Copy the artifact here
```

Behavior by target:

| Target | Cargo target | Features used | Artifact |
|--------|--------------|---------------|----------|
| `linux` | host (Linux only) | `--features` value | `target/<profile>/<package>` |
| `windows` | host on Windows; `x86_64-pc-windows-gnu` on Linux | host features on Windows; `opengl` (forced) on Linux | `target/<profile>/<package>.exe` or `target/x86_64-pc-windows-gnu/<profile>/<package>.exe` |
| `android` | `aarch64-linux-android` + `armv7-linux-androideabi` | `opengl` (forced) | `target/<triple>/<profile>/lib<package>.so` |
| `all` | Linux + Windows + Android on Linux; Windows + Android on Windows | as above | all locally buildable artifacts |

On Linux, Windows builds first check for `x86_64-w64-mingw32-gcc` and fail with
an install hint if MinGW is missing. On Windows, the native MSVC target is used
and both backends are available. Linux builds are rejected on a Windows host.
Android builds require an NDK via `ANDROID_NDK_HOME` or
`$ANDROID_SDK_ROOT/ndk`.

With `--out-dir`, artifacts are copied as:

| File | Target |
|------|--------|
| `<package>` | Linux |
| `<package>.exe` | Windows |
| `lib<package>-arm64-v8a.so` | Android arm64 |
| `lib<package>-armeabi-v7a.so` | Android armv7 |

```bash
s3d build
s3d build linux --release
s3d build windows --release --out-dir dist
s3d build android --release
s3d build all --release --out-dir dist
s3d build linux --features opengl
```

`build` produces libraries/binaries, not APKs. Use `s3d apk` for Android
installables.

## `s3d run`

```text
Usage: s3d run [OPTIONS] [TARGET] [-- <ARGS>...]

Arguments:
  [TARGET]  linux | windows | android   [default: linux]

Options:
  -r, --release       Run a release build
  --                  Additional cargo arguments, passed through

Arguments after `--` go to `cargo run`; Cargo itself does not forward them to
the game unless they come after Cargo's own `--`.
```

Examples:

```bash
s3d run
s3d run --release
s3d run linux -- --release
```

`android` is rejected: use `s3d apk` and install on a device. Running
`windows` from Linux uses the MinGW target and requires Wine to execute.

## `s3d apk`

```text
Usage: s3d apk [OPTIONS] [ABI]

Arguments:
  [ABI]  arm64-v8a | armeabi-v7a | x86_64   [default: arm64-v8a]

Options:
  -r, --release  Build a release APK
      --sign     Accepted, but signing is configured through environment variables
```

Runs `./build_apk.sh <abi> [--release]` from the project root. The script must
exist; `s3d new` provides one.

```bash
s3d apk
s3d apk arm64-v8a
s3d apk armeabi-v7a --release
s3d apk x86_64
```

Output: `apk/app-<abi>-<debug|release>.apk`. Release signing is controlled by
`SOLDATIK3D_KEYSTORE*` variables inside Gradle; `--sign` currently only prints
a notice.

## `s3d export`

```text
Usage: s3d export [OPTIONS]

Options:
  -p, --platforms <PLATFORMS>  Comma-separated: linux,windows,android
                               [default: linux,windows,android]
  -o, --out-dir <OUT_DIR>      Output directory [default: dist]
```

Builds every listed platform in **release mode** and copies the artifacts
directly into the output directory. No archives and no APKs are created.

```bash
s3d export
s3d export --platforms linux,windows --out-dir dist
```

Windows export requires MinGW on Linux; on a Windows host it uses MSVC. Android
export requires the NDK. Unknown platform names are reported and skipped.
Because Linux cross-compilation is not available on Windows, use
`s3d export --platforms windows,android` there.

## `s3d info`

Prints the engine version, supported platforms, renderer features and
environment variables. Takes no options.

## `s3d engine`

```text
Usage: s3d engine <COMMAND>

Commands:
  version  Show engine version and path
  update   Update engine to latest version
  link     Link local engine for development
```

| Subcommand | Current behavior |
|------------|------------------|
| `version` | Prints `s3d 0.1.0` and the compile-time engine path |
| `update` | Prints that updating is not implemented |
| `link <path>` | Prints the path; no files are modified |

## `s3d check`

```text
Usage: s3d check [OPTIONS] [TARGET]

Arguments:
  [TARGET]  linux | windows | android | all
```

Checks that the current directory looks like a generated project:

- `Cargo.toml` exists (hard error otherwise),
- warns if `[lib] crate-type` does not include `cdylib`,
- warns if `build.rs` is missing,
- warns if `.cargo/config.toml` is missing,
- warns if Android is being checked and `ANDROID_NDK_HOME` is unset.

It does not compile anything.

```bash
s3d check
s3d check android
```

## Exit behavior

All commands return a non-zero exit code on failure. Errors are printed to
stderr. `s3d build all` and `s3d export` stop at the first failing platform.

## Related

- [Console commands](cli-commands.md)
- [Configuration](configuration.md)
- [Cross-compilation](../guides/cross-compilation.md)
- [Android](../guides/android.md)
