//! Soldatik3D CLI - Game development toolchain
//!
//! This tool helps you create, build, and export Soldatik3D games for all
//! supported platforms (Linux, Windows, Android).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "s3d")]
#[command(about = "Soldatik3D development toolchain: project creation, building, exporting", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new game project from template
    New {
        /// Project name (directory name)
        name: String,

        /// Target directory (default: current directory)
        #[arg(short, long)]
        dir: Option<PathBuf>,

        /// Template to use (default: built-in)
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Build the game for a specific platform
    Build {
        /// Target platform
        #[arg(value_enum, default_value = "linux")]
        target: TargetPlatform,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Features to enable
        #[arg(short, long, default_value = "vulkan,opengl")]
        features: String,

        /// Output directory
        #[arg(short, long)]
        out_dir: Option<PathBuf>,
    },

    /// Run the game (desktop only)
    Run {
        /// Target platform
        #[arg(value_enum, default_value = "linux")]
        target: TargetPlatform,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Additional cargo arguments
        #[arg(last = true)]
        args: Vec<String>,
    },

    /// Build Android APK
    Apk {
        /// Target ABI
        #[arg(value_enum, default_value = "arm64-v8a")]
        abi: Abi,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,

        /// Sign the APK (requires keystore configuration)
        #[arg(long)]
        sign: bool,
    },

    /// Export distributable packages for all platforms
    Export {
        /// Platforms to export (comma-separated, default: all)
        #[arg(short, long, default_value = "linux,windows,android")]
        platforms: String,

        /// Output directory
        #[arg(short, long, default_value = "dist")]
        out_dir: PathBuf,
    },

    /// Show engine information and capabilities
    Info,

    /// Manage engine installation
    Engine {
        #[command(subcommand)]
        action: EngineAction,
    },

    /// Validate project configuration
    Check {
        /// Check specific platform
        #[arg(value_enum)]
        target: Option<TargetPlatform>,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum TargetPlatform {
    Linux,
    Windows,
    Android,
    All,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Abi {
    Arm64V8a,
    ArmeabiV7a,
    #[value(name = "x86_64", alias = "x86-64")]
    X86_64,
}

#[derive(Subcommand)]
enum EngineAction {
    /// Show engine version and path
    Version,
    /// Update engine to latest version
    Update,
    /// Link local engine for development
    Link { path: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New {
            name,
            dir,
            template,
        } => cmd_new(name, dir, template),
        Commands::Build {
            target,
            release,
            features,
            out_dir,
        } => cmd_build(target, release, features, out_dir),
        Commands::Run {
            target,
            release,
            args,
        } => cmd_run(target, release, args),
        Commands::Apk { abi, release, sign } => cmd_apk(abi, release, sign),
        Commands::Export { platforms, out_dir } => cmd_export(platforms, out_dir),
        Commands::Info => cmd_info(),
        Commands::Engine { action } => cmd_engine(action),
        Commands::Check { target } => cmd_check(target),
    }
}

fn cmd_new(name: String, dir: Option<PathBuf>, template: Option<String>) -> Result<()> {
    let target_dir = dir.unwrap_or_else(|| PathBuf::from(".")).join(&name);
    let package_name = to_snake_case(&name);

    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", target_dir.display());
    }

    println!(
        "{} Creating new Soldatik3D game: {}",
        style("→").green().bold(),
        style(&name).cyan().bold()
    );

    let pb = ProgressBar::new(5);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Create project structure
    pb.set_message("Creating project structure...");
    fs::create_dir_all(&target_dir)?;
    fs::create_dir_all(target_dir.join("src"))?;
    fs::create_dir_all(target_dir.join("assets"))?;
    fs::create_dir_all(target_dir.join("shaders"))?;
    fs::create_dir_all(target_dir.join(".cargo"))?;
    pb.inc(1);

    // Determine template source
    let template_dir = if let Some(t) = template {
        PathBuf::from(t)
    } else {
        find_template_dir()?
    };

    pb.set_message("Copying template files...");
    copy_template(&template_dir, &target_dir, &package_name)?;
    pb.inc(1);

    // Initialize git repo
    pb.set_message("Initializing git repository...");
    Command::new("git")
        .args(["init"])
        .current_dir(&target_dir)
        .output()
        .context("Failed to init git")?;
    pb.inc(1);

    // Create initial commit
    pb.set_message("Creating initial commit...");
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&target_dir)
        .output()?;
    Command::new("git")
        .args(["commit", "-m", "Initial commit: Soldatik3D game template"])
        .current_dir(&target_dir)
        .output()?;
    pb.inc(1);

    pb.set_message("Done!");
    pb.finish_with_message(format!("Created project at {}", target_dir.display()));

    println!("\n{}", style("Next steps:").bold());
    println!("  cd {}", name);
    println!("  cargo run                    # Run on desktop");
    println!("  s3d build --release          # Release build");
    println!("  s3d apk arm64-v8a            # Build Android APK");
    println!("  s3d export                   # Export all platforms");

    Ok(())
}

fn find_template_dir() -> Result<PathBuf> {
    let mut candidates = Vec::new();

    // Prefer templates shipped next to the executable. This makes an extracted
    // CLI toolkit use its own copy even if the checkout it was built from still
    // exists on the machine (for example during local testing).
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        candidates.push(exe_dir.join("../templates/game"));
        candidates.push(exe_dir.join("../../templates/game"));
    }

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../templates/game"));
    candidates.push(PathBuf::from("/usr/share/s3d/templates/game"));
    candidates.push(
        dirs::data_dir()
            .map(|d| d.join("s3d/templates/game"))
            .unwrap_or_default(),
    );

    for c in candidates {
        if c.join("Cargo.toml").exists() {
            return Ok(c);
        }
    }

    anyhow::bail!("Template directory not found. Install s3d or specify --template path.");
}

