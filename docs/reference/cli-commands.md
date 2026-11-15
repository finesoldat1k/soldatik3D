# Console Commands

The developer console is part of the runtime. Toggle it with `F1` or `` ` ``.
While it is open the cursor is released, text input goes to the console, and
`UpdateContext::console_open` is `true`.

Features:

- `Enter` executes the current line,
- `↑` / `↓` browse history,
- `Tab` completes command names and cycles matches,
- `Esc` closes the console,
- output keeps the last 200 lines and displays the last 12.

Type `help` for the built-in list. Game commands registered through
`Game::command_help` are shown too.

## Built-in commands

### Session

| Command | Arguments | Description |
|---------|-----------|-------------|
| `help` | — | List commands |
| `clear` | — | Clear console history |
| `quit` | — | Request shutdown |
| `exit` | — | Alias for `quit` |
| `backend` | — | Print the active backend **and the env variable hint**; it does not switch backends |
| `screenshot` | — | Save `screenshot_<seconds>.png` in the working directory |

### VSync

| Command | Description |
|---------|-------------|
| `vsync.on` | Wait for vertical blank |
| `vsync.off` | Present immediately; may tear |
| `vsync.adaptive` | Wait when on time, present immediately when late |
| `vsync.mailbox` | Triple buffering; Vulkan only |
| `vsync.toggle` | Toggle between `On` and `Off` |
| `vsync.current` | Print the current mode |

`vsync.mailbox` on OpenGL falls back to `On`.

### Cameras

| Command | Arguments | Description |
|---------|-----------|-------------|
| `camera.list` | — | List cameras; active one is marked |
| `camera.select` | `<name\|id>` | Select a camera |
| `camera.next` | — | Select the next camera in insertion order |
| `camera.prev` | — | Select the previous camera |
| `camera.previous` | — | Alias for `camera.prev` |
| `camera.spawn` | `<name> [x y z] [yaw] [pitch]` | Spawn a camera; without a position it copies the active camera |
| `camera.remove` | `<name\|id>` | Remove a camera; the last remaining camera cannot be removed |
| `camera.rename` | `<name>` | Rename the active camera |
| `camera.info` | — | Print position, yaw, pitch, FOV and clip range |
| `camera.set` | `<x y z> [yaw] [pitch]` | Move the active camera |

Angles for `camera.spawn` and `camera.set` are in degrees. Positions are floats.
Camera IDs are the numbers shown by `camera.list`.

```text
camera.spawn overview 0 20 20 -90 -35
camera.select overview
camera.set 0 25 25 -90 -40
camera.info
camera.remove 1
```

## Command execution order

1. The console parses the line into a command name and whitespace-separated
   arguments.
2. Built-in commands are looked up first.
3. If no built-in matches, `Game::command` is called with the name, arguments
   and `&mut World`.
4. If the game returns `None`, the console prints `unknown command`.

Game commands cannot override built-in names.

## Adding game commands

Declare the help text and handle the command:

```rust
use runtime::{CommandContext, Game};

impl Game for MyGame {
    // ...

    fn command_help(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("teleport", "teleport <x> <y> <z>"),
            ("spawn", "spawn <x> <z>"),
        ]
    }

    fn command(&mut self, ctx: &mut CommandContext<'_>) -> Option<Result<String, String>> {
        match ctx.name {
            "teleport" => {
                if ctx.args.len() != 3 {
                    return Some(Err("usage: teleport <x> <y> <z>".into()));
                }
                let mut values = [0.0_f32; 3];
                for (slot, argument) in values.iter_mut().zip(ctx.args) {
                    *slot = match argument.parse() {
                        Ok(value) => value,
                        Err(_) => return Some(Err("invalid coordinate".into())),
                    };
                }
                if let Some(camera) = ctx.world.active_camera_mut() {
                    camera.camera.position = runtime::glam::Vec3::from(values);
                }
                Some(Ok(format!("teleported to {} {} {}", values[0], values[1], values[2])))
            }
            "spawn" => {
                if ctx.args.len() != 2 {
                    return Some(Err("usage: spawn <x> <z>".into()));
                }
                Some(Ok("spawned".into()))
            }
            _ => None,
        }
    }
}
```

`CommandContext` exposes only `name`, `args` and `world`. Game state is available
through `&mut self`; renderer handles are not exposed.

## Startup commands

`SOLDATIK3D_COMMANDS` executes commands at startup, separated by semicolons:

```bash
SOLDATIK3D_COMMANDS="camera.spawn overview 0 20 20 -90 -35;camera.select overview" \
  cargo run -p demo
```

Commands run after the world and renderer exist. An invalid command prints an
error but does not stop the game. This is useful for screenshots and smoke
tests:

```bash
SOLDATIK3D_BACKEND=opengl \
SOLDATIK3D_VSYNC=off \
SOLDATIK3D_MAX_FRAMES=3 \
SOLDATIK3D_COMMANDS="screenshot" \
cargo run -p demo
```

`SOLDATIK3D_CONSOLE=1` (also `true`, `yes`, `on`) opens the console at startup.

## Keys

| Key | Action |
|-----|--------|
| `F1` / `` ` `` | Toggle console |
| `Enter` | Execute |
| `Backspace` | Delete |
| `↑` / `↓` | History |
| `Tab` | Complete / cycle matches |
| `Esc` | Close console |
| `F12` | Engine screenshot, console open or not |

## Related

- [Configuration](configuration.md)
- [UI API](../api/ui.md)
- [Game loop](../guides/game-loop.md)
