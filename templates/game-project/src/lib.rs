//! My Game - A game built with Soldatik3D engine
//!
//! This is the main entry point for your game. Replace this template with your
//! own game logic.

use soldatik3d::{
    run,
    #[cfg(target_os = "android")]
    run_android,
    Camera,
    Game,
    GameConfig,
    UpdateContext,
    World,
    Vec3,
    Mat4,
};

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
            .with_window_size(1280, 720)
            .with_vsync(true)
            // Uncomment to force a specific backend:
            // .with_backend(soldatik3d::BackendKind::Vulkan)
    }

    fn create_world(&mut self) -> World {
        let mut world = World::default();

        // Create a camera
        let camera = soldatik3d::Camera::new()
            .with_position(Vec3::new(0.0, 2.0, 5.0))
            .with_target(Vec3::new(0.0, 1.0, 0.0))
            .with_fov_y(70.0)
            .with_near_far(0.1, 1000.0);
        world.add_camera("main".to_string(), camera);

        // Add a directional light (sun)
        let sun = soldatik3d::Light::directional()
            .with_direction(Vec3::new(-0.5, -1.0, -0.3).normalize())
            .with_color(soldatik3d::Color::rgb(1.0, 0.95, 0.8))
            .with_intensity(3.0)
            .with_cast_shadows(true);
        world.add_light(sun);

        // Add ambient light
        let ambient = soldatik3d::Light::ambient()
            .with_color(soldatik3d::Color::rgb(0.2, 0.2, 0.3))
            .with_intensity(0.5);
        world.add_light(ambient);

        // Add some point lights for atmosphere
        let light1 = soldatik3d::Light::point()
            .with_position(Vec3::new(3.0, 2.0, 3.0))
            .with_color(soldatik3d::Color::rgb(1.0, 0.5, 0.2))
            .with_intensity(5.0)
            .with_radius(10.0);
        world.add_light(light1);

        let light2 = soldatik3d::Light::point()
            .with_position(Vec3::new(-3.0, 2.0, -3.0))
            .with_color(soldatik3d::Color::rgb(0.2, 0.5, 1.0))
            .with_intensity(5.0)
            .with_radius(10.0);
        world.add_light(light2)

        world
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

        if let Some(camera) = ctx.world.active_camera_mut() {
            camera.position = cam_pos;
            camera.target = Vec3::new(0.0, 1.0, 0.0);
        }

        // Animate point lights
        if let Some(lights) = ctx.world.lights_mut() {
            for light in lights {
                if light.light_type == soldatik3d::LightType::Point {
                    let phase = self.time * 2.0;
                    light.position.y = 2.0 + (phase + light.position.x).sin() * 0.5;
                    light.intensity = 5.0 + phase.sin() * 1.0;
                }
            }
        }

        // Handle input
        let input = ctx.input;
        if input.is_key_pressed(soldatik3d::KeyCode::Escape) {
            ctx.request_exit();
        }

        // Toggle wireframe mode with F1
        if input.is_key_just_pressed(soldatik3d::KeyCode::F1) {
            ctx.renderer.set_debug_view(soldatik3d::DebugView::Wireframe);
        }
        if input.is_key_just_pressed(soldatik3d::KeyCode::F2) {
            ctx.renderer.set_debug_view(soldatik3d::DebugView::Normals);
        }
        if input.is_key_just_pressed(soldatik3d::KeyCode::F3) {
            ctx.renderer.set_debug_view(soldatik3d::DebugView::Depth);
        }
        if input.is_key_just_pressed(soldatik3d::KeyCode::F4) {
            ctx.renderer.set_debug_view(soldatik3d::DebugView::Lit);
        }

        // Take screenshot with F12
        if input.is_key_just_pressed(soldatik3d::KeyCode::F12) {
            ctx.commands.execute("screenshot");
        }

        // Print FPS every second
        static mut LAST_FPS_PRINT: f32 = 0.0;
        unsafe {
            LAST_FPS_PRINT += dt;
            if LAST_FPS_PRINT >= 1.0 {
                let stats = ctx.renderer.stats();
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
pub fn android_main(app: soldatik3d::winit::platform::android::activity::AndroidApp) {
    if let Err(e) = run_android(MyGame::default(), app) {
        eprintln!("Game failed: {e:?}");
    }
}