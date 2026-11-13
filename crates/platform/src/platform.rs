//! Platform detection and configuration for the Soldatik3D engine.
//!
//! This module provides compile-time and runtime platform detection,
//! allowing the engine to adapt its behavior across different targets.

use std::env;

/// Target operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOS {
    Linux,
    Windows,
    MacOS,
    Android,
    iOS,
    WebAssembly,
    Unknown,
}

impl TargetOS {
    /// Returns the target OS at compile time.
    pub const fn current() -> Self {
        #[cfg(target_os = "linux")]
        return TargetOS::Linux;
        #[cfg(target_os = "windows")]
        return TargetOS::Windows;
        #[cfg(target_os = "macos")]
        return TargetOS::MacOS;
        #[cfg(target_os = "android")]
        return TargetOS::Android;
        #[cfg(target_os = "ios")]
        return TargetOS::iOS;
        #[cfg(target_arch = "wasm32")]
        return TargetOS::WebAssembly;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "android",
            target_os = "ios",
            target_arch = "wasm32",
        )))]
        return TargetOS::Unknown;
    }

    /// Returns the target OS from the environment (useful for cross-compilation).
    pub fn from_env() -> Self {
        if let Ok(target) = env::var("CARGO_CFG_TARGET_OS") {
            match target.as_str() {
                "linux" => TargetOS::Linux,
                "windows" => TargetOS::Windows,
                "macos" => TargetOS::MacOS,
                "android" => TargetOS::Android,
                "ios" => TargetOS::iOS,
                _ => TargetOS::Unknown,
            }
        } else {
            Self::current()
        }
    }

    /// Returns a human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            TargetOS::Linux => "Linux",
            TargetOS::Windows => "Windows",
            TargetOS::MacOS => "macOS",
            TargetOS::Android => "Android",
            TargetOS::iOS => "iOS",
            TargetOS::WebAssembly => "WebAssembly",
            TargetOS::Unknown => "Unknown",
        }
    }

    /// Whether this is a mobile platform.
    pub fn is_mobile(&self) -> bool {
        matches!(self, TargetOS::Android | TargetOS::iOS)
    }

    /// Whether this is a desktop platform.
    pub fn is_desktop(&self) -> bool {
        matches!(self, TargetOS::Linux | TargetOS::Windows | TargetOS::MacOS)
    }

    /// Whether this supports Vulkan natively.
    pub fn supports_vulkan(&self) -> bool {
        matches!(self, TargetOS::Linux | TargetOS::Windows | TargetOS::Android)
    }

    /// Whether this supports Metal.
    pub fn supports_metal(&self) -> bool {
        matches!(self, TargetOS::MacOS | TargetOS::iOS)
    }

    /// Whether this supports OpenGL.
    pub fn supports_opengl(&self) -> bool {
        matches!(self, TargetOS::Linux | TargetOS::Windows | TargetOS::MacOS | TargetOS::Android)
    }

    /// Default graphics backend for this platform.
    pub fn default_backend(&self) -> render::BackendKind {
        match self {
            TargetOS::Linux | TargetOS::Windows | TargetOS::Android => {
                render::BackendKind::Vulkan
            }
            TargetOS::MacOS | TargetOS::iOS => {
                // Metal would be preferred, but we only have Vulkan/OpenGL
                render::BackendKind::OpenGl
            }
            TargetOS::WebAssembly => render::BackendKind::OpenGl,
            TargetOS::Unknown => render::BackendKind::Vulkan,
        }
    }
}

/// Target architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    AArch64,
    ARMv7,
    RISCV64,
    X86,
    WASM32,
    Unknown,
}

impl TargetArch {
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        return TargetArch::X86_64;
        #[cfg(target_arch = "aarch64")]
        return TargetArch::AArch64;
        #[cfg(target_arch = "arm")]
        return TargetArch::ARMv7;
        #[cfg(target_arch = "riscv64")]
        return TargetArch::RISCV64;
        #[cfg(target_arch = "x86")]
        return TargetArch::X86;
        #[cfg(target_arch = "wasm32")]
        return TargetArch::WASM32;
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "arm",
            target_arch = "riscv64",
            target_arch = "x86",
            target_arch = "wasm32",
        )))]
        return TargetArch::Unknown;
    }

    pub fn name(&self) -> &'static str {
        match self {
            TargetArch::X86_64 => "x86_64",
            TargetArch::AArch64 => "aarch64",
            TargetArch::ARMv7 => "armv7",
            TargetArch::RISCV64 => "riscv64",
            TargetArch::X86 => "x86",
            TargetArch::WASM32 => "wasm32",
            TargetArch::Unknown => "unknown",
        }
    }

    pub fn is_64bit(&self) -> bool {
        matches!(self, TargetArch::X86_64 | TargetArch::AArch64 | TargetArch::RISCV64)
    }

    pub fn is_wasm(&self) -> bool {
        matches!(self, TargetArch::WASM32)
    }
}

