# Console Commands Reference

Complete list of all built-in console commands available at runtime.

## Opening the Console

- Press `` ` `` (backtick/tilde key) to toggle console
- Or set `SOLDATIK3D_CONSOLE=1` to open at startup
- Or call `ctx.console.set_visible(true)` from code

## Command Syntax

```
command [arguments...]
```

- Commands are case-insensitive
- Arguments separated by spaces
- Quoted strings support spaces: `spawn "my entity" 0 0 0`

## Built-in Commands

### Help & Info

| Command | Description |
|---------|-------------|
| `help` | List all available commands |
| `help <command>` | Show help for specific command |
| `version` | Show engine version |
| `info` | Show system/engine info |

### Engine Control

| Command | Description |
|---------|-------------|
| `quit` / `exit` | Exit the game cleanly |
| `restart` | Restart the game (reload world) |

### Graphics Backend

| Command | Description |
|---------|-------------|
| `backend` | Show current graphics backend |
| `backend vulkan` | Switch to Vulkan backend |
| `backend opengl` | Switch to OpenGL backend |
| `backend auto` | Auto-select best backend |

### VSync Control

| Command | Description |
|---------|-------------|
| `vsync.on` | Enable VSync (wait for vblank) |
| `vsync.off` | Disable VSync (immediate present) |
| `vsync.adaptive` | Adaptive VSync (sync when > refresh rate) |
| `vsync.mailbox` | Mailbox mode (triple buffering, latest frame) |
| `vsync.toggle` | Toggle between On/Off |
| `vsync.current` | Show current VSync mode |

**VSync Modes:**
| Mode | Behavior | Best For |
|------|----------|----------|
| `on` | Wait for vblank | Normal gameplay |
| `off` | Immediate present | Benchmarking, max FPS |
| `adaptive` | Sync when > 60fps | Variable framerate |
| `mailbox` | Triple buffer, latest frame | Low latency, no tearing |

### Debug Visualization

| Command | Description |
|---------|-------------|
| `debug.lit` | Normal lit rendering (default) |
| `debug.normals` | Visualize normals (RGB = XYZ) |
| `debug.depth` | Visualize depth (linearized) |
| `debug.wireframe` | Wireframe overlay |
| `debug.shadow` | Visualize shadow map |

**Debug Views:**
| View | Visualization | Use Case |
|------|---------------|----------|
| `lit` | Normal PBR shading | Normal gameplay |
| `normals` | Red=X, Green=Y, Blue=Z | Normal mapping issues |
| `depth` | White=near, Black=far | Depth precision, z-fighting |
| `wireframe` | Triangle edges | Geometry complexity, LOD |
| `shadow` | Shadow map depth | Shadow acne, peter-panning |

### Screenshot

| Command | Description |
|---------|-------------|
| `screenshot` | Save current frame as PNG |
| `screenshot <filename>` | Save with custom name |

**Output:** `screenshot_<timestamp>.png` in working directory

### Camera Commands

| Command | Description |
|---------|-------------|
| `camera.list` | List all cameras |
| `camera.select <name>` | Switch active camera |
| `camera.select <id>` | Switch by CameraId |
| `camera.next` | Select next camera |
| `camera.prev` / `camera.previous` | Select previous camera |
| `camera.spawn <name> [x y z] [yaw] [pitch]` | Create new camera |
| `camera.remove <name>` | Delete camera |
| `camera.info` | Show active camera details |

**Examples:**
```
camera.list
camera.select main
camera.spawn overhead 0 50 0 -1.57 0
camera.spawn player_cam 0 2 5 0 0
camera.remove old_camera
```

### Entity Commands (If Enabled)

| Command | Description |
|---------|-------------|
| `entity.list` | List all entities |
| `entity.spawn <type> [x y z]` | Spawn entity |
| `entity.remove <id>` | Remove entity |
| `entity.select <id>` | Select entity for editing |
| `entity.move <id> <x> <y> <z>` | Move entity |
| `entity.rotate <id> <x> <y> <z>` | Rotate entity |
| `entity.scale <id> <x> <y> <z>` | Scale entity |

### Light Commands

| Command | Description |
|---------|-------------|
| `light.list` | List all lights |
| `light.add directional [x y z] [r g b] [intensity]` | Add directional light |
| `light.add point [x y z] [r g b] [intensity] [range]` | Add point light |
| `light.add spot [x y z] [dir] [r g b] [intensity] [range] [inner] [outer]` | Add spot light |
| `light.remove <id>` | Remove light |
| `light.shadows on\|off <id>` | Toggle shadows |

### Physics Commands (If Enabled)

| Command | Description |
|---------|-------------|
| `physics.gravity <x> <y> <z>` | Set gravity |
| `physics.debug on\|off` | Toggle physics debug draw |

### Audio Commands (If Enabled)

| Command | Description |
|---------|-------------|
| `audio.volume <0-1>` | Set master volume |
| `audio.mute on\|off` | Toggle mute |
| `audio.play <sound> [volume]` | Play sound effect |

### UI Commands

| Command | Description |
|---------|-------------|
| `ui.show <panel>` | Show UI panel |
| `ui.hide <panel>` | Hide UI panel |
| `ui.scale <factor>` | Set UI scale |

### Development Commands

| Command | Description |
|---------|-------------|
| `reload` | Reload shaders/assets |
| `profile start\|stop\|frame` | Profiling control |
| `mem` | Show memory usage |
| `gc` | Force garbage collection (if applicable) |

---

## Command Aliases

| Alias | Full Command |
|-------|--------------|
| `q` | `quit` |
| `x` | `exit` |
| `b` | `backend` |
| `v` | `vsync` |
| `d` | `debug` |
| `cam` | `camera` |
| `ent` | `entity` |
| `lt` | `light` |
| `ss` | `screenshot` |
| `h` | `help` |

---

## Startup Commands

Run commands automatically at startup:

```bash
# Environment variable
SOLDATIK3D_COMMANDS="vsync.off;debug.wireframe;camera.spawn top 0 10 0" cargo run

