//! My Game - A game built with Soldatik3D engine
//!
//! This is the main entry point for your game. Replace this template with your
//! own game logic. `src/main.rs` calls [`run_desktop`] for desktop builds and
//! the `android_main` export below starts the same game on Android.

use runtime::glam::Vec3;
use runtime::{
    run, Camera, CameraRig, Game, GameConfig, Light, Material, PrimaryLight, Scene, ShadowSettings,
    UpdateContext, VsyncMode, World,
};

#[cfg(target_os = "android")]
use runtime::{run_android, winit};

/// Your game state. Add whatever fields you need here.
pub struct MyGame {
    /// Camera rotation angle.
    angle: f32,
    /// Time accumulator for animations.
    time: f32,
    /// Seconds since the last FPS report.
    fps_timer: f32,
}

impl Default for MyGame {
    fn default() -> Self {
        Self {
            angle: 0.0,
            time: 0.0,
            fps_timer: 0.0,
        }
    }
}

impl Game for MyGame {
    fn config(&self) -> GameConfig {
        GameConfig::default()
            .with_title("My Soldatik3D Game")
            .with_size(1280, 720)
            .with_vsync(VsyncMode::On)
            // Uncomment to force a specific backend:
            // .with_backend(runtime::BackendKind::Vulkan)
    }

    fn create_world(&mut self) -> World {
        let scene = Scene::new(Material::default())
            .with_primary_light(PrimaryLight {
                position: Vec3::new(10.0, 10.0, 10.0),
                target: Vec3::ZERO,
                up: Vec3::Y,
                color: Vec3::new(1.0, 0.95, 0.8),
                intensity: 3.0,
                shadow: ShadowSettings::default(),
            })
            .with_lights(vec![
                Light::new(Vec3::new(3.0, 2.0, 3.0), Vec3::new(1.0, 0.5, 0.2), 10.0)
                    .with_intensity(5.0),
                Light::new(Vec3::new(-3.0, 2.0, -3.0), Vec3::new(0.2, 0.5, 1.0), 10.0)
                    .with_intensity(5.0),
            ]);

        let mut cameras = CameraRig::empty();
        let mut camera = Camera::at(Vec3::new(0.0, 2.0, 5.0));
        camera.fov_y_degrees = 70.0;
        camera.z_near = 0.1;
        camera.z_far = 1000.0;
        cameras.spawn("main", camera);

        World::with_cameras(scene, cameras)
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt;
        self.time += dt;

        // Orbit the camera around the lights.
        self.angle += dt * 0.3;
        let radius = 8.0;
        let height = 3.0;
        let cam_pos = Vec3::new(
            self.angle.cos() * radius,
            height,
            self.angle.sin() * radius,
        );

        if let Some(named_camera) = ctx.world.active_camera_mut() {
            let camera = &mut named_camera.camera;
            camera.position = cam_pos;
            let direction = (Vec3::ZERO - cam_pos).normalize();
            camera.pitch = direction.y.asin();
            camera.yaw = direction.z.atan2(direction.x);
        }

        // Animate the point lights.
        let phase = self.time * 2.0;
        for light in &mut ctx.world.scene_mut().lights {
            light.position.y = 2.0 + (phase + light.position.x).sin() * 0.5;
            light.intensity = 5.0 + phase.sin();
        }

        // Report frame statistics once per second.
        self.fps_timer += dt;
        if self.fps_timer >= 1.0 {
            let stats = ctx.stats;
            println!(
                "FPS: {:.1} | Vertices: {} | Triangles: {} | Draw calls: {}",
                1.0 / dt,
                stats.vertices,
                stats.triangles,
                stats.draw_calls
            );
            self.fps_timer = 0.0;
        }
    }
}

/// Run the game on Linux or Windows.
///
/// This is called by the desktop binary in `src/main.rs`.
pub fn run_desktop() -> anyhow::Result<()> {
    run(MyGame::default())
}

/// Android native-activity entry point. The host APK loads this shared library
/// and calls `android_main`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    if let Err(error) = run_android(MyGame::default(), app) {
        eprintln!("Game failed: {error:?}");
    }
}
