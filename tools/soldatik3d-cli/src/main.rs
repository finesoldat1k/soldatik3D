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
#[command(name = "soldatik3d")]
#[command(about = "Soldatik3D game development CLI", long_about = None)]
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
    copy_template(&template_dir, &target_dir, &name)?;
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
    println!("  soldatik3d build --release   # Release build");
    println!("  soldatik3d apk arm64-v8a     # Build Android APK");
    println!("  soldatik3d export            # Export all platforms");

    Ok(())
}

fn find_template_dir() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../templates/game-project"),
        PathBuf::from("/usr/share/soldatik3d/templates/game-project"),
        dirs::data_dir()
            .map(|d| d.join("soldatik3d/templates/game-project"))
            .unwrap_or_default(),
    ];

    for c in candidates {
        if c.exists() {
            return Ok(c);
        }
    }

    anyhow::bail!("Template directory not found. Install soldatik3d or specify --template path.");
}

fn copy_template(src: &Path, dst: &Path, project_name: &str) -> Result<()> {
    let mut options = fs_extra::dir::CopyOptions::new();
    options.overwrite = true;
    options.copy_inside = true;
    options.content_only = true;

    fs_extra::dir::copy(src, dst, &options).context("Failed to copy template")?;

    // Replace placeholders in files
    replace_in_file(dst.join("Cargo.toml"), "my_game", project_name)?;
    replace_in_file(dst.join("README.md"), "my_game", project_name)?;
    replace_in_file(
        dst.join("src/lib.rs"),
        "MyGame",
        &to_pascal_case(project_name),
    )?;

    Ok(())
}

fn replace_in_file(path: PathBuf, from: &str, to: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path)?;
    let new_content = content.replace(from, to);
    fs::write(path, new_content)?;
    Ok(())
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
    println!("{} Building for {:?}...", style("→").green().bold(), target);

    let targets = match target {
        TargetPlatform::All => vec![
            TargetPlatform::Linux,
            TargetPlatform::Windows,
            TargetPlatform::Android,
        ],
        t => vec![t],
    };

    for t in targets {
        build_for_target(t, release, &features, out_dir.as_deref())?;
    }

    println!("{} Build complete!", style("✓").green().bold());
    Ok(())
}

fn build_for_target(
    target: TargetPlatform,
    release: bool,
    features: &str,
    out_dir: Option<&Path>,
) -> Result<()> {
    let profile_args = if release { vec!["--release"] } else { vec![] };

    match target {
        TargetPlatform::Linux => {
            println!("  Building for Linux x86_64...");
            run_cargo_build("", &profile_args, features)?;
        }
        TargetPlatform::Windows => {
            println!("  Building for Windows x86_64 (MinGW)...");
            // Check for MinGW
            if !Command::new("x86_64-w64-mingw32-gcc")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                anyhow::bail!("MinGW-w64 not found. Install with: sudo apt-get install mingw-w64");
            }
            run_cargo_build("x86_64-pc-windows-gnu", &profile_args, features)?;
        }
        TargetPlatform::Android => {
            println!("  Building for Android (arm64-v8a, armeabi-v7a)...");
            check_android_setup()?;
            run_cargo_build("aarch64-linux-android", &profile_args, "opengl")?;
            run_cargo_build("armv7-linux-androideabi", &profile_args, "opengl")?;
        }
        TargetPlatform::All => unreachable!(),
    }

    // Copy artifacts to output directory if specified
    if let Some(out) = out_dir {
        copy_artifacts(target, release, out)?;
    }

    Ok(())
}

fn run_cargo_build(target: &str, profile_args: &[&str], features: &str) -> Result<()> {
    let mut args = vec!["build", "--locked"];
    args.extend(profile_args);
    args.extend(["-p", "my_game"]);
    args.extend(["--features", features]);
    if !target.is_empty() {
        args.extend(["--target", target]);
    }

    let status = Command::new("cargo")
        .args(&args)
        .status()
        .context("Failed to run cargo")?;

    if !status.success() {
        anyhow::bail!(
            "Cargo build failed for target: {}",
            if target.is_empty() { "host" } else { target }
        );
    }

    Ok(())
}

fn check_android_setup() -> Result<()> {
    if std::env::var("ANDROID_NDK_HOME").is_err() {
        anyhow::bail!(
            "ANDROID_NDK_HOME not set. Please set it to your NDK path (e.g., $HOME/android-ndk-r26c)"
        );
    }
    Ok(())
}