fn find_engine_root(template_dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(explicit) = std::env::var("SOLDATIK3D_ENGINE") {
        candidates.push(PathBuf::from(explicit));
    }
    // templates/game -> repository root
    if let Some(root) = template_dir.parent().and_then(Path::parent) {
        candidates.push(root.to_path_buf());
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));

    candidates
        .into_iter()
        .find(|root| root.join("crates/runtime/Cargo.toml").is_file())
        .map(|root| root.canonicalize().unwrap_or(root))
}

fn copy_template(src: &Path, dst: &Path, project_name: &str) -> Result<()> {
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    options.content_only = true;

    fs_extra::dir::copy(src, dst, &options).context("Failed to copy template")?;

    let pascal_name = to_pascal_case(project_name);
    replace_in_tree(dst, "my_game", project_name)?;
    replace_in_tree(dst, "MyGame", &pascal_name)?;

    // Replace the template's relative runtime path with a path that points at
    // the engine checkout that created this project. This makes `s3d new`
    // work from any directory, not only from the engine repository root.
    if let Some(root) = find_engine_root(src) {
        let runtime_dir = root.join("crates/runtime");
        let relative = relative_path(dst, &runtime_dir);
        let relative = relative.to_string_lossy().replace('\\', "/");
        replace_in_tree(dst, "../crates/runtime", &relative)?;
    }

    Ok(())
}

fn replace_in_tree(root: &Path, from: &str, to: &str) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | "target" | "apk" | "build")
            ) {
                continue;
            }
            replace_in_tree(&path, from, to)?;
            continue;
        }

        // Skip binary files: the Gradle wrapper JAR and images must not be
        // decoded as UTF-8.
        if matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("jar" | "png" | "jpg" | "jpeg" | "gif" | "ico" | "so" | "dll" | "exe")
        ) {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if content.contains(from) {
            fs::write(&path, content.replace(from, to))?;
        }
    }
    Ok(())
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.canonicalize().unwrap_or_else(|_| from.to_path_buf());
    let to = to.canonicalize().unwrap_or_else(|_| to.to_path_buf());

    let from_components: Vec<_> = from.components().collect();
    let to_components: Vec<_> = to.components().collect();
    let common = from_components
        .iter()
        .zip(to_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut result = PathBuf::new();
    for _ in common..from_components.len() {
        result.push("..");
    }
    for component in &to_components[common..] {
        result.push(component.as_os_str());
    }
    result
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut previous_was_lower = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && previous_was_lower {
                result.push('_');
            }
            result.extend(c.to_lowercase());
            previous_was_lower = c.is_lowercase() || c.is_numeric();
        } else {
            if !result.ends_with('_') && !result.is_empty() {
                result.push('_');
            }
            previous_was_lower = false;
        }
    }
    let result = result.trim_matches('_').to_string();
    if result.is_empty() {
        "game".to_string()
    } else if result.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        format!("game_{result}")
    } else {
        result
    }
}

fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_'])
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn cmd_build(
    target: TargetPlatform,
    release: bool,
    features: String,
    out_dir: Option<PathBuf>,
) -> Result<()> {
    let project = read_project_name()?;
    println!(
        "{} Building {} for {:?}...",
        style("→").green().bold(),
        project,
        target
    );

    let targets = match target {
        TargetPlatform::All => {
            if cfg!(target_os = "windows") {
                // Linux cross-compilation is not configured by s3d. On a
                // Windows host, "all" means all locally buildable targets.
                vec![TargetPlatform::Windows, TargetPlatform::Android]
            } else {
                vec![
                    TargetPlatform::Linux,
                    TargetPlatform::Windows,
                    TargetPlatform::Android,
                ]
            }
        }
        t => vec![t],
    };

    for t in targets {
        build_for_target(t, release, &features, out_dir.as_deref(), &project)?;
    }

    println!("{} Build complete!", style("✓").green().bold());
    Ok(())
}

fn build_for_target(
    target: TargetPlatform,
    release: bool,
    features: &str,
    out_dir: Option<&Path>,
    project: &str,
) -> Result<()> {
    let profile_args = if release { vec!["--release"] } else { vec![] };

    match target {
        TargetPlatform::Linux => {
            if cfg!(target_os = "windows") {
                anyhow::bail!(
                    "Linux builds are not available on a Windows host. Use `s3d build windows` \
                     or build the Linux target on Linux."
                );
            }
            println!("  Building for Linux x86_64...");
            run_cargo_build(project, "", &profile_args, features)?;
        }
        TargetPlatform::Windows => {
            if cfg!(target_os = "windows") {
                println!("  Building for Windows x86_64 (MSVC)...");
                run_cargo_build(project, "", &profile_args, features)?;
            } else {
                println!("  Building for Windows x86_64 (MinGW, OpenGL only)...");
                if !Command::new("x86_64-w64-mingw32-gcc")
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
                {
                    anyhow::bail!(
                        "MinGW-w64 not found. Install with: sudo apt-get install mingw-w64"
                    );
                }
                // The Vulkan backend is not available for MinGW cross-builds.
                run_cargo_build(project, "x86_64-pc-windows-gnu", &profile_args, "opengl")?;
            }
        }
        TargetPlatform::Android => {
            println!("  Building for Android (arm64-v8a, armeabi-v7a, OpenGL ES)...");
            check_android_setup()?;
            run_cargo_build(project, "aarch64-linux-android", &profile_args, "opengl")?;
            run_cargo_build(project, "armv7-linux-androideabi", &profile_args, "opengl")?;
        }
        TargetPlatform::All => unreachable!(),
    }

    if let Some(out) = out_dir {
        copy_artifacts(target, release, out, project)?;
    }

    Ok(())
}

fn read_project_name() -> Result<String> {
    let cargo_toml = PathBuf::from("Cargo.toml");
    let content = fs::read_to_string(&cargo_toml)
        .with_context(|| format!("Cannot read {}", cargo_toml.display()))?;
    let document: toml::Value = content.parse().context("Invalid Cargo.toml")?;
    document
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .map(str::to_owned)
        .context("Cargo.toml has no [package] name; run s3d from a game project root")
}

