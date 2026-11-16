# UI API

The `ui` crate provides a hybrid immediate-mode UI. Widgets are called every
frame and return a [`Response`], while a retained arena tree stores stable ids,
hover/press state, scroll offsets, dropdown state and the text cache.

```rust
use runtime::ui::{Layout, Response, Ui};
```

`Ui` is available as `UpdateContext::ui`. The runtime begins the frame before
`Game::update`, lays out and paints the retained tree when the game is done,
draws the console on top, and uploads the resulting vertices plus the glyph
atlas to the renderer.

## Coordinates

- Origin is the **top-left** corner of the framebuffer.
- Units are physical pixels.
- `x` grows right, `y` grows down.
- Every widget is clipped by its parent and by scroll panels.

## Frame lifecycle

```rust
// The runtime does this for every game:
ui.begin_frame(&input, viewport); // reset tree state, build input snapshot
game.update(&mut ctx);            // call widgets, read responses
ui.end_frame();                   // layout + paint into the draw list
renderer.render(&FrameData { ui_vertices: ui.draw().vertices(), .. });
```

`begin_frame` and `end_frame` are engine-owned; games only build widgets inside
`Game::update`. `begin_frame_snapshot(UiInput, Vec2)` is available for tests and
replay tools that do not have a platform input snapshot.

The default font is embedded Open Sans Regular/Bold (Latin + Cyrillic). Font
sizes are pixels, **not** the old 1.0/1.5/2.0 bitmap scales.

## Layout

Layouts are a compact flexbox subset: row/column direction, auto/fixed/percent
sizing, `Fill` children, gaps, padding, margins, alignment and absolute
placement.

```rust
ui.panel(
    Layout::column().padding(16.0).gap(8.0).width(320.0),
    |ui| {
        ui.heading("Settings");
        ui.label("Vertical sync");
        ui.separator();
    },
);
```

| Type | Meaning |
|------|---------|
| `Size::Auto` | Fit the measured content |
| `Size::Fixed(px)` | Exact pixels |
| `Size::Percent(p)` | Percentage of the parent content box |
| `Size::Fill` | Share leftover space with other fill siblings |

`Layout` builder highlights:

| Method | Effect |
|--------|--------|
| `Layout::row()` / `Layout::column()` | Flow direction |
| `.padding(px)` / `.padding_edges(edges)` | Inner spacing |
| `.margin(px)` | Outer spacing |
| `.gap(px)` | Space between children |
| `.width(px)` / `.height(px)` / `.size(w, h)` | Fixed size |
| `.fill_width()` / `.fill_height()` / `.fill()` | Fill available space |
| `.grow(weight)` | Share of leftover space for fill children |
| `.align_items(AlignItems)` | Start / Center / End / Stretch |
| `.justify_content(JustifyContent)` | Start / Center / End / SpaceBetween |
| `.absolute(Rect)` | Place relative to the parent content box |
| `.scrollable()` | Vertical scrolling + clipping |
| `.id(value)` | Explicit stable id instead of the auto id |

Automatic ids are stable across frames and derived from the parent id, the
child index and the widget kind. Explicit `.id("name")` is only needed when a
widget moves between parents or must be addressed directly.

A root widget with an explicit width and height is centered in the viewport.
`.absolute(Rect)` places a widget relative to its parent, so wrap absolutely
positioned panels in a full-screen container:

```rust
ui.begin_container(Layout::default().fill());
ui.begin_styled_container(
    Layout::column().absolute(Rect::new(20.0, 60.0, 320.0, 240.0)),
    theme.window,
    theme.panel_border,
    theme.radius,
);
// ... panel content ...
ui.end_container();
ui.end_container();
```

## Widgets

All widgets are methods on `Ui`. Every method has a `*_styled` or `*_with`
variant for custom styles and layout overrides.

### Text

```rust
ui.label("Simple label");
ui.heading("Section title");
ui.hint("Muted hint");
ui.paragraph("Long text that wraps to the current viewport width.");

let style = runtime::ui::LabelStyle::new(18.0)
    .color([1.0, 0.9, 0.3, 1.0])
    .align(runtime::ui::TextAlign::Center)
    .wrap(240.0);
ui.label_styled("Styled", style);
```

`measure_label(text, &style)` returns the pixel size without drawing.

### Buttons

```rust
if ui.button("Play").clicked {
    start_game();
}
if ui.button_primary("Continue").clicked { /* accent action */ }
if ui.button_danger("Delete").clicked { /* destructive action */ }
```

`ButtonStyle::from_theme(&theme)` can be customized with `.height(px)`,
`.font_size(px)` and `.padding(px)`.

### Toggle, slider, progress

```rust
ui.toggle("VSync", &mut self.vsync);
ui.slider("Volume", &mut self.volume, 0.0, 1.0);
ui.progress(self.loaded / self.total, 1.0);
ui.progress_labeled("Loading 42%", 0.42, 1.0);
```

