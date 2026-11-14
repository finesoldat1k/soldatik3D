# Basic Examples

Minimal, self-contained examples for common tasks.

## Hello World - Minimal Game

```rust
// src/lib.rs
use soldatik3d::{run, Game, GameConfig, UpdateContext, World};

struct HelloGame;

impl Game for HelloGame {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("Hello Soldatik3D")
    }

    fn create_world(&mut self) -> World {
        World::default()
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        if ctx.input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }
    }
}

fn main() -> anyhow::Result<()> {
    run(HelloGame)
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(HelloGame, app).unwrap();
}
```

---

## Moving Cube

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec3, Mesh, Material, Transform, Color,
};

struct CubeGame {
    cube_entity: Option<soldatik3d::EntityId>,
    angle: f32,
}

impl Default for CubeGame {
    fn default() -> Self {
        Self { cube_entity: None, angle: 0.0 }
    }
}

impl Game for CubeGame {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("Rotating Cube")
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Camera
        let cam = soldatik3d::Camera::new()
            .with_position(Vec3::new(0.0, 0.0, 5.0))
            .with_fov_y(60.0);
        world.add_camera("main".into(), cam);

        // Light
        world.add_light(soldatik3d::Light::directional()
            .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
            .with_intensity(3.0));

        // Cube
        let cube = Mesh::cube(1.0);
        let material = Material::default()
            .with_albedo(Color::rgb(0.8, 0.2, 0.2));
        self.cube_entity = Some(world.spawn_mesh(cube, Transform::identity(), material));

        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt;
        self.angle += dt;

        if let Some(id) = self.cube_entity {
            if let Some(entity) = ctx.world.entity_mut(id) {
                entity.transform.rotation = 
                    soldatik3d::Quat::from_rotation_y(self.angle) *
                    soldatik3d::Quat::from_rotation_x(self.angle * 0.7);
            }
        }

        if ctx.input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }
    }
}

fn main() -> anyhow::Result<()> { run(CubeGame::default()) }

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(CubeGame::default(), app).unwrap();
}
```

---

## FPS Controller

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec3, Vec2, Camera, Mesh, Material, Transform, Color,
    Input, KeyCode, MouseButton,
};

struct FpsGame {
    yaw: f32,
    pitch: f32,
    position: Vec3,
    velocity: Vec3,
    on_ground: bool,
}

impl Default for FpsGame {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            position: Vec3::new(0.0, 1.7, 0.0),
            velocity: Vec3::ZERO,
            on_ground: false,
        }
    }
}

impl Game for FpsGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("FPS Controller")
            .with_window_hints(soldatik3d::WindowHints {
                cursor_visible: false,
                cursor_locked: true,
                ..Default::default()
            })
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Camera will be controlled manually
        let cam = Camera::new()
            .with_position(self.position)
            .with_fov_y(90.0);
        world.add_camera("fps".into(), cam);

        // Sun
        world.add_light(soldatik3d::Light::directional()
            .with_direction(soldatik3d::Vec3::new(-0.3, -1.0, -0.5).normalize())
            .with_intensity(3.0)
            .with_cast_shadows(true));

        // Floor
        let floor = Mesh::plane(100.0, 10);
        world.spawn_mesh(floor, Transform::identity(),
            Material::default().with_albedo(soldatik3d::Color::rgb(0.3, 0.3, 0.4)));

        // Some boxes
        for i in -5..=5 {
            for j in -5..=5 {
                if (i == 0 && j == 0) || fastrand::f32() > 0.7 {
                    continue;
                }
                let cube = Mesh::cube(1.0);
                let mut transform = Transform::from_translation(
                    Vec3::new(i as f32 * 2.0, 0.5, j as f32 * 2.0)
                );
                transform.scale = Vec3::new(1.0, fastrand::f32() * 2.0 + 1.0, 1.0);
                world.spawn_mesh(cube, transform, 
                    Material::default().with_albedo(Color::rgb(
                        fastrand::f32() * 0.5 + 0.3,
                        fastrand::f32() * 0.5 + 0.3,
                        fastrand::f32() * 0.5 + 0.3,
                    ))
                );
            }
        }

        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt;
        let input = ctx.input;

        // Exit
        if input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }

        // Toggle cursor
        if input.is_key_just_pressed(soldatik3d::KeyCode::Tab) {
            // Toggle cursor lock (not directly exposed, would need platform API)
        }

        // Mouse look
        let mouse_delta = input.mouse_delta();
        self.yaw -= mouse_delta.x * 0.002;
        self.pitch = (self.pitch - mouse_delta.y * 0.002).clamp(-1.5, 1.5);

        // Movement input
        let mut move_dir = Vec3::ZERO;
        if input.is_key_pressed(KeyCode::W) { move_dir.z -= 1.0; }
        if input.is_key_pressed(KeyCode::S) { move_dir.z += 1.0; }
        if input.is_key_pressed(KeyCode::A) { move_dir.x -= 1.0; }
        if input.is_key_pressed(KeyCode::D) { move_dir.x += 1.0; }

        // Rotate move direction by yaw
        let forward = soldatik3d::Quat::from_rotation_y(self.yaw) * Vec3::Z;
        let right = soldatik3d::Quat::from_rotation_y(self.yaw) * Vec3::X;
        let move_world = forward * -move_dir.z + right * move_dir.x;

        // Physics
        const SPEED: f32 = 8.0;
        const JUMP: f32 = 12.0;
        const GRAVITY: f32 = -25.0;

        self.velocity.x = move_world.x * SPEED;
        self.velocity.z = move_world.z * SPEED;

        if input.is_key_just_pressed(KeyCode::Space) && self.on_ground {
            self.velocity.y = JUMP;
            self.on_ground = false;
        }

        self.velocity.y += GRAVITY * dt;

        // Ground check
        if self.position.y <= 1.7 && self.velocity.y <= 0.0 {
            self.position.y = 1.7;
            self.velocity.y = 0.0;
            self.on_ground = true;
        }

        self.position += self.velocity * dt;

        // Update camera
        if let Some(cam) = ctx.world.active_camera_mut() {
            cam.position = self.position;
            cam.target = self.position + soldatik3d::Quat::from_rotation_y(self.yaw) * Vec3::Z;
        }
    }
}

fn main() -> anyhow::Result<()> { run(FpsGame::default()) }

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    soldatik3d::run_android(FpsGame::default(), app).unwrap();
}
```