fn copy_artifacts(target: TargetPlatform, release: bool, out_dir: &Path) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    fs::create_dir_all(out_dir)?;

    match target {
        TargetPlatform::Linux => {
            let src = PathBuf::from(format!("target/{}/my_game", profile));
            if src.exists() {
                fs::copy(&src, out_dir.join("my_game"))?;
            }
        }
        TargetPlatform::Windows => {
            let src = PathBuf::from(format!(
                "target/x86_64-pc-windows-gnu/{}/my_game.exe",
                profile
            ));
            if src.exists() {
                fs::copy(&src, out_dir.join("my_game.exe"))?;
            }
        }
        TargetPlatform::Android => {
            for (abi, rust_target) in [
                ("arm64-v8a", "aarch64-linux-android"),
                ("armeabi-v7a", "armv7-linux-androideabi"),
            ] {
                let src =
                    PathBuf::from(format!("target/{}/{}/libmy_game.so", rust_target, profile));
                if src.exists() {
                    let dst = out_dir.join(format!("libmy_game-{}.so", abi));
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
            "Cannot run Android target directly. Use `soldatik3d apk` to build APK, then install on device."
        );
    }

    let profile_args = if release { vec!["--release"] } else { vec![] };
    let target_arg = match target {
        TargetPlatform::Linux => "",
        TargetPlatform::Windows => "x86_64-pc-windows-gnu",
        _ => "",
    };

    let mut cargo_args: Vec<String> = vec!["run".into(), "--locked".into()];
    cargo_args.extend(profile_args.iter().map(|s| s.to_string()));
    if !target_arg.is_empty() {
        cargo_args.extend(["--target".into(), target_arg.into()]);
    }
    cargo_args.extend(["--features".into(), "vulkan,opengl".into()]);
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

    check_android_setup()?;

    let abi_str = match abi {
        Abi::Arm64V8a => "arm64-v8a",
        Abi::ArmeabiV7a => "armeabi-v7a",
        Abi::X86_64 => "x86_64",
    };

    // Check if build_apk.sh exists
    let script = PathBuf::from("build_apk.sh");
    if !script.exists() {
        anyhow::bail!(
            "build_apk.sh not found. Run this from your game project root, or use `soldatik3d new` to create a project with the script."
        );
    }

    let mut cmd = Command::new("./build_apk.sh");
    cmd.arg(abi_str);
    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().context("Failed to run build_apk.sh")?;

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
    println!(
        "{} Exporting to {:?}...",
        style("→").green().bold(),
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

        build_for_target(target, true, "vulkan,opengl", Some(&out_dir))?;
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
    println!();
    println!("{}", style("Features:").bold());
    println!("  • Deferred rendering with HDR, SSAO, Bloom");
    println!("  • Physically-based rendering (PBR)");
    println!("  • Dynamic shadows (cascaded shadow maps)");
    println!("  • In-game console with command system");
    println!("  • Hot-reloadable shaders");
    println!("  • Cross-platform input abstraction");
    println!();
    println!("{}", style("Environment Variables:").bold());
    for (k, v) in [
        ("SOLDATIK3D_BACKEND", "vulkan|opengl"),
        ("SOLDATIK3D_VSYNC", "on|off|adaptive|mailbox"),
        ("SOLDATIK3D_MAX_FRAMES", "<number>"),
        ("SOLDATIK3D_COMMANDS", "\"cmd1;cmd2\""),
        ("SOLDATIK3D_CONSOLE", "1"),
        ("SOLDATIK3D_DEBUG_VIEW", "normals|depth|wireframe"),
        ("SOLDATIK3D_VALIDATION", "1"),
    ] {
        println!("  {} = {}", style(k).cyan(), v);
    }

    Ok(())
}

fn cmd_engine(action: EngineAction) -> Result<()> {
    match action {
        EngineAction::Version => {
            println!("soldatik3d 0.1.0");
            println!("Engine path: {}", env!("CARGO_MANIFEST_DIR"));
        }
        EngineAction::Update => {
            println!(
                "Engine update not yet implemented. Use `cargo update -p soldatik3d` in your project."
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
    if target.is_none() || matches!(target, Some(TargetPlatform::Android)) {
        if std::env::var("ANDROID_NDK_HOME").is_err() {
            println!(
                "{} ANDROID_NDK_HOME not set (required for Android builds)",
                style("!").yellow().bold()
            );
        }
    }

    println!("{} Project validation complete", style("✓").green().bold());
    Ok(())
}
