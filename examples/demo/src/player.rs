//! First-person player controller for the demo.

use runtime::glam::Vec3;
use runtime::winit::keyboard::KeyCode;
use runtime::{Aabb, Camera, Input, InputMap};

const PLAYER_HEIGHT: f32 = 1.7;
const PLAYER_RADIUS: f32 = 0.5;
const GRAVITY: f32 = 25.0;
const JUMP_FORCE: f32 = 11.0;
const MOUSE_SENSITIVITY: f32 = 0.002;

pub struct Player {
    camera: Camera,
    velocity: Vec3,
    is_flying: bool,
    on_ground: bool,
    current_fov: f32,
    bob_time: f32,
    bob_intensity: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

impl Player {
    pub fn new() -> Self {
        let camera = Camera {
            position: Vec3::new(0.0, -8.29, 0.0),
            yaw: 90.0_f32.to_radians(),
            pitch: 0.0,
            fov_y_degrees: 45.0,
            z_near: 0.1,
            z_far: 100.0,
        };

        Self {
            camera,
            velocity: Vec3::ZERO,
            is_flying: false,
            on_ground: true,
            current_fov: 45.0,
            bob_time: 0.0,
            bob_intensity: 0.0,
        }
    }

    pub fn camera(&self) -> Camera {
        self.render_camera()
    }

    pub fn is_flying(&self) -> bool {
        self.is_flying
    }

    pub fn set_flying(&mut self, flying: bool) {
        self.is_flying = flying;
        self.velocity = Vec3::ZERO;
        if flying {
            self.on_ground = false;
        }
    }

    /// Moves the player to a new position. Yaw and pitch are in degrees.
    pub fn teleport(
        &mut self,
        position: Vec3,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    ) {
        self.camera.position = position;
        if let Some(yaw) = yaw_degrees {
            self.camera.yaw = yaw.to_radians();
        }
        if let Some(pitch) = pitch_degrees {
            self.camera.pitch = pitch.to_radians();
        }
        self.velocity = Vec3::ZERO;
        self.on_ground = false;
    }

    /// Returns to the initial spawn position and resets flight mode.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Stops horizontal movement, used when the cursor is released for the
    /// debug menu.
    pub fn stop(&mut self) {
        self.velocity.x = 0.0;
        self.velocity.z = 0.0;
        self.bob_intensity = 0.0;
    }

    /// Adds to the camera yaw (for touch camera control).
    pub fn add_yaw(&mut self, delta: f32) {
        self.camera.yaw += delta;
    }

    /// Adds to the camera pitch (for touch camera control).
    pub fn add_pitch(&mut self, delta: f32) {
        self.camera.pitch =
            (self.camera.pitch + delta).clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
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
        .bind("fly_toggle", &[KeyCode::KeyF])
        .bind("camera_rot_left", &[KeyCode::ArrowLeft])
        .bind("camera_rot_right", &[KeyCode::ArrowRight])
        .bind("camera_rot_up", &[KeyCode::ArrowUp])
        .bind("camera_rot_down", &[KeyCode::ArrowDown])
}

impl Player {
    /// Updates the player with desktop input (mouse + keyboard).
    pub fn update(&mut self, dt: f32, input: &Input, colliders: &[Aabb]) {
        self.update_with_virtual(dt, input, colliders, None);
    }

