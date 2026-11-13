/// Vertical sync mode for the swapchain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum VsyncMode {
    /// VSync off - present immediately (may cause tearing).
    Off,
    /// VSync on - wait for vertical blank (caps at monitor refresh rate).
    #[default]
    On,
    /// Adaptive VSync - wait for vblank if frame is late, otherwise present immediately.
    /// Not supported on all platforms; falls back to `On`.
    Adaptive,
    /// Mailbox (triple buffering) - always presents latest frame, no tearing.
    /// Vulkan-only; falls back to `On` for OpenGL.
    Mailbox,
}

impl VsyncMode {
    /// Parses a vsync mode name from string (case-insensitive).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "off" | "0" | "false" | "disabled" => Some(Self::Off),
            "on" | "1" | "true" | "enabled" => Some(Self::On),
            "adaptive" | "adapt" => Some(Self::Adaptive),
            "mailbox" | "triple" | "triple_buffer" => Some(Self::Mailbox),
            _ => None,
        }
    }

    /// Reads vsync override from `SOLDATIK3D_VSYNC` environment variable.
    pub fn from_env() -> Option<Self> {
        std::env::var("SOLDATIK3D_VSYNC")
            .ok()
            .and_then(|value| Self::from_name(&value))
    }
}