# GameConfig
GameConfig::default()
    .with_startup_commands(vec![
        "vsync.off".into(),
        "debug.normals".into(),
        "camera.spawn overview 0 10 0".into(),
    ])
```

---

## Custom Commands

Register your own commands from game code:

```rust
fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    ctx.commands.register(Command::new(
        "mycmd",
        "Description here",
        |ctx| {
            ctx.console.print("Custom command executed!");
            CommandResult::Ok(None)
        },
    ));
    
    // With arguments
    ctx.commands.register(Command::new(
        "teleport",
        "Teleport to coordinates: teleport <x> <y> <z>",
        |ctx| {
            let args = ctx.args;
            if args.len() != 3 {
                return CommandResult::Err("Usage: teleport <x> <y> <z>".into());
            }
            let pos = Vec3::new(
                args[0].parse()?,
                args[1].parse()?,
                args[2].parse()?,
            );
            // Teleport logic
            CommandResult::Ok(Some(format!("Teleported to {:?}", pos)))
        },
    ));
}
```

---

## Tab Completion

- Press `Tab` to autocomplete command names
- Press `Tab` twice to show all completions
- Works for command names and enum arguments (e.g., `backend vulkan|opengl`)

---

## History Navigation

- `Up` / `Down` arrows: Navigate command history
- `Ctrl+R`: Reverse search history (if implemented)
- History persists during session

---

## Console Settings

```rust
// In GameConfig
GameConfig::default()
    .with_open_console(true)  // Open at startup

// Runtime
ctx.console.set_visible(true);
ctx.console.clear();
ctx.console.set_max_history(1000);
```

---

## Command Output Format

```
[12:34:56] [INFO] Message
[12:34:56] [WARN] Warning message
[12:34:56] [ERROR] Error message
[12:34:56] [CMD] > command args
[12:34:56] [OK] Command output
[12:34:56] [ERR] Command error
```

---

## Troubleshooting

### Console Won't Open
- Check `` ` `` key binding (may conflict with OS)
- Try `SOLDATIK3D_CONSOLE=1` environment variable
- Call `ctx.console.set_visible(true)` in first `update()`

### Command Not Found
- Check spelling (case-insensitive)
- Run `help` to see all commands
- Custom commands must be registered in `update()` or `create_world()`

### Command Errors
- Run `help <command>` for usage
- Check argument count and types
- Arguments with spaces need quotes: `spawn "my entity" 0 0 0`

---

*See [CLI Tool](cli.md) for `soldatik3d` command-line tool, or [Runtime API](api/runtime.md) for `CommandRegistry` API.*