---

## Camera System

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Camera, Vec3, KeyCode,
};

struct CameraDemo {
    cameras: Vec<String>,
    current: usize,
}

impl Default for CameraDemo {
    fn default() -> Self {
        Self {
            cameras: vec!["main".into(), "overhead".into(), "side".into()],
            current: 0,
        }
    }
}

impl Game for CameraDemo {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("Camera System")
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Main camera
        world.add_camera("main".into(), Camera::new()
            .with_position(Vec3::new(0.0, 2.0, 5.0))
            .with_target(Vec3::ZERO)
            .with_fov_y(70.0));

        // Overhead camera
        world.add_camera("overhead".into(), Camera::new()
            .with_position(Vec3::new(0.0, 20.0, 0.0))
            .with_target(Vec3::ZERO)
            .with_fov_y(60.0));

        // Side camera
        world.add_camera("side".into(), Camera::new()
            .with_position(Vec3::new(10.0, 5.0, 0.0))
            .with_target(Vec3::ZERO)
            .with_fov_y(70.0));

        // Light
        world.add_light(soldatik3d::Light::directional()
            .with_intensity(3.0));

        // Ground
        let floor = soldatik3d::Mesh::plane(50.0, 1);
        world.spawn_mesh(floor, soldatik3d::Transform::identity(),
            soldatik3d::Material::default().with_albedo(soldatik3d::Color::rgb(0.2, 0.4, 0.2)));

        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let input = ctx.input;

        if input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }

        // Cycle cameras with C
        if input.is_key_just_pressed(soldatik3d::KeyCode::C) {
            self.current = (self.current + 1) % self.cameras.len();
            ctx.world.select_camera(&self.cameras[self.current]);
        }

        // Or use console commands:
        // camera.list
        // camera.select overhead
        // camera.spawn my_cam 0 10 0
    }
}

