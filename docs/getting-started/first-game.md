# Your First Game - Complete Tutorial

Build a complete, playable 3D game step by step. By the end, you'll have a game with:
- Player movement and camera control
- Procedural level generation
- Collectibles and scoring
- Win/lose conditions
- Polish (particles, UI, sound)

**Estimated time:** 30-45 minutes

---

## Step 0: Setup

```bash
soldatik3d new my_first_game
cd my_first_game
cargo run   # Verify it works
```

Press `~` to open console, try `help`, `debug.normals`, `screenshot`.

---

## Step 1: Player Movement

Replace `src/lib.rs` with:

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec3, Quat, Input, KeyCode, MouseButton,
};

struct MyGame {
    player_pos: Vec3,
    player_vel: Vec3,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
}

impl Default for MyGame {
    fn default() -> Self {
        Self {
            player_pos: Vec3::new(0.0, 1.0, 0.0),
            player_vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
        }
    }
}

impl Game for MyGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("My First Game - Platformer")
            .with_window_size(1280, 720)
            .with_vsync(true)
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Camera
        let cam = soldatik3d::Camera::new()
            .with_position(self.player_pos + Vec3::Y * 1.7)
            .with_target(self.player_pos + Vec3::Y * 1.7 + Vec3::Z)
            .with_fov_y(75.0);
        world.add_camera("player".to_string(), cam);

        // Sun light
        let sun = soldatik3d::Light::directional()
            .with_direction(Vec3::new(-0.3, -1.0, -0.5).normalize())
            .with_color(soldatik3d::Color::rgb(1.0, 0.95, 0.85))
            .with_intensity(3.0)
            .with_cast_shadows(true);
        world.add_light(sun);

        // Ambient
        world.add_light(soldatik3d::Light::ambient()
            .with_color(soldatik3d::Color::rgb(0.3, 0.3, 0.4))
            .with_intensity(0.5));

        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt;
        let input = ctx.input;

        // --- Camera rotation (mouse) ---
        let mouse_delta = input.mouse_delta();
        self.yaw -= mouse_delta.x * 0.002;
        self.pitch = (self.pitch - mouse_delta.y * 0.002).clamp(-1.5, 1.5);

        // --- Movement input ---
        let mut move_dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::W) { move_dir.z -= 1.0; }
        if input.is_key_pressed(KeyCode::S) { move_dir.z += 1.0; }
        if input.is_key_pressed(KeyCode::A) { move_dir.x -= 1.0; }
        if input.is_key_pressed(KeyCode::D) { move_dir.x += 1.0; }

        // Rotate move direction by yaw
        let forward = Quat::from_rotation_y(self.yaw) * Vec3::Z;
        let right = Quat::from_rotation_y(self.yaw) * Vec3::X;
        let move_world = forward * -move_dir.z + right * move_dir.x;

        // --- Physics ---
        const SPEED: f32 = 8.0;
        const JUMP_FORCE: f32 = 12.0;
        const GRAVITY: f32 = -25.0;
        const DRAG: f32 = 10.0;

        // Horizontal movement
        self.player_vel.x = move_world.x * SPEED;
        self.player_vel.z = move_world.z * SPEED;

        // Jump
        if input.is_key_just_pressed(KeyCode::Space) && self.on_ground {
            self.player_vel.y = JUMP_FORCE;
            self.on_ground = false;
        }

        // Gravity
        self.player_vel.y += GRAVITY * dt;

        // Ground check (simple)
        if self.player_pos.y <= 1.0 && self.player_vel.y <= 0.0 {
            self.player_pos.y = 1.0;
            self.player_vel.y = 0.0;
            self.on_ground = true;
        }

        // Apply velocity
        self.player_pos += self.player_vel * dt;

        // Update camera
        if let Some(cam) = ctx.world.active_camera_mut() {
            cam.position = self.player_pos + Vec3::Y * 1.7;
            cam.target = cam.position + Quat::from_rotation_y(self.yaw) * Vec3::Z;
        }

        // Exit
        if input.is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
    }
}