Sliders use `Response::changed` and are dragged with the pointer that started
the interaction, including on touch screens.

### Dropdown

```rust
let items = ["LIT", "NORMALS", "DEPTH"];
let response = ui.dropdown("", &items, &mut self.selected);
if response.changed {
    self.view = items[self.selected];
}
```

The popup is rendered after the regular tree, above all widgets, and blocks
input to widgets underneath it.

### Panels, scroll panels and spacing

```rust
ui.panel(Layout::column().padding(12.0), |ui| {
    ui.label("Card content");
});

let panel = Layout::column().height(220.0).scrollable().padding(8.0);
let response = ui.scroll_panel(panel, |ui| {
    for row in &self.rows {
        ui.label(row);
    }
});

ui.spacer(0.0, 12.0);
ui.separator();
ui.flexible_space();
```

Scroll panels handle the mouse wheel, touch scrolling and a draggable
scrollbar. `scroll_info(id)` exposes `(offset, max_offset)` for custom UI.

## Touch controls

### Virtual joystick

```rust
use runtime::ui::{JoystickState, JoystickStyle};

// Field in the game struct; state survives between frames.
joystick: JoystickState,

let style = JoystickStyle::from_theme(ui.theme());
let axes = ui.virtual_joystick(&mut self.joystick, &style);
// axes.x: right, axes.y: down in screen space (deadzoned and normalized).
// JoystickState::axes() returns (strafe, forward) with forward positive.
```

The joystick area defaults to the **left half** of the screen. Its base follows
the finger that started the gesture instead of staying at a fixed position,
which is much more comfortable on phones. `JoystickStyle::area` restricts the
gesture area and `handle_radius` / `radius` change the visuals.

The joystick and touch camera are background gestures: call them **after** the
HUD widgets, so a button placed in the same screen half gets the first chance
to claim the pointer.

### Touch camera

```rust
use runtime::ui::TouchCameraState;

camera: TouchCameraState,

let style = runtime::ui::TouchCameraStyle::from_theme(ui.theme());
let delta = ui.touch_camera(&mut self.camera, &style);
self.player.add_yaw(delta.x);
self.player.add_pitch(-delta.y);
```

The camera area defaults to the **right half** of the screen and only consumes
touch pointers. Mouse look on desktop stays on the platform's captured cursor.

## Pointer capture

The UI marks the widget that owns the current pointer. `ui.captures_pointer()`
returns `true` while a widget or a scrollbar is being dragged; games use it to
skip camera look or world clicks. Dropdown popups, joystick and touch camera
claim their pointers too, so a finger never drives two controls at once.

## Styling

`Theme` owns all colors and control metrics:

```rust
let theme = runtime::ui::Theme::dark().scaled(display_scale);
ui.set_theme(theme);

let accent = ui.theme().accent;
let control_height = ui.theme().control_height;
```

`Theme::dark()`, `Theme::light()`, `.with_font_size(px)` and `.scaled(scale)`
cover the common cases. `ButtonStyle`, `DropdownStyle`, `SliderStyle`,
`ToggleStyle`, `ProgressStyle`, `ScrollPanelStyle`, `LabelStyle`,
`JoystickStyle` and `TouchCameraStyle` override individual widgets.

## Low-level drawing

For HUD overlays and debug tools, `Ui` forwards the draw list:

```rust
ui.rect(x, y, w, h, color);
ui.rect_rounded(x, y, w, h, radius, color);
ui.rect_outline(rect, 1.0, color);
ui.line_h(x, y, width, thickness, color);
ui.circle(center, radius, color);
ui.circle_outline(center, radius, thickness, color);
ui.text("TEXT", x, y, 18.0, color);
ui.crosshair();
ui.set_clip(rect);
ui.reset_clip();
```

Text uses `runtime::ui::TextStyle`:

```rust
let style = runtime::ui::TextStyle::new(18.0)
    .with_color([1.0; 4])
    .align(runtime::ui::TextAlign::Center)
    .wrap(300.0);
ui.text_styled("Centered", &style, runtime::glam::Vec2::new(20.0, 20.0));
```

Raw calls made during `Game::update` are emitted before the retained tree.
The console is drawn after `Ui::end_frame` by the runtime, so it stays on top
of game HUDs.

## Text engine

The `text` crate is a dependency-free TTF engine:

- SFNT/TTF parsing, `cmap`, `glyf`, `kern`,
- anti-aliased rasterization at the requested pixel size,
- dynamic R8 atlas with shelf packing and dirty-rectangle tracking,
- layout with alignment and wrapping,
- embedded Open Sans Regular/Bold; external fonts can be loaded at runtime.

A new font atlas is uploaded only when new glyphs were rasterized. The atlas
starts at 512×512 and grows up to 2048×4096.

## Console

The developer console is drawn by the runtime into the same `Ui` after the
game. While it is open, `ctx.console_open` is `true` and the cursor is released.
It supports:

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