/// Platform capabilities detected at runtime.
#[derive(Debug, Clone, Default)]
pub struct PlatformCapabilities {
    /// Vulkan instance version supported.
    pub vulkan_version: Option<u32>,
    /// Available Vulkan extensions.
    pub vulkan_extensions: Vec<String>,
    /// OpenGL version string.
    pub opengl_version: Option<String>,
    /// OpenGL extensions.
    pub opengl_extensions: Vec<String>,
    /// Metal support (macOS/iOS).
    pub metal_supported: bool,
    /// Whether running in a container/VM.
    pub virtualized: bool,
    /// Available CPU cores.
    pub cpu_cores: usize,
    /// Total system memory in MB.
    pub total_memory_mb: usize,
    /// GPU vendor string.
    pub gpu_vendor: Option<String>,
    /// GPU renderer string.
    pub gpu_renderer: Option<String>,
    /// High DPI display.
    pub high_dpi: bool,
    /// Touch screen support.
    pub touch_support: bool,
    /// Gamepad support.
    pub gamepad_support: bool,
}

impl PlatformCapabilities {
    /// Detects platform capabilities at runtime.
    pub fn detect() -> Self {
        let mut caps = Self {
            cpu_cores: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            ..Default::default()
        };

        // Read /proc/cpuinfo on Linux for virtualization
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
                caps.virtualized = content.contains("hypervisor")
                    || content.contains("QEMU")
                    || content.contains("VMware")
                    || content.contains("VirtualBox");
            }
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<usize>() {
                                caps.total_memory_mb = kb / 1024;
                            }
                        }
                        break;
                    }
                }
            }
        }

        caps
    }

    /// Checks if the platform meets minimum requirements.
    pub fn meets_minimum_requirements(&self) -> bool {
        self.cpu_cores >= 2 && self.total_memory_mb >= 2048
    }
}

/// Platform-specific window hints.
#[derive(Debug, Clone)]
pub struct WindowHints {
    /// Preferred graphics API.
    pub preferred_backend: Option<render::BackendKind>,
    /// Enable VSync by default.
    pub vsync: bool,
    /// Start in fullscreen mode.
    pub fullscreen: bool,
    /// Window transparency.
    pub transparent: bool,
    /// High DPI support.
    pub high_dpi: bool,
    /// Resizable window.
    pub resizable: bool,
    /// Window title.
    pub title: String,
    /// Initial window size.
    pub size: Option<(u32, u32)>,
    /// Minimum window size.
    pub min_size: Option<(u32, u32)>,
    /// Maximum window size.
    pub max_size: Option<(u32, u32)>,
}

impl Default for WindowHints {
    fn default() -> Self {
        Self {
            preferred_backend: None,
            vsync: true,
            fullscreen: false,
            transparent: false,
            high_dpi: true,
            resizable: true,
            title: "Soldatik3D".to_string(),
            size: Some((1280, 720)),
            min_size: Some((640, 480)),
            max_size: None,
        }
    }
}

impl WindowHints {
    /// Creates platform-appropriate default hints.
    pub fn for_platform() -> Self {
        let os = TargetOS::current();
        let mut hints = Self::default();
        
        match os {
            TargetOS::Android => {
                hints.fullscreen = true;
                hints.resizable = false;
                hints.vsync = true;
                hints.size = None; // Use full screen
            }
            TargetOS::WebAssembly => {
                hints.size = Some((1280, 720));
                hints.vsync = true;
            }
            TargetOS::MacOS => {
                hints.high_dpi = true;
                hints.transparent = true; // For titlebar integration
            }
            _ => {}
        }
        
        hints.preferred_backend = Some(os.default_backend());
        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_os_current() {
        let os = TargetOS::current();
        assert_ne!(os, TargetOS::Unknown);
    }

    #[test]
    fn test_target_arch_current() {
        let arch = TargetArch::current();
        assert_ne!(arch, TargetArch::Unknown);
    }

    #[test]
    fn test_platform_capabilities_detect() {
        let caps = PlatformCapabilities::detect();
        assert!(caps.cpu_cores > 0);
    }
}