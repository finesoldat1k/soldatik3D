use glam::{Mat4, Vec3};
use ui::UiVertex;

/// Camera matrices for one frame. The projection is already configured for the
/// current viewport aspect ratio.
#[derive(Clone, Copy, Debug)]
pub struct CameraFrame {
    pub view: Mat4,
    pub projection: Mat4,
    pub position: Vec3,
}

impl CameraFrame {
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }
}

/// Debug visualization mode of the forward pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugView {
    #[default]
    Lit = 0,
    Normals = 1,
    Depth = 2,
}

impl DebugView {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// Statistics of the last submitted frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub vertices: u64,
    pub triangles: u64,
    pub draw_calls: u32,
}

/// Rectangle inside the glyph atlas that changed since the last upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiAtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Glyph atlas update embedded in [`FrameData`].
///
/// `pixels` is a single-channel (R8) coverage texture owned by the UI. It is
/// only present when new glyphs were rasterized, so backing textures can skip
/// the upload entirely on most frames.
#[derive(Clone, Copy, Debug)]
pub struct UiAtlasFrame<'a> {
    pub width: u32,
    pub height: u32,
    /// Full R8 coverage texture.
    pub pixels: &'a [u8],
    /// The whole texture must be reuploaded (first use or after a resize).
    pub full: bool,
    /// Changed sub-rectangles when `full` is false.
    pub dirty: &'a [UiAtlasRect],
}

/// Per-frame data handed to the renderer backend.
pub struct FrameData<'a> {
    pub camera: CameraFrame,
    pub ui_vertices: &'a [UiVertex],
    pub ui_atlas: Option<UiAtlasFrame<'a>>,
    pub debug_view: DebugView,
}
