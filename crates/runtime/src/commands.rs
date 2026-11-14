//! Console command registry with tab completion and game extensibility.

use std::collections::HashMap;

use render::{Renderer, VsyncMode};
use scene::World;

use crate::console::Console;

/// Result of a command execution.
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Command succeeded with optional output message.
    Ok(Option<String>),
    /// Command failed with error message.
    Err(String),
}

/// Function signature for built-in engine commands.
type BuiltinHandler = dyn Fn(&mut BuiltinCommandContext<'_>) -> CommandResult + Send + Sync;

/// A registered command with metadata.
pub struct Command {
    name: String,
    description: String,
    handler: Box<BuiltinHandler>,
}

impl Command {
    pub fn new<F>(name: impl Into<String>, description: impl Into<String>, handler: F) -> Self
    where
        F: Fn(&mut BuiltinCommandContext<'_>) -> CommandResult + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            handler: Box::new(handler),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn execute(&self, ctx: &mut BuiltinCommandContext<'_>) -> CommandResult {
        (self.handler)(ctx)
    }
}

/// Registry of console commands with prefix-based lookup for tab completion.
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
    /// Sorted command names for tab completion.
    sorted_names: Vec<String>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            sorted_names: Vec::new(),
        }
    }

    /// Registers a new built-in command. Panics if name already exists.
    pub fn register(&mut self, command: Command) {
        let name = command.name().to_owned();
        if self.commands.contains_key(&name) {
            panic!("command '{}' already registered", name);
        }
        self.commands.insert(name.clone(), command);
        self.rebuild_sorted();
    }

    /// Returns the command if found.
    pub fn get(&self, name: &str) -> Option<&Command> {
        self.commands.get(name)
    }

    /// Returns all command names matching the prefix (for tab completion).
    pub fn complete(&self, prefix: &str) -> Vec<&str> {
        self.sorted_names
            .iter()
            .filter(|name| name.starts_with(prefix))
            .map(|s| s.as_str())
            .collect()
    }

    /// Returns all registered commands for help display (owned).
    pub fn all(&self) -> Vec<&Command> {
        self.commands.values().collect()
    }

    fn rebuild_sorted(&mut self) {
        let mut names: Vec<String> = self.commands.keys().cloned().collect();
        names.sort();
        self.sorted_names = names;
    }
}

/// Context passed to built-in command handlers.
/// Different from `crate::game::CommandContext` - this has access to engine internals.
pub struct BuiltinCommandContext<'a> {
    /// The game's world.
    pub world: &'a mut World,
    /// The engine console for output.
    pub console: &'a mut Console,
    /// The renderer for backend-specific commands.
    pub renderer: Option<&'a mut Box<dyn Renderer>>,
    /// Command arguments.
    pub args: &'a [&'a str],
    /// Exit flag.
    pub exit_requested: &'a mut bool,
}

impl<'a> BuiltinCommandContext<'a> {
    pub fn new(
        world: &'a mut World,
        console: &'a mut Console,
        renderer: Option<&'a mut Box<dyn Renderer>>,
        args: &'a [&'a str],
        exit_requested: &'a mut bool,
    ) -> Self {
        Self {
            world,
            console,
            renderer,
            args,
            exit_requested,
        }
    }
}