fn main() -> anyhow::Result<()> { run(CameraDemo::default()) }
```

---

## Light Showcase

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec3, Color, Light, Mesh, Material, Transform,
};

struct LightDemo {
    time: f32,
}

impl Default for LightDemo {
    fn default() -> Self { Self { time: 0.0 } }
}

impl Game for LightDemo {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("Light Types")
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Camera
        world.add_camera("main".into(), soldatik3d::Camera::new()
            .with_position(Vec3::new(0.0, 3.0, 10.0))
            .with_target(Vec3::ZERO)
            .with_fov_y(60.0));

        // Directional (sun)
        world.add_light(Light::directional()
            .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
            .with_color(Color::rgb(1.0, 0.95, 0.8))
            .with_intensity(3.0)
            .with_cast_shadows(true));

        // Point lights (animated)
        for i in 0..4 {
            let angle = i as f32 * std::f32::consts::TAU / 4.0;
            world.add_light(Light::point()
                .with_position(Vec3::new(angle.cos() * 5.0, 2.0, angle.sin() * 5.0))
                .with_color(Color::hsv(i as f32 * 90.0, 1.0, 1.0))
                .with_intensity(10.0)
                .with_range(15.0));
        }

        // Spot light
        world.add_light(Light::spot()
            .with_position(Vec3::new(0.0, 5.0, 0.0))
            .with_direction(Vec3::NEG_Y)
            .with_color(Color::WHITE)
            .with_intensity(20.0)
            .with_range(20.0)
            .with_angles(15.0_f32.to_radians(), 30.0_f32.to_radians()));

        // Ambient
        world.add_light(Light::ambient()
            .with_color(Color::rgb(0.1, 0.1, 0.15))
            .with_intensity(0.3));

        // Objects to illuminate
        // Central sphere
        let sphere = Mesh::sphere(1.5, 32);
        world.spawn_mesh(sphere, Transform::identity(),
            Material::default()
                .with_albedo(Color::rgb(0.8, 0.8, 0.8))
                .with_metallic(0.0)
                .with_roughness(0.3));

        // Ground
        let floor = Mesh::plane(20.0, 1);
        world.spawn_mesh(floor, Transform::identity(),
            Material::default().with_albedo(Color::rgb(0.2, 0.2, 0.3)));

        // Cubes around
        for i in -3..=3 {
            for j in -3..=3 {
                if i == 0 && j == 0 { continue; }
                let cube = Mesh::cube(1.0);
                world.spawn_mesh(cube,
                    Transform::from_translation(Vec3::new(i as f32 * 3.0, 0.5, j as f32 * 3.0)),
                    Material::default().with_albedo(Color::rgb(0.5, 0.5, 0.5)));
            }
        }

        world
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        self.time += ctx.dt;

        // Animate point lights
        for (i, light) in ctx.world.lights_mut().iter_mut().enumerate() {
            if light.light_type == soldatik3d::LightType::Point {
                let angle = self.time * 0.5 + i as f32 * std::f32::consts::TAU / 4.0;
                light.position.x = angle.cos() * 5.0;
                light.position.z = angle.sin() * 5.0;
                light.position.y = 2.0 + (self.time + i as f32).sin() * 0.5;
                light.intensity = 10.0 + self.time.sin() * 2.0;
            }
        }

        if ctx.input.is_key_just_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }
    }
}

fn main() -> anyhow::Result<()> { run(LightDemo::default()) }
```

---

## UI/HUD Example

```rust
use soldatik3d::{
    run, Game, GameConfig, UpdateContext, World,
    Vec2, Color, KeyCode,
};

struct UiDemo {
    score: u32,
    health: i32,
    max_health: i32,
}

impl Default for UiDemo {
    fn default() -> Self {
        Self { score: 0, health: 100, max_health: 100 }
    }
}

impl Game for UiDemo {
    fn config(&self) -> GameConfig {
        GameConfig::default().with_title("UI Demo")
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();
        world.add_camera("main".into(), soldatik3d::Camera::new()
            .with_position(soldatik3d::Vec3::new(0.0, 0.0, 5.0)));
        world.add_light(soldatik3d::Light::directional().with_intensity(3.0));
        
        let floor = soldatik3d::Mesh::plane(20.0, 1);
        world.spawn_mesh(floor, soldatik3d::Transform::identity(),
            soldatik3d::Material::default().with_albedo(soldatik3d::Color::rgb(0.2, 0.2, 0.3)));
        
        World::default()
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let input = ctx.input;
        let ui = ctx.renderer.ui();

        // Score
        ui.text(&format!("Score: {}", self.score), Vec2::new(10.0, 10.0), 24.0, Color::WHITE);

        // Health bar
        let bar_width = 200.0 * (self.health as f32 / self.max_health as f32);
        ui.rect(Vec2::new(10.0, 40.0), Vec2::new(200.0, 20.0), Color::rgba(0.0, 0.0, 0.0, 0.5));
        ui.rect(Vec2::new(10.0, 40.0), Vec2::new(bar_width, 20.0), Color::GREEN);
        ui.text(&format!("HP: {}/{}", self.health, self.max_health), Vec2::new(10.0, 65.0), 18.0, Color::WHITE);

        // FPS
        ui.text(&format!("FPS: {:.0}", 1.0 / ctx.dt), Vec2::new(10.0, 90.0), 18.0, Color::CYAN);

        // Crosshair
        let center = Vec2::new(ctx.renderer.extent().0 as f32 / 2.0, ctx.renderer.extent().1 as f32 / 2.0);
        ui.line(Vec2::new(center.x - 10.0, center.y), Vec2::new(center.x + 10.0, center.y), 2.0, Color::WHITE);
        ui.line(Vec2::new(center.x, center.y - 10.0), Vec2::new(center.x, center.y + 10.0), 2.0, Color::WHITE);

        // Input
        if ctx.input.is_key_just_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        if input.is_key_just_pressed(KeyCode::Space) {
            self.score += 10;
        }
        if input.is_key_just_pressed(KeyCode::H) {
            self.health = (self.health - 10).max(0);
        }
    }
}

fn main() -> anyhow::Result<()> { run(UiDemo::default()) }
```

---

*See [Getting Started](getting-started/first-game.md) for complete tutorial, or [API Reference](api/) for full API documentation.*