fn main() -> anyhow::Result<()> { run(MyGame::default()) }

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(MyGame::default(), app).unwrap();
}
```

**Test:** `cargo run` - Move with WASD, jump with Space, look with mouse.

---

## Step 2: Procedural Level

Add level generation in `create_world`:

```rust
fn create_world(&mut self) -> World {
    let mut world = World::default();

    // ... camera and lights from Step 1 ...

    // Generate floor
    for x in -20..=20 {
        for z in -20..=20 {
            if (x == 0 && z == 0) || fastrand::f32() > 0.15 {
                let cube = soldatik3d::Mesh::cube(1.0);
                let mut transform = soldatik3d::Transform::default();
                transform.translation = Vec3::new(x as f32, 0.0, z as f32);
                world.spawn_mesh(cube, transform, soldatik3d::Material::default()
                    .with_albedo(soldatik3d::Color::rgb(0.4, 0.4, 0.5)));
            }
        }
    }

    // Add some platforms
    for i in 0..10 {
        let platform = soldatik3d::Mesh::cube(3.0);
        let mut transform = soldatik3d::Transform::default();
        transform.translation = Vec3::new(
            fastrand::f32() * 30.0 - 15.0,
            fastrand::f32() * 5.0 + 2.0,
            fastrand::f32() * 30.0 - 15.0,
        );
        transform.scale = Vec3::new(1.0, 0.2, 1.0);
        world.spawn_mesh(platform, transform, soldatik3d::Material::default()
            .with_albedo(soldatik3d::Color::rgb(0.3, 0.6, 0.9)));
    }

    world
}
```

Add `fastrand = "2.0"` to `Cargo.toml` dependencies.

---

## Step 3: Collectibles (Coins)

Add to `MyGame` struct:

```rust
struct MyGame {
    // ... existing fields ...
    coins: Vec<Coin>,
    score: u32,
}

struct Coin {
    entity: soldatik3d::EntityId,
    position: Vec3,
    rotation: f32,
    collected: bool,
}
```

In `create_world`, spawn coins:

```rust
use soldatik3d::EntityId;

self.coins.clear();
for i in 0..30 {
    let coin_mesh = soldatik3d::Mesh::cylinder(0.3, 0.1, 16);
    let mut transform = soldatik3d::Transform::default();
    transform.translation = Vec3::new(
        fastrand::f32() * 30.0 - 15.0,
        fastrand::f32() * 3.0 + 1.5,
        fastrand::f32() * 30.0 - 15.0,
    );
    let entity = world.spawn_mesh(coin_mesh, transform, soldatik3d::Material::default()
        .with_albedo(soldatik3d::Color::rgb(1.0, 0.85, 0.1))
        .with_emissive(soldatik3d::Color::rgb(1.0, 0.8, 0.0), 2.0));

    self.coins.push(Coin {
        entity,
        position: transform.translation,
        rotation: fastrand::f32() * std::f32::consts::TAU,
        collected: false,
    });
}
```

In `update`, check collection:

```rust
// Rotate and bob coins
for coin in &mut self.coins {
    if coin.collected { continue; }
    coin.rotation += dt * 2.0;
    coin.position.y += (ctx.time * 3.0 + coin.rotation).sin() * 0.01;

    // Update visual position
    if let Some(entity) = ctx.world.entity_mut(coin.entity) {
        entity.transform.translation = coin.position;
        entity.transform.rotation = Quat::from_rotation_y(coin.rotation);
    }

    // Check collection
    if (self.player_pos - coin.position).length() < 1.0 {
        coin.collected = true;
        self.score += 10;
        ctx.console.print(format!("Coin! Score: {}", self.score));
        // Remove visual
        if let Some(entity) = ctx.world.entity_mut(coin.entity) {
            entity.visible = false;
        }
    }
}
```

---

## Step 4: Win Condition & UI

Add timer and win check:

```rust
struct MyGame {
    // ... existing ...
    start_time: f32,
    game_won: bool,
}

impl Default for MyGame {
    fn default() -> Self {
        Self {
            // ...
            start_time: 0.0,
            game_won: false,
        }
    }
}