    /// Updates the player with desktop input and optional virtual controls for mobile.
    pub fn update_with_virtual(
        &mut self,
        dt: f32,
        input: &Input,
        colliders: &[Aabb],
        virtual_controls: Option<&crate::virtual_joystick::VirtualControls>,
    ) {
        // Handle fly toggle from both keyboard and virtual button
        let fly_toggle = input.action_pressed("fly_toggle")
            || virtual_controls.is_some_and(|vc| vc.fly_pressed());
        if fly_toggle {
            self.is_flying = !self.is_flying;
            self.velocity = Vec3::ZERO;
        }

        // Handle mouse look (desktop) or touch look (mobile)
        let mouse = input.mouse_delta();
        if mouse.length_squared() > 0.0 {
            self.camera.yaw += mouse.x * MOUSE_SENSITIVITY;
            self.camera.pitch -= mouse.y * MOUSE_SENSITIVITY;
            self.camera.pitch = self
                .camera
                .pitch
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }

        // Keyboard rotation (for desktop without mouse capture)
        let rotation_speed = 2.0 * dt;
        if input.action_down("camera_rot_left") {
            self.camera.yaw -= rotation_speed;
        }
        if input.action_down("camera_rot_right") {
            self.camera.yaw += rotation_speed;
        }
        if input.action_down("camera_rot_up") {
            self.camera.pitch += rotation_speed;
        }
        if input.action_down("camera_rot_down") {
            self.camera.pitch -= rotation_speed;
        }
        self.camera.pitch = self
            .camera
            .pitch
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

        // Determine movement input from keyboard or virtual joystick
        let move_forward = input.action_down("move_forward")
            || virtual_controls.is_some_and(|vc| vc.movement().y < -0.1);
        let move_backward = input.action_down("move_backward")
            || virtual_controls.is_some_and(|vc| vc.movement().y > 0.1);
        let move_left = input.action_down("move_left")
            || virtual_controls.is_some_and(|vc| vc.movement().x < -0.1);
        let move_right = input.action_down("move_right")
            || virtual_controls.is_some_and(|vc| vc.movement().x > 0.1);

        let sprinting = input.action_down("sprint");
        let horizontal_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();

        let max_speed = if self.is_flying {
            if sprinting { 24.0 } else { 12.0 }
        } else if self.on_ground {
            if sprinting { 12.0 } else { 6.0 }
        } else {
            horizontal_speed.max(6.0)
        };

        let forward = Vec3::new(self.camera.yaw.cos(), 0.0, self.camera.yaw.sin()).normalize();
        let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();

        let mut wish_direction = Vec3::ZERO;
        if move_forward {
            wish_direction += forward;
        }
        if move_backward {
            wish_direction -= forward;
        }
        if move_left {
            wish_direction -= right;
        }
        if move_right {
            wish_direction += right;
        }
        if wish_direction.length_squared() > 0.0 {
            wish_direction = wish_direction.normalize();
        }

        if self.is_flying {
            let target_velocity = wish_direction * max_speed;
            let blend = (10.0 * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_velocity.x - self.velocity.x) * blend;
            self.velocity.z += (target_velocity.z - self.velocity.z) * blend;

            let jump_down =
                input.action_down("jump") || virtual_controls.is_some_and(|vc| vc.jump_pressed());
            if jump_down {
                self.velocity.y += 60.0 * dt;
            }
            if input.action_down("crouch") {
                self.velocity.y -= 60.0 * dt;
            }
            self.velocity.y *= (1.0 - 5.0 * dt).max(0.0);

            self.apply_movement(dt, colliders, false);
            self.on_ground = false;
        } else {
            let target_velocity = wish_direction * max_speed;
            let acceleration = if self.on_ground { 15.0 } else { 3.0 };
            let blend = (acceleration * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_velocity.x - self.velocity.x) * blend;
            self.velocity.z += (target_velocity.z - self.velocity.z) * blend;
            self.velocity.y -= GRAVITY * dt;

            let jump_pressed =
                input.action_down("jump") || virtual_controls.is_some_and(|vc| vc.jump_pressed());
            if self.on_ground && jump_pressed {
                self.velocity.y = JUMP_FORCE;
                self.on_ground = false;
            }
            self.apply_movement(dt, colliders, true);
        }

        let horizontal_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
        let speed_ratio = (horizontal_speed / 12.0).clamp(0.0, 1.0);
        let target_fov = 45.0 + speed_ratio.powi(3) * 20.0;
        self.current_fov += (target_fov - self.current_fov) * dt * 8.0;
        self.camera.fov_y_degrees = self.current_fov;

        let target_bob = if self.on_ground {
            (horizontal_speed / 6.0).min(1.0)
        } else {
            0.0
        };
        self.bob_intensity += (target_bob - self.bob_intensity) * dt * 10.0;
        if self.on_ground && horizontal_speed > 0.1 {
            self.bob_time += dt * (horizontal_speed * 0.8 + 1.0);
        }
    }