/// Creates the default engine command registry.
pub fn create_default_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(Command::new("help", "Show this help message", |ctx| {
        ctx.console.print("Soldatik3D console commands:");
        // Built-in commands are printed by the registry in print_help
        // Game commands are printed by the game's command_help
        CommandResult::Ok(None)
    }));

    registry.register(Command::new("clear", "Clear console history", |ctx| {
        ctx.console.clear();
        CommandResult::Ok(None)
    }));

    registry.register(Command::new("quit", "Exit the application", |ctx| {
        *ctx.exit_requested = true;
        ctx.console.print("shutting down");
        CommandResult::Ok(None)
    }));

    registry.register(Command::new("exit", "Alias for quit", |ctx| {
        *ctx.exit_requested = true;
        ctx.console.print("shutting down");
        CommandResult::Ok(None)
    }));

    registry.register(Command::new(
        "backend",
        "Show current graphics backend",
        |ctx| {
            let name = ctx
                .renderer
                .as_ref()
                .map(|r| r.backend().name())
                .unwrap_or("none");
            ctx.console.print(format!(
                "backend: {name} (SOLDATIK3D_BACKEND=vulkan|opengl)"
            ));
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "screenshot",
        "Save current frame to PNG",
        |ctx| {
            if let Some(renderer) = &mut ctx.renderer {
                match renderer.screenshot() {
                    Ok(()) => ctx.console.print("screenshot requested"),
                    Err(e) => ctx.console.error(format!("screenshot failed: {e}")),
                }
            }
            CommandResult::Ok(None)
        },
    ));

    // Camera commands
    registry.register(Command::new("camera.list", "List all cameras", |ctx| {
        let active = ctx.world.active_camera_id();
        let lines: Vec<String> = ctx
            .world
            .cameras()
            .iter()
            .map(|cam| {
                let marker = if Some(cam.id) == active { "*" } else { " " };
                format!(
                    "{marker} {:<16} {}  pos ({:.1}, {:.1}, {:.1})",
                    cam.name,
                    cam.id,
                    cam.camera.position.x,
                    cam.camera.position.y,
                    cam.camera.position.z
                )
            })
            .collect();
        ctx.console.print(format!("{} camera(s):", lines.len()));
        for line in lines {
            ctx.console.print(format!("  {line}"));
        }
        CommandResult::Ok(None)
    }));

    registry.register(Command::new(
        "camera.select",
        "Select active camera by name or ID",
        |ctx| {
            let Some(token) = ctx.args.first() else {
                return CommandResult::Err("usage: camera.select <name|id>".into());
            };
            let selected = if ctx.world.select_camera_name(token) {
                true
            } else {
                crate::app::resolve_camera(ctx.world, token)
                    .is_some_and(|id| ctx.world.select_camera(id))
            };
            if selected {
                let name = ctx.world.active_camera_name().to_owned();
                ctx.console.print(format!("active camera: {name}"));
                CommandResult::Ok(None)
            } else {
                CommandResult::Err(format!("unknown camera: {token}"))
            }
        },
    ));

    registry.register(Command::new(
        "camera.next",
        "Select next camera in ring",
        |ctx| {
            if let Some(id) = ctx.world.select_next_camera() {
                ctx.console.print(format!(
                    "active camera: {} ({id})",
                    ctx.world.active_camera_name()
                ));
                CommandResult::Ok(None)
            } else {
                CommandResult::Err("no cameras".into())
            }
        },
    ));

    registry.register(Command::new(
        "camera.prev",
        "Select previous camera in ring",
        |ctx| {
            if let Some(id) = ctx.world.select_previous_camera() {
                ctx.console.print(format!(
                    "active camera: {} ({id})",
                    ctx.world.active_camera_name()
                ));
                CommandResult::Ok(None)
            } else {
                CommandResult::Err("no cameras".into())
            }
        },
    ));

    registry.register(Command::new(
        "camera.previous",
        "Alias for camera.prev",
        |ctx| {
            if let Some(id) = ctx.world.select_previous_camera() {
                ctx.console.print(format!(
                    "active camera: {} ({id})",
                    ctx.world.active_camera_name()
                ));
                CommandResult::Ok(None)
            } else {
                CommandResult::Err("no cameras".into())
            }
        },
    ));

    registry.register(Command::new("camera.spawn", "Spawn a new camera", |ctx| {
        let Some(name) = ctx.args.first() else {
            return CommandResult::Err("usage: camera.spawn <name> [x y z] [yaw] [pitch]".into());
        };
        let mut camera = ctx.world.active_camera();
        if ctx.args.len() > 1 {
            if ctx.args.len() < 4 {
                return CommandResult::Err("camera.spawn: provide all of x y z, or none".into());
            }
            let Some(position) = crate::app::parse_vec3(&ctx.args[1..4]) else {
                return CommandResult::Err("camera.spawn: invalid position".into());
            };
            camera.position = position;
        }
        if let Some(yaw) = ctx.args.get(4) {
            match yaw.parse::<f32>() {
                Ok(value) => camera.yaw = value.to_radians(),
                Err(_) => return CommandResult::Err("camera.spawn: invalid yaw".into()),
            }
        }
        if let Some(pitch) = ctx.args.get(5) {
            match pitch.parse::<f32>() {
                Ok(value) => camera.pitch = value.to_radians(),
                Err(_) => return CommandResult::Err("camera.spawn: invalid pitch".into()),
            }
        }
        let id = ctx.world.spawn_camera(*name, camera);
        let resolved = ctx
            .world
            .cameras()
            .get(id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| (*name).to_owned());
        ctx.console
            .print(format!("spawned camera {resolved} ({id})"));
        CommandResult::Ok(None)
    }));

    registry.register(Command::new(
        "camera.remove",
        "Remove a camera (cannot remove the last one)",
        |ctx| {
            let Some(token) = ctx.args.first() else {
                return CommandResult::Err("usage: camera.remove <name|id>".into());
            };
            let Some(id) = crate::app::resolve_camera(ctx.world, token) else {
                return CommandResult::Err(format!("unknown camera: {token}"));
            };
            match ctx.world.remove_camera(id) {
                Some(removed) => {
                    ctx.console
                        .print(format!("removed camera {} ({id})", removed.name));
                    CommandResult::Ok(None)
                }
                None => CommandResult::Err("cannot remove the last remaining camera".into()),
            }
        },
    ));

    registry.register(Command::new(
        "camera.rename",
        "Rename the active camera",
        |ctx| {
            let Some(name) = ctx.args.first() else {
                return CommandResult::Err("usage: camera.rename <name>".into());
            };
            match ctx.world.cameras_mut().rename_active(*name) {
                Some(resolved) => {
                    ctx.console.print(format!("active camera: {resolved}"));
                    CommandResult::Ok(None)
                }
                None => CommandResult::Err("no active camera".into()),
            }
        },
    ));

    registry.register(Command::new(
        "camera.info",
        "Show detailed info for active camera",
        |ctx| {
            let Some(camera) = ctx.world.active_camera_ref() else {
                return CommandResult::Err("no active camera".into());
            };
            let camera = camera.camera;
            ctx.console.print(format!(
                "{}: pos ({:.2}, {:.2}, {:.2}) yaw {:.1}° pitch {:.1}° fov {:.1}° range {:.2}..{:.1}",
                ctx.world.active_camera_name(),
                camera.position.x,
                camera.position.y,
                camera.position.z,
                camera.yaw.to_degrees(),
                camera.pitch.to_degrees(),
                camera.fov_y_degrees,
                camera.z_near,
                camera.z_far
            ));
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "camera.set",
        "Move active camera to position",
        |ctx| {
            if ctx.args.len() < 3 {
                return CommandResult::Err("usage: camera.set x y z [yaw] [pitch]".into());
            }
            let Some(position) = crate::app::parse_vec3(&ctx.args[0..3]) else {
                return CommandResult::Err("camera.set: invalid position".into());
            };
            let yaw = ctx.args.get(3).and_then(|v| v.parse::<f32>().ok());
            let pitch = ctx.args.get(4).and_then(|v| v.parse::<f32>().ok());
            let Some(camera) = ctx.world.active_camera_mut() else {
                return CommandResult::Err("no active camera".into());
            };
            camera.camera.position = position;
            if let Some(yaw) = yaw {
                camera.camera.yaw = yaw.to_radians();
            }
            if let Some(pitch) = pitch {
                camera.camera.pitch = pitch.to_radians();
            }
            let name = camera.name.clone();
            ctx.console.print(format!("moved camera {name}"));
            CommandResult::Ok(None)
        },
    ));

    // VSync commands
    registry.register(Command::new("vsync.toggle", "Toggle VSync on/off", |ctx| {
        if let Some(renderer) = &mut ctx.renderer {
            let new_vsync = match renderer.vsync() {
                VsyncMode::On => VsyncMode::Off,
                _ => VsyncMode::On,
            };
            let _ = renderer.set_vsync(new_vsync);
            ctx.console.print(format!("vsync: {:?}", new_vsync));
        } else {
            ctx.console.error("no renderer available");
        }
        CommandResult::Ok(None)
    }));

    registry.register(Command::new(
        "vsync.on",
        "Enable VSync (wait for vblank)",
        |ctx| {
            if let Some(renderer) = &mut ctx.renderer {
                let _ = renderer.set_vsync(VsyncMode::On);
                ctx.console.print("vsync: On");
            } else {
                ctx.console.error("no renderer available");
            }
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "vsync.off",
        "Disable VSync (immediate present, may tear)",
        |ctx| {
            if let Some(renderer) = &mut ctx.renderer {
                let _ = renderer.set_vsync(VsyncMode::Off);
                ctx.console.print("vsync: Off");
            } else {
                ctx.console.error("no renderer available");
            }
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "vsync.mailbox",
        "Enable Mailbox (triple buffering, no tear, lowest latency)",
        |ctx| {
            if let Some(renderer) = &mut ctx.renderer {
                let _ = renderer.set_vsync(VsyncMode::Mailbox);
                ctx.console.print("vsync: Mailbox");
            } else {
                ctx.console.error("no renderer available");
            }
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "vsync.adaptive",
        "Enable Adaptive VSync (wait if late, otherwise immediate)",
        |ctx| {
            if let Some(renderer) = &mut ctx.renderer {
                let _ = renderer.set_vsync(VsyncMode::Adaptive);
                ctx.console.print("vsync: Adaptive");
            } else {
                ctx.console.error("no renderer available");
            }
            CommandResult::Ok(None)
        },
    ));

    registry.register(Command::new(
        "vsync.current",
        "Show current VSync mode",
        |ctx| {
            if let Some(renderer) = &ctx.renderer {
                ctx.console.print(format!("vsync: {:?}", renderer.vsync()));
            } else {
                ctx.console.error("no renderer available");
            }
            CommandResult::Ok(None)
        },
    ));

    registry
}
