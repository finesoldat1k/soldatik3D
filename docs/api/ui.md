# UI API

The `ui` crate is a small immediate-mode draw list. It does not own layout,
widgets or input handling: a game appends rectangles and text every frame and
the runtime uploads the vertices after `Game::update`.

```rust
use runtime::{UiDrawList, UiVertex};
```

`UiDrawList` is also available through `UpdateContext::ui`.

## Coordinates

- Origin is the **top-left** corner of the framebuffer.
- Units are physical pixels, not logical points. Multiply by the display scale
  if you design in logical pixels.
- `x` grows right, `y` grows down.
- There is no clipping, scissor or transform stack. Anything outside the
  viewport is clipped by the GPU.

```rust
let width = ctx.viewport.0 as f32;
let height = ctx.viewport.1 as f32;
ctx.ui.rect(0.0, 0.0, width, 48.0, [0.0, 0.0, 0.0, 0.7]);
```

## `UiDrawList`

```rust
impl UiDrawList {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self;
    pub fn clear(&mut self);
    pub fn resize(&mut self, viewport_width: u32, viewport_height: u32);
    pub fn width(&self) -> f32;
    pub fn height(&self) -> f32;
    pub fn vertices(&self) -> &[UiVertex];
    pub fn is_empty(&self) -> bool;
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]);
    pub fn text(&mut self, text: &str, x: f32, y: f32, scale: f32, color: [f32; 4]);
    pub fn text_width(text: &str, scale: f32) -> f32;
    pub fn crosshair(&mut self);
}
```

| Method | Description |
|--------|-------------|
| `rect` | Solid rectangle; six vertices |
| `text` | Bitmap text at the top-left of the first glyph |
| `text_width` | Width in pixels at a given scale |
| `crosshair` | 4×4 white square at the viewport center |
| `clear` | Drop all vertices (called by the engine every frame) |
| `resize` | Update the viewport size used for NDC conversion |

Every rectangle and glyph adds six vertices. There is no batching limit in the
engine, but very large UI text still costs one quad per character.

## Text

`text` renders printable ASCII (`0x20..=0x7E`) with the built-in 5×7 font.
`scale` multiplies both glyph size and advance:

| Scale | Approximate character cell |
|-------|----------------------------|
| `1.0` | 5×7 pixels + 1 pixel advance |
| `1.5` | 7.5×10.5 pixels |
| `2.0` | 10×14 pixels |

Characters outside the range are skipped. To center text:

```rust
let label = "PAUSED";
let scale = 2.0;
let width = UiDrawList::text_width(label, scale);
ctx.ui.text(
    label,
    (ctx.viewport.0 as f32 - width) * 0.5,
    80.0,
    scale,
    [1.0, 0.9, 0.4, 1.0],
);
```

`text_width` counts only printable characters, matching `text`.

## Colors

Colors are linear RGBA in `0.0..=1.0`; alpha is blended. The UI pass runs after
tone mapping and gamma correction, so values are close to sRGB. Use alpha for
translucent panels:

```rust
ctx.ui.rect(16.0, 16.0, 280.0, 96.0, [0.05, 0.06, 0.08, 0.85]);
ctx.ui.rect(16.0, 16.0, 4.0, 96.0, [0.2, 0.8, 0.4, 1.0]);
ctx.ui.text("SOLDATIK3D", 32.0, 28.0, 1.4, [1.0; 4]);
```

There are no rounded corners, gradients or borders. Compose them from
rectangles.

## A simple HUD

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let (w, h) = ctx.viewport;
    let w = w as f32;
    let h = h as f32;

    ctx.ui.rect(0.0, h - 36.0, w, 36.0, [0.0, 0.0, 0.0, 0.6]);
    ctx.ui.text(
        &format!("HP {:>3}   AMMO {:>2}", self.hp, self.ammo),
        16.0,
        h - 26.0,
        1.2,
        [1.0; 4],
    );
    ctx.ui.crosshair();

    if self.paused {
        ctx.ui.rect(0.0, 0.0, w, h, [0.0, 0.0, 0.0, 0.5]);
        let label = "PAUSED";
        let width = UiDrawList::text_width(label, 2.0);
        ctx.ui.text(label, (w - width) * 0.5, h * 0.5 - 7.0, 2.0, [1.0; 4]);
    }
}
```

## Buttons and hit testing

The engine has no widget layer. A button is a rectangle plus a cursor test:

```rust
fn button(ui: &mut UiDrawList, input: &Input, label: &str, x: f32, y: f32) -> bool {
    let width = 120.0;
    let height = 32.0;
    let cursor = input.cursor_position();
    let hovered = cursor.x >= x
        && cursor.x <= x + width
        && cursor.y >= y
        && cursor.y <= y + height;
    let color = if hovered {
        [0.2, 0.45, 0.8, 1.0]
    } else {
        [0.15, 0.2, 0.3, 1.0]
    };
    ui.rect(x, y, width, height, color);
    ui.text(label, x + 10.0, y + 10.0, 1.0, [1.0; 4]);
    hovered && input.primary_clicked()
}
```

Works only while the cursor is released; captured cursor motion produces zero
`cursor_position` changes on some platforms.

## Console

The developer console is drawn by the engine into the same `UiDrawList`, after
the game. While it is open, `ctx.console_open` is true and the cursor is
released. The console supports:

- command history with `↑` / `↓`,
- tab completion of command names,
- `help`, `clear`, `quit`/`exit`, backend/screenshot/VSync commands,
- `camera.*` commands,
- game commands from `Game::command`.

See [Console commands](../reference/cli-commands.md) for the full list.

## Related

- [Runtime API](runtime.md)
- [Input API](input.md)
- [Assets API](assets.md)
