//! First-person player: camera, look and movement on top of [`crate::physics`].

use runtime::glam::Vec3;
use runtime::winit::keyboard::KeyCode;
use runtime::{Camera, Input, InputMap};

use crate::physics::{Character, EYE_HEIGHT, PhysicsWorld};

const BASE_FOV: f32 = 70.0;
const SPRINT_FOV: f32 = 78.0;
const MAX_PITCH: f32 = 88.0_f32 * std::f32::consts::PI / 180.0;

pub struct Player {
    pub body: Character,
    pub yaw: f32,
    pub pitch: f32,
    fov: f32,
    bob_time: f32,
    bob_amount: f32,
    /// Negative offset used to smooth camera height after an automatic step.
    step_offset: f32,
}

impl Player {
    pub fn new(position: Vec3, yaw: f32) -> Self {
        Self {
            body: Character::new(position),
            yaw,
            pitch: 0.0,
            fov: BASE_FOV,
            bob_time: 0.0,
            bob_amount: 0.0,
            step_offset: 0.0,
        }
    }

    pub fn teleport(&mut self, position: Vec3, yaw: f32) {
        self.body.teleport(position);
        self.yaw = yaw;
        self.pitch = 0.0;
        self.bob_time = 0.0;
        self.bob_amount = 0.0;
        self.step_offset = 0.0;
    }

    pub fn is_flying(&self) -> bool {
        self.body.flying
    }

    pub fn set_flying(&mut self, flying: bool) {
        self.body.set_flying(flying);
    }

    /// Applies a look delta in radians.
    pub fn look(&mut self, yaw_delta: f32, pitch_delta: f32) {
        self.yaw = (self.yaw + yaw_delta).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + pitch_delta).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Mouse input is in pixels; the caller supplies a radians-per-pixel value.
    pub fn look_mouse(&mut self, delta: runtime::glam::Vec2, sensitivity: f32, invert_y: bool) {
        if delta.length_squared() <= 0.0 {
            return;
        }
        let yaw = delta.x * sensitivity;
        let pitch = delta.y * sensitivity * if invert_y { 1.0 } else { -1.0 };
        self.look(yaw, pitch);
    }

    /// Keyboard camera rotation, independent of pointer lock.
    pub fn look_keyboard(&mut self, input: &Input, dt: f32) {
        let speed = 2.2 * dt;
        let mut yaw = 0.0;
        let mut pitch = 0.0;
        if input.action_down("camera_rot_left") {
            yaw -= speed;
        }
        if input.action_down("camera_rot_right") {
            yaw += speed;
        }
        if input.action_down("camera_rot_up") {
            pitch += speed;
        }
        if input.action_down("camera_rot_down") {
            pitch -= speed;
        }
        if yaw != 0.0 || pitch != 0.0 {
            self.look(yaw, pitch);
        }
    }

    /// World-space movement direction for `(strafe, forward)` axes.
    pub fn wish_direction(&self, axes: runtime::glam::Vec2) -> Vec3 {
        if axes.length_squared() < 1e-6 {
            return Vec3::ZERO;
        }
        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let right = Vec3::new(-forward.z, 0.0, forward.x);
        (forward * axes.y + right * axes.x).normalize_or_zero()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        dt: f32,
        world: &PhysicsWorld,
        axes: runtime::glam::Vec2,
        jump_pressed: bool,
        crouch: bool,
        sprint: bool,
    ) {
        let previous_y = self.body.position.y;
        let wish = self.wish_direction(axes);
        self.body
            .update(dt, world, wish, jump_pressed, crouch, sprint);

        // Smooth the camera up when the controller snapped onto a low step.
        let rise = self.body.position.y - previous_y;
        if self.body.on_ground && rise > 0.0 && rise <= 0.5 {
            self.step_offset = (self.step_offset - rise).max(-0.5);
        }
        self.step_offset *= (-18.0 * dt.max(0.0)).exp();

        let horizontal_speed = Vec3::new(self.body.velocity.x, 0.0, self.body.velocity.z).length();
        let target_fov = if sprint && horizontal_speed > 6.0 {
            SPRINT_FOV
        } else {
            BASE_FOV
        };
        self.fov += (target_fov - self.fov) * (1.0 - (-6.0 * dt).exp());

        let moving = self.body.on_ground && horizontal_speed > 0.4;
        let target_bob = if moving {
            (horizontal_speed / 5.6).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.bob_amount += (target_bob - self.bob_amount) * (1.0 - (-10.0 * dt).exp());
        if moving {
            self.bob_time += dt * (5.2 + horizontal_speed * 1.1);
        }
    }

    /// Camera for the current frame, including head bob and step smoothing.
    pub fn camera(&self) -> Camera {
        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let right = Vec3::new(-forward.z, 0.0, forward.x);
        let bob_vertical = (self.bob_time * 2.0).sin() * 0.035 * self.bob_amount;
        let bob_side = (self.bob_time).cos() * 0.018 * self.bob_amount;
        let eye = self.body.position
            + Vec3::new(0.0, EYE_HEIGHT + bob_vertical + self.step_offset, 0.0)
            + right * bob_side;
        Camera {
            position: eye,
            yaw: self.yaw,
            pitch: self.pitch,
            fov_y_degrees: self.fov,
            z_near: 0.05,
            z_far: 180.0,
        }
    }

    /// Forward vector used by interaction checks.
    pub fn look_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
    }

    /// Eye position used by interaction checks.
    pub fn eye_position(&self) -> Vec3 {
        self.body.position + Vec3::Y * EYE_HEIGHT
    }
}

/// Default action bindings for the demo player.
pub fn default_input_map() -> InputMap {
    InputMap::new()
        .bind("move_forward", &[KeyCode::KeyW])
        .bind("move_backward", &[KeyCode::KeyS])
        .bind("move_left", &[KeyCode::KeyA])
        .bind("move_right", &[KeyCode::KeyD])
        .bind("jump", &[KeyCode::Space])
        .bind("crouch", &[KeyCode::ControlLeft])
        .bind("sprint", &[KeyCode::ShiftLeft])
        .bind("interact", &[KeyCode::KeyE])
        .bind("restart", &[KeyCode::KeyR])
        .bind("pause", &[KeyCode::KeyP])
        .bind("fly_toggle", &[KeyCode::KeyF])
        .bind("camera_rot_left", &[KeyCode::ArrowLeft])
        .bind("camera_rot_right", &[KeyCode::ArrowRight])
        .bind("camera_rot_up", &[KeyCode::ArrowUp])
        .bind("camera_rot_down", &[KeyCode::ArrowDown])
}