fn run_cargo_build(
    project: &str,
    target: &str,
    profile_args: &[&str],
    features: &str,
) -> Result<()> {
    let mut args = vec!["build", "-p", project];
    args.extend(profile_args);
    if !features.is_empty() {
        args.extend(["--no-default-features", "--features", features]);
    }
    if !target.is_empty() {
        args.extend(["--target", target]);
    }

    let mut command = Command::new("cargo");
    command.args(&args);
    if target.starts_with("aarch64-linux-android")
        || target.starts_with("armv7-linux-androideabi")
        || target.starts_with("x86_64-linux-android")
    {
        configure_android_linker(&mut command, target)?;
    }

    let status = command.status().context("Failed to run cargo")?;

    if !status.success() {
        anyhow::bail!(
            "Cargo build failed for target: {}",
            if target.is_empty() { "host" } else { target }
        );
    }

    Ok(())
}

fn find_android_ndk() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("ANDROID_NDK_HOME")
        && !explicit.is_empty()
    {
        let path = PathBuf::from(explicit);
        if path.is_dir() {
            return Ok(path);
        }
    }

    for variable in ["ANDROID_SDK_ROOT", "ANDROID_HOME"] {
        let Ok(sdk) = std::env::var(variable) else {
            continue;
        };
        let ndk_dir = PathBuf::from(sdk).join("ndk");
        let newest = fs::read_dir(&ndk_dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().is_dir())
            .map(|entry| entry.path())
            .max();
        if let Some(path) = newest {
            return Ok(path);
        }
    }

    anyhow::bail!(
        "Android NDK not found. Set ANDROID_NDK_HOME to your NDK path (e.g. $HOME/android-ndk-r26c), \
         or install an NDK under $ANDROID_SDK_ROOT/ndk."
    )
}

fn check_android_setup() -> Result<()> {
    find_android_ndk().map(|_| ())
}

fn configure_android_linker(command: &mut Command, target: &str) -> Result<()> {
    let ndk = find_android_ndk()?;
    let host_tag = match std::env::consts::OS {
        "linux" => "linux-x86_64",
        "macos" => "darwin-x86_64",
        "windows" => "windows-x86_64",
        other => anyhow::bail!("Unsupported host OS for Android builds: {other}"),
    };
    let bin = ndk
        .join("toolchains/llvm/prebuilt")
        .join(host_tag)
        .join("bin");

    let (linker_name, linker_env, cc_env, ar_env) = match target {
        "aarch64-linux-android" => (
            "aarch64-linux-android29-clang",
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            "CC_aarch64_linux_android",
            "AR_aarch64_linux_android",
        ),
        "armv7-linux-androideabi" => (
            "armv7a-linux-androideabi29-clang",
            "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
            "CC_armv7_linux_androideabi",
            "AR_armv7_linux_androideabi",
        ),
        "x86_64-linux-android" => (
            "x86_64-linux-android29-clang",
            "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
            "CC_x86_64_linux_android",
            "AR_x86_64_linux_android",
        ),
        other => anyhow::bail!("Unsupported Android target: {other}"),
    };

    let mut linker = bin.join(linker_name);
    if !linker.is_file() && cfg!(target_os = "windows") {
        // The Windows NDK ships batch wrappers for the compiler drivers.
        let wrapper = bin.join(format!("{linker_name}.cmd"));
        if wrapper.is_file() {
            linker = wrapper;
        }
    }
    if !linker.is_file() {
        anyhow::bail!(
            "Android linker not found: {}. Check that ANDROID_NDK_HOME points to a complete NDK.",
            linker.display()
        );
    }

    let ar = bin.join(if cfg!(target_os = "windows") {
        "llvm-ar.exe"
    } else {
        "llvm-ar"
    });

    command.env(linker_env, &linker);
    command.env(cc_env, &linker);
    command.env(ar_env, ar);
    Ok(())
}