    /// Updates the player with mobile virtual input (joystick vector, jump, fly toggle).
    pub fn update_with_virtual_mobile(
        &mut self,
        dt: f32,
        input: &Input,
        colliders: &[Aabb],
        move_input: Vec3,
        jump_pressed: bool,
        fly_toggle: bool,
    ) {
        if fly_toggle {
            self.is_flying = !self.is_flying;
            self.velocity = Vec3::ZERO;
        }

        // Handle mouse look (desktop)
        let mouse = input.mouse_delta();
        if mouse.length_squared() > 0.0 {
            self.camera.yaw += mouse.x * MOUSE_SENSITIVITY;
            self.camera.pitch -= mouse.y * MOUSE_SENSITIVITY;
            self.camera.pitch = self
                .camera
                .pitch
                .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());
        }

        // Keyboard rotation (for desktop without mouse capture)
        let rotation_speed = 2.0 * dt;
        if input.action_down("camera_rot_left") {
            self.camera.yaw -= rotation_speed;
        }
        if input.action_down("camera_rot_right") {
            self.camera.yaw += rotation_speed;
        }
        if input.action_down("camera_rot_up") {
            self.camera.pitch += rotation_speed;
        }
        if input.action_down("camera_rot_down") {
            self.camera.pitch -= rotation_speed;
        }
        self.camera.pitch = self
            .camera
            .pitch
            .clamp(-89.0_f32.to_radians(), 89.0_f32.to_radians());

        // Determine movement input from keyboard or virtual joystick
        let move_forward = input.action_down("move_forward") || move_input.z < -0.1;
        let move_backward = input.action_down("move_backward") || move_input.z > 0.1;
        let move_left = input.action_down("move_left") || move_input.x < -0.1;
        let move_right = input.action_down("move_right") || move_input.x > 0.1;

        let sprinting = input.action_down("sprint");
        let horizontal_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();

        let max_speed = if self.is_flying {
            if sprinting { 24.0 } else { 12.0 }
        } else if self.on_ground {
            if sprinting { 12.0 } else { 6.0 }
        } else {
            horizontal_speed.max(6.0)
        };

        let forward = Vec3::new(self.camera.yaw.cos(), 0.0, self.camera.yaw.sin()).normalize();
        let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();

        let mut wish_direction = Vec3::ZERO;
        if move_forward {
            wish_direction += forward;
        }
        if move_backward {
            wish_direction -= forward;
        }
        if move_left {
            wish_direction -= right;
        }
        if move_right {
            wish_direction += right;
        }
        if wish_direction.length_squared() > 0.0 {
            wish_direction = wish_direction.normalize();
        }

        if self.is_flying {
            let target_velocity = wish_direction * max_speed;
            let blend = (10.0 * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_velocity.x - self.velocity.x) * blend;
            self.velocity.z += (target_velocity.z - self.velocity.z) * blend;

            if jump_pressed {
                self.velocity.y += 60.0 * dt;
            }
            if input.action_down("crouch") {
                self.velocity.y -= 60.0 * dt;
            }
            self.velocity.y *= (1.0 - 5.0 * dt).max(0.0);

            self.apply_movement(dt, colliders, false);
            self.on_ground = false;
        } else {
            let target_velocity = wish_direction * max_speed;
            let acceleration = if self.on_ground { 15.0 } else { 3.0 };
            let blend = (acceleration * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_velocity.x - self.velocity.x) * blend;
            self.velocity.z += (target_velocity.z - self.velocity.z) * blend;
            self.velocity.y -= crate::player::GRAVITY * dt;

            if self.on_ground && jump_pressed {
                self.velocity.y = crate::player::JUMP_FORCE;
                self.on_ground = false;
            }
            self.apply_movement(dt, colliders, true);
        }

        let horizontal_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
        let speed_ratio = (horizontal_speed / 12.0).clamp(0.0, 1.0);
        let target_fov = 45.0 + speed_ratio.powi(3) * 20.0;
        self.current_fov += (target_fov - self.current_fov) * dt * 8.0;
        self.camera.fov_y_degrees = self.current_fov;

        let target_bob = if self.on_ground {
            (horizontal_speed / 6.0).min(1.0)
        } else {
            0.0
        };
        self.bob_intensity += (target_bob - self.bob_intensity) * dt * 10.0;
        if self.on_ground && horizontal_speed > 0.1 {
            self.bob_time += dt * (horizontal_speed * 0.8 + 1.0);
        }
    }