fn update(&mut self, ctx: &mut UpdateContext<'_>) {
    let dt = ctx.dt;

    if !self.game_won {
        self.start_time += dt;

        // Win condition: all coins collected
        if self.coins.iter().all(|c| c.collected) {
            self.game_won = true;
            ctx.console.print(format!("YOU WIN! Time: {:.1}s", self.start_time));
        }
    }

    // ... existing movement code ...

    // UI overlay
    let ui = ctx.renderer.ui();
    ui.text(&format!("Score: {}", self.score), Vec2::new(10.0, 10.0), 24.0, soldatik3d::Color::WHITE);
    ui.text(&format!("Time: {:.1}s", self.start_time), Vec2::new(10.0, 40.0), 24.0, soldatik3d::Color::WHITE);
    ui.text(&format!("Coins: {}/30", self.coins.iter().filter(|c| !c.collected).count()), Vec2::new(10.0, 70.0), 24.0, soldatik3d::Color::WHITE);

    if self.game_won {
        ui.text("YOU WIN! Press R to restart", Vec2::new(400.0, 360.0), 48.0, soldatik3d::Color::GREEN);
        if ctx.input.is_key_just_pressed(KeyCode::R) {
            *self = MyGame::default(); // Reset
        }
    }
}
```

---

## Step 5: Polish - Particles & Effects

Add coin sparkle particles:

```rust
// In create_world, after coin spawn
let particle_mesh = soldatik3d::Mesh::quad(0.1);
for coin in &self.coins {
    // Particles handled in update via custom render
}
```

Add screen shake on jump:

```rust
// In update, when jumping
if input.is_key_just_pressed(KeyCode::Space) && self.on_ground {
    self.player_vel.y = JUMP_FORCE;
    self.on_ground = false;
    ctx.renderer.add_screen_shake(0.3, 0.2); // intensity, duration
}
```

Add footstep sounds (requires audio crate):

```toml
# Cargo.toml
rodio = "0.19"
```

```rust
use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

lazy_static::lazy_static! {
    static ref AUDIO: (OutputStream, Sink) = {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();
        (stream, sink)
    };
}

fn play_sound(data: &[u8]) {
    let cursor = Cursor::new(data);
    if let Ok(source) = Decoder::new(cursor) {
        AUDIO.1.append(source);
    }
}

// In update, on step
if self.on_ground && move_dir.length() > 0.1 && fastrand::f32() < 0.05 {
    play_sound(include_bytes!("assets/sounds/step.ogg"));
}
```

---

## Step 6: Build & Share

```bash
# Release build
soldatik3d build --release

# Export all platforms
soldatik3d export

# Output in dist/
ls dist/
# my_game-linux-x86_64.tar.gz
# my_game-windows-x86_64.zip
# my_game-android-arm64-v8a.apk
```

---

## Complete Source

The complete `src/lib.rs` is available at:
`templates/examples/first_game_complete.rs` (in the engine repo)

---

## What You Learned

| Concept | Implementation |
|---------|----------------|
| Game trait | `Game` with `config`, `create_world`, `update` |
| Camera control | Mouse delta → yaw/pitch → quaternion rotation |
| Physics | Velocity + gravity + ground check |
| Procedural generation | `fastrand` in `create_world` |
| Entity management | `World::spawn_mesh`, `entity_mut` |
| Collision detection | Distance check (simple) |
| UI rendering | `ctx.renderer.ui().text()` |
| State management | Struct fields + enums |
| Cross-platform | Same code runs on Linux, Windows, Android |

---

## Next Challenges

1. **Add enemies** - Simple AI that chases player
2. **Add weapons** - Raycast shooting with particles
3. **Save/Load** - Serialize game state with `serde`
4. **Multiplayer** - Use `tokio` + `bevy_net` (advanced)
5. **Custom shaders** - Write GLSL for unique visual effects
6. **Level editor** - Build in-game editor using UI system

---

## Full API Reference

- [Runtime API](api/runtime.md) - `Game`, `GameConfig`, `UpdateContext`
- [Scene API](api/scene.md) - `World`, `Entity`, `Mesh`, `Material`
- [Input API](api/input.md) - Keyboard, mouse, gamepad
- [Rendering API](api/rendering.md) - `Renderer`, `Camera`, `Light`

---

*Congratulations! You've built a complete 3D game with Soldatik3D! 🎉*