fn copy_artifacts(
    target: TargetPlatform,
    release: bool,
    out_dir: &Path,
    project: &str,
) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let library = project.replace('-', "_");
    fs::create_dir_all(out_dir)?;

    match target {
        TargetPlatform::Linux => {
            let src = PathBuf::from(format!("target/{profile}/{project}"));
            if src.exists() {
                fs::copy(&src, out_dir.join(project))?;
            }
        }
        TargetPlatform::Windows => {
            let src = if cfg!(target_os = "windows") {
                PathBuf::from(format!("target/{profile}/{project}.exe"))
            } else {
                PathBuf::from(format!(
                    "target/x86_64-pc-windows-gnu/{profile}/{project}.exe"
                ))
            };
            if src.exists() {
                fs::copy(&src, out_dir.join(format!("{project}.exe")))?;
            }
        }
        TargetPlatform::Android => {
            for (abi, rust_target) in [
                ("arm64-v8a", "aarch64-linux-android"),
                ("armeabi-v7a", "armv7-linux-androideabi"),
            ] {
                let src = PathBuf::from(format!("target/{rust_target}/{profile}/lib{library}.so"));
                if src.exists() {
                    let dst = out_dir.join(format!("lib{library}-{abi}.so"));
                    fs::copy(&src, &dst)?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn cmd_run(target: TargetPlatform, release: bool, args: Vec<String>) -> Result<()> {
    if matches!(target, TargetPlatform::Android) {
        anyhow::bail!(
            "Cannot run Android target directly. Use `s3d apk` to build APK, then install on device."
        );
    }
    if matches!(target, TargetPlatform::All) {
        anyhow::bail!("`s3d run` needs one target: linux, windows or android.");
    }
    if matches!(target, TargetPlatform::Linux) && cfg!(target_os = "windows") {
        anyhow::bail!("Linux builds are not available on a Windows host. Use `s3d run windows`.");
    }

    let profile_args = if release { vec!["--release"] } else { vec![] };
    let target_arg = match target {
        TargetPlatform::Linux => "",
        TargetPlatform::Windows if cfg!(target_os = "windows") => "",
        TargetPlatform::Windows => "x86_64-pc-windows-gnu",
        _ => "",
    };

    let mut cargo_args: Vec<String> = vec!["run".into()];
    cargo_args.extend(profile_args.iter().map(|s| s.to_string()));
    if !target_arg.is_empty() {
        cargo_args.extend(["--target".into(), target_arg.into()]);
        // MinGW cross-builds use OpenGL only.
        cargo_args.extend([
            "--no-default-features".into(),
            "--features".into(),
            "opengl".into(),
        ]);
    }
    cargo_args.extend(args);

    let status = Command::new("cargo")
        .args(&cargo_args)
        .status()
        .context("Failed to run cargo")?;

    if !status.success() {
        anyhow::bail!("Game exited with error");
    }

    Ok(())
}

fn cmd_apk(abi: Abi, release: bool, sign: bool) -> Result<()> {
    println!(
        "{} Building APK for {:?}...",
        style("→").green().bold(),
        abi
    );

    let abi_str = match abi {
        Abi::Arm64V8a => "arm64-v8a",
        Abi::ArmeabiV7a => "armeabi-v7a",
        Abi::X86_64 => "x86_64",
    };

    // Check if build_apk.sh exists
    let script = PathBuf::from("build_apk.sh");
    if !script.exists() {
        anyhow::bail!(
            "build_apk.sh not found. Run this from your game project root, or use `s3d new` to create a project with the script."
        );
    }

    // On Windows the script must be started through bash (Git Bash / MSYS).
    let mut cmd = if cfg!(target_os = "windows") {
        let mut command = Command::new("bash");
        command.arg("build_apk.sh");
        command
    } else {
        Command::new("./build_apk.sh")
    };
    cmd.arg(abi_str);
    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .context("Failed to run build_apk.sh (on Windows, run it from Git Bash or install bash)")?;

    if !status.success() {
        anyhow::bail!("APK build failed");
    }

    if sign {
        println!(
            "{} APK signing not yet implemented",
            style("!").yellow().bold()
        );
    }

    println!("{} APK built successfully!", style("✓").green().bold());
    println!(
        "  Location: apk/app-{}-{}.apk",
        abi_str,
        if release { "release" } else { "debug" }
    );

    Ok(())
}

fn cmd_export(platforms: String, out_dir: PathBuf) -> Result<()> {
    let project = read_project_name()?;
    println!(
        "{} Exporting {} to {:?}...",
        style("→").green().bold(),
        project,
        out_dir
    );

    let platform_list: Vec<&str> = platforms.split(',').map(|s| s.trim()).collect();

    for platform in platform_list {
        let target = match platform {
            "linux" => TargetPlatform::Linux,
            "windows" => TargetPlatform::Windows,
            "android" => TargetPlatform::Android,
            _ => {
                eprintln!("Unknown platform: {}", platform);
                continue;
            }
        };

        build_for_target(target, true, "vulkan,opengl", Some(&out_dir), &project)?;
    }

    println!(
        "{} Export complete! Packages in: {}",
        style("✓").green().bold(),
        out_dir.display()
    );
    Ok(())
}

fn cmd_info() -> Result<()> {
    println!("{}", style("Soldatik3D Engine").bold().cyan());
    println!("Version: 0.1.0");
    println!("Edition: 2024");
    println!();
    println!("{}", style("Supported Platforms:").bold());
    println!("  • Linux x86_64 (Vulkan + OpenGL)");
    println!("  • Windows x86_64 MSVC (Vulkan + OpenGL)");
    println!("  • Windows x86_64 GNU (OpenGL only)");
    println!("  • Android arm64-v8a (OpenGL ES 3.0)");
    println!("  • Android armeabi-v7a (OpenGL ES 3.0)");
    println!("  • Android x86_64 (OpenGL ES 3.0, emulator)");
    println!();
    println!("{}", style("Features:").bold());
    println!("  • Forward PBR rendering with HDR, SSAO and bloom");
    println!("  • Orthographic shadow map with PCF filtering");
    println!("  • Vulkan 1.2 and OpenGL 3.3 / ES 3.0 backends");
    println!("  • Named cameras with an in-game console and command system");
    println!("  • Immediate-mode UI and bitmap text");
    println!();
    println!("{}", style("Environment Variables:").bold());
    for (k, v) in [
        ("SOLDATIK3D_BACKEND", "vulkan|opengl"),
        ("SOLDATIK3D_VSYNC", "on|off|adaptive|mailbox"),
        ("SOLDATIK3D_MAX_FRAMES", "<number>"),
        ("SOLDATIK3D_COMMANDS", "\"cmd1;cmd2\""),
        ("SOLDATIK3D_CONSOLE", "1|true|yes|on"),
        ("SOLDATIK3D_DEBUG_VIEW", "lit|normals|depth"),
        ("SOLDATIK3D_VALIDATION", "1|true|yes|on"),
    ] {
        println!("  {} = {}", style(k).cyan(), v);
    }

    Ok(())
}

fn cmd_engine(action: EngineAction) -> Result<()> {
    match action {
        EngineAction::Version => {
            println!("s3d 0.1.0");
            println!("Engine path: {}", env!("CARGO_MANIFEST_DIR"));
        }
        EngineAction::Update => {
            println!(
                "Engine update not yet implemented. Use `cargo update -p s3d` in your project."
            );
        }
        EngineAction::Link { path } => {
            println!("Linking engine from: {}", path.display());
        }
    }
    Ok(())
}

fn cmd_check(target: Option<TargetPlatform>) -> Result<()> {
    println!(
        "{} Validating project configuration...",
        style("→").green().bold()
    );

    // Check Cargo.toml
    let cargo_toml = PathBuf::from("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!("Cargo.toml not found. Run from project root.");
    }

    let content = fs::read_to_string(&cargo_toml)?;
    if !content.contains("crate-type") || !content.contains("cdylib") {
        println!(
            "{} Cargo.toml missing `crate-type = [\"cdylib\", \"rlib\"]` for Android support",
            style("!").yellow().bold()
        );
    }

    // Check build.rs
    if !PathBuf::from("build.rs").exists() {
        println!(
            "{} build.rs not found (recommended for asset handling)",
            style("!").yellow().bold()
        );
    }

    // Check .cargo/config.toml
    if !PathBuf::from(".cargo/config.toml").exists() {
        println!(
            "{} .cargo/config.toml not found (recommended for cross-compilation)",
            style("!").yellow().bold()
        );
    }

    // Check platform-specific setup
    if (target.is_none() || matches!(target, Some(TargetPlatform::Android)))
        && std::env::var("ANDROID_NDK_HOME").is_err()
    {
        println!(
            "{} ANDROID_NDK_HOME not set (required for Android builds)",
            style("!").yellow().bold()
        );
    }

    println!("{} Project validation complete", style("✓").green().bold());
    Ok(())
}