    fn render_camera(&self) -> Camera {
        let mut camera = self.camera;
        if self.bob_intensity > 0.001 {
            let forward = camera.forward();
            let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();
            camera.position.y += (self.bob_time * 2.0).sin() * 0.04 * self.bob_intensity;
            camera.position += right * (self.bob_time.cos() * 0.02 * self.bob_intensity);
        }
        camera
    }

    fn apply_movement(&mut self, dt: f32, colliders: &[Aabb], apply_gravity: bool) {
        if self.is_flying {
            self.camera.position += self.velocity * dt;
            return;
        }

        let next_x = self.camera.position.x + self.velocity.x * dt;
        if !self.collides(
            Vec3::new(next_x, self.camera.position.y, self.camera.position.z),
            colliders,
        ) {
            self.camera.position.x = next_x;
        } else {
            self.velocity.x = 0.0;
        }

        let next_z = self.camera.position.z + self.velocity.z * dt;
        if !self.collides(
            Vec3::new(self.camera.position.x, self.camera.position.y, next_z),
            colliders,
        ) {
            self.camera.position.z = next_z;
        } else {
            self.velocity.z = 0.0;
        }

        let next_y = self.camera.position.y + self.velocity.y * dt;
        let previous_y = self.camera.position.y;
        let player_min = Vec3::new(
            self.camera.position.x - PLAYER_RADIUS,
            next_y - PLAYER_HEIGHT,
            self.camera.position.z - PLAYER_RADIUS,
        );
        let player_max = Vec3::new(
            self.camera.position.x + PLAYER_RADIUS,
            next_y + 0.1,
            self.camera.position.z + PLAYER_RADIUS,
        );

        let mut collided_y = false;
        for collider in colliders {
            if !overlaps(player_min, player_max, collider) {
                continue;
            }
            if self.velocity.y <= 0.0 && previous_y - PLAYER_HEIGHT >= collider.max.y - 0.1 {
                self.camera.position.y = collider.max.y + PLAYER_HEIGHT + 0.001;
                self.velocity.y = 0.0;
                self.on_ground = true;
                collided_y = true;
                break;
            }
            if self.velocity.y > 0.0 && previous_y + 0.1 <= collider.min.y + 0.1 {
                self.camera.position.y = collider.min.y - 0.1 - 0.001;
                self.velocity.y = 0.0;
                self.on_ground = false;
                collided_y = true;
                break;
            }
        }

        if collided_y {
            return;
        }

        self.camera.position.y = next_y;
        if !apply_gravity {
            return;
        }

        if self.velocity.y < 0.0 {
            let ground_min = Vec3::new(
                self.camera.position.x - PLAYER_RADIUS,
                self.camera.position.y - PLAYER_HEIGHT - 0.1,
                self.camera.position.z - PLAYER_RADIUS,
            );
            let ground_max = Vec3::new(
                self.camera.position.x + PLAYER_RADIUS,
                self.camera.position.y - PLAYER_HEIGHT,
                self.camera.position.z + PLAYER_RADIUS,
            );
            self.on_ground = colliders
                .iter()
                .any(|collider| overlaps(ground_min, ground_max, collider));
        } else {
            self.on_ground = false;
        }
    }

    fn collides(&self, position: Vec3, colliders: &[Aabb]) -> bool {
        let epsilon = 0.001;
        let player_min = Vec3::new(
            position.x - PLAYER_RADIUS + epsilon,
            position.y - PLAYER_HEIGHT + epsilon,
            position.z - PLAYER_RADIUS + epsilon,
        );
        let player_max = Vec3::new(
            position.x + PLAYER_RADIUS - epsilon,
            position.y + 0.1 - epsilon,
            position.z + PLAYER_RADIUS - epsilon,
        );
        colliders
            .iter()
            .any(|collider| overlaps(player_min, player_max, collider))
    }
}

fn overlaps(min: Vec3, max: Vec3, collider: &Aabb) -> bool {
    min.x <= collider.max.x
        && max.x >= collider.min.x
        && min.y <= collider.max.y
        && max.y >= collider.min.y
        && min.z <= collider.max.z
        && max.z >= collider.min.z
}
