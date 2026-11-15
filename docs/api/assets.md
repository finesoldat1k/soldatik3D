# Assets API

The `assets` crate owns CPU-side data that does not depend on a graphics API:
texture pixels, the built-in font atlas and a procedural environment cubemap.
`runtime` re-exports the public helpers.

```rust
use runtime::{TextureData, font_atlas_rgba8, generate_environment_cubemap};
```

The font layout constants live in the `assets` crate when you depend on it
directly:

```rust
use assets::{
    FONT_ATLAS_SIZE, FONT_CELL_SIZE, FONT_GLYPH_COLUMNS, FONT_GLYPH_HEIGHT,
    FONT_GLYPH_WIDTH,
};
```

## `TextureData`

RGBA8 pixel data used as the scene material's albedo texture.

```rust
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl TextureData {
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Self;
    pub fn white() -> Self;
    pub fn from_rgb8(width: u32, height: u32, rgb: &[u8]) -> Self;
}
```

| Constructor | Notes |
|-------------|-------|
| `new` | Panics unless `pixels.len() == width * height * 4` |
| `white` | 1×1 opaque white, a neutral default |
| `from_rgb8` | Expands each RGB triple to RGBA with alpha 255 |

The engine uploads the material texture once when the renderer is created.
There is no texture array, no per-mesh material and no runtime texture update.

### Procedural texture

```rust
fn checkerboard(size: u32) -> TextureData {
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    let cell = (size / 8).max(1);
    for y in 0..size {
        for x in 0..size {
            let light = ((x / cell) + (y / cell)) % 2 == 0;
            let value = if light { 220 } else { 40 };
            pixels.extend_from_slice(&[value, value, value, 255]);
        }
    }
    TextureData::new(size, size, pixels)
}
```

### Loading a PNG

Image decoding is not part of the engine. Add the `image` crate to the game and
convert its buffer:

```toml
[dependencies]
image = { version = "0.25", default-features = false, features = ["png"] }
```

```rust
use runtime::TextureData;

fn load_png(path: &str) -> anyhow::Result<TextureData> {
    let image = image::open(path)?.to_rgba8();
    let (width, height) = image.dimensions();
    Ok(TextureData::new(width, height, image.into_raw()))
}
```

Use the exact dimensions and an RGBA8 buffer. Textures are sampled with
repeating UVs by the engine shaders.

## Font atlas

The engine ships a 128×128 bitmap font atlas covering printable ASCII
(`0x20..=0x7E`). Each glyph occupies an 8×8 cell in a 16-column grid; only the
top-left 5×7 pixels contain ink.

```rust
pub const FONT_ATLAS_SIZE: u32 = 128;
pub const FONT_CELL_SIZE: u32 = 8;
pub const FONT_GLYPH_WIDTH: u32 = 5;
pub const FONT_GLYPH_HEIGHT: u32 = 7;
pub const FONT_GLYPH_COLUMNS: u32 = 16;

pub fn font_atlas_rgba8() -> Vec<u8>;
```

`font_atlas_rgba8` returns `128 * 128 * 4` bytes. The `ui` crate uses it through
`UiDrawList::text`, so games rarely call it directly. Characters outside
printable ASCII are skipped by the text renderer.

## Environment cubemap

```rust
pub fn generate_environment_cubemap(size: u32) -> Vec<f32>;
```

Generates six `size × size` RGBA32F faces in `+X, -X, +Y, -Y, +Z, -Z` order.
The sky has a horizon gradient, a sun disk and a sun glow, and is used as the
default image-based-lighting environment by both backends.

The buffer length is `size * size * 6 * 4` floats. A small size (32 or 64) is
enough for diffuse IBL; larger sizes sharpen specular reflections.

## Where assets live

| Build | Location |
|-------|----------|
| Desktop | Working directory; the game reads files at runtime |
| Android | `assets/` is copied into the APK by `build.rs` and Gradle |
| Generated project | `assets/` at the project root |

The engine itself has no asset manager, virtual filesystem or hot reload. If a
game needs both desktop and Android loading, put shared files under `assets/`
and use platform-specific code for the path:

```rust
#[cfg(target_os = "android")]
const ASSET_ROOT: &str = "";       // Android asset manager, not a filesystem path
#[cfg(not(target_os = "android"))]
const ASSET_ROOT: &str = "assets/";
```

The generated template currently copies assets into the APK but does not read
them back through `AAssetManager`; add the `android-activity` asset APIs if your
game needs runtime asset loading on Android.

## Related

- [Scene API](scene.md)
- [UI API](ui.md)
- [Project structure](../getting-started/project-structure.md)
