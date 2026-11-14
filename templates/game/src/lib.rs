//! My Game - A game built with Soldatik3D engine
//!
//! This is the main entry point for your game. Replace this template with your
//! own game logic.

use runtime::glam::{Vec3, Mat4};
use runtime::{
    run,
    Camera,
    CameraRig,
    Game,
    GameConfig,
    Light,
    Material,
    Mesh,
    PrimaryLight,
    Scene,
    ShadowSettings,
    UpdateContext,
    World,
    VsyncMode,
    winit,
};

#[cfg(target_os = "android")]
use runtime::run_android;

/// Your game state. Add whatever fields you need here.
pub struct MyGame {
    /// Camera rotation angle
    angle: f32,
    /// Time accumulator for animations
    time: f32,
}

impl Default for MyGame {
    fn default() -> Self {
        Self {
            angle: 0.0,
            time: 0.0,
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
        // Build the scene (like the demo does)
        let material = Material::default();

        // Primary light (sun) with shadows
        let primary_light = PrimaryLight {
            position: Vec3::new(10.0, 10.0, 10.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            color: Vec3::new(1.0, 0.95, 0.8),
            intensity: 3.0,
            shadow: ShadowSettings::default(),
        };

        // Point lights
        let lights = vec![
            Light::new(Vec3::new(3.0, 2.0, 3.0), Vec3::new(1.0, 0.5, 0.2), 10.0)
                .with_intensity(5.0),
            Light::new(Vec3::new(-3.0, 2.0, -3.0), Vec3::new(0.2, 0.5, 1.0), 10.0)
                .with_intensity(5.0),
        ];

        // Simple scene with just a primary light and point lights
        // (no meshes - just lighting for testing)
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

        // Create camera rig (like the demo does)
        let mut cameras = CameraRig::empty();
        let mut camera = Camera::at(Vec3::new(0.0, 2.0, 5.0));
        camera.fov_y_degrees = 70.0;
        camera.z_near = 0.1;
        camera.z_far = 1000.0;
        cameras.spawn("main", camera);

        // Create world with scene and cameras (like the demo does)
        World::with_cameras(scene, cameras)
    }

    fn update(&mut self, ctx: &mut UpdateContext<'_>) {
        let dt = ctx.dt;

        // Update time
        self.time += dt;

        // Rotate camera around the scene
        self.angle += dt * 0.3;
        let radius = 8.0;
        let height = 3.0;
        let cam_pos = Vec3::new(
            self.angle.cos() * radius,
            height,
            self.angle.sin() * radius,
        );

        if let Some(named_camera) = ctx.world.active_camera_mut() {
            named_camera.camera.position = cam_pos;
            named_camera.camera.yaw = 0.0;
            named_camera.camera.pitch = 0.0;
        }

        // Animate point lights
        let lights = &mut ctx.world.scene_mut().lights;
        for light in lights {
            // Animate all lights (engine distinguishes by radius > 0 for point lights)
            let phase = self.time * 2.0;
            light.position.y = 2.0 + (phase + light.position.x).sin() * 0.5;
            light.intensity = 5.0 + phase.sin() * 1.0;
        }

        // Handle input
        let input = ctx.input;
        if input.key_pressed(winit::keyboard::KeyCode::Escape) {
            // Exit is handled via console command "quit" or "exit"
        }

        // Print FPS every second using ctx.stats
        static mut LAST_FPS_PRINT: f32 = 0.0;
        unsafe {
            LAST_FPS_PRINT += dt;
            if LAST_FPS_PRINT >= 1.0 {
                let stats = ctx.stats;
                println!("FPS: {:.1} | Vertices: {} | Triangles: {} | Draw calls: {}",
                    1.0 / dt, stats.vertices, stats.triangles, stats.draw_calls);
                LAST_FPS_PRINT = 0.0;
            }
        }
    }
}

/// Desktop entry point
fn main() -> anyhow::Result<()> {
    run(MyGame::default())
}

/// Android native-activity entry point
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: runtime::winit::platform::android::activity::AndroidApp) {
    if let Err(e) = run_android(MyGame::default(), app) {
        eprintln!("Game failed: {e:?}");
    }
}