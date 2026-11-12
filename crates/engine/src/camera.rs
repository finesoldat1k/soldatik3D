use glam::{Mat4, Vec3};
use std::collections::HashSet;
use winit::keyboard::KeyCode;
use crate::world::Collider;

const PLAYER_HEIGHT: f32 = 1.7;  
const PLAYER_RADIUS: f32 = 0.5;  
const GRAVITY: f32 = 25.0;
const JUMP_FORCE: f32 = 11.0;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub is_flying: bool,
    pub on_ground: bool,
    pub current_fov: f32,
    pub bob_time: f32,
    pub bob_intensity: f32, // <-- НОВОЕ: 0..1, плавно затухает
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, -8.29, 0.0), 
            yaw: -90.0f32.to_radians(),
            pitch: 0.0,
            velocity: Vec3::ZERO,
            is_flying: false,
            on_ground: true,
            current_fov: 45.0,
            bob_time: 0.0,
            bob_intensity: 0.0,
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        let forward = Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        ).normalize();
        
        let mut eye_pos = self.position;
        if self.bob_intensity > 0.001 {
            let bob_y = (self.bob_time * 2.0).sin() * 0.04 * self.bob_intensity;
            let bob_x = (self.bob_time).cos() * 0.02 * self.bob_intensity;
            eye_pos.y += bob_y;
            let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();
            eye_pos += right * bob_x;
        }

        let target = eye_pos + forward;
        Mat4::look_at_rh(eye_pos, target, Vec3::Y)
    }

    pub fn process_mouse(&mut self, dx: f32, dy: f32) {
        let sensitivity = 0.002;
        self.yaw += dx * sensitivity;
        self.pitch -= dy * sensitivity; 
        self.pitch = self.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }

    pub fn update(&mut self, dt: f32, keys: &HashSet<KeyCode>, colliders: &[Collider]) {
        let rot_speed = 2.0 * dt;
        let is_sprinting = keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::ShiftRight);
        let current_horiz_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();

        let max_speed = if self.is_flying {
            if is_sprinting { 24.0 } else { 12.0 }
        } else if self.on_ground {
            if is_sprinting { 12.0 } else { 6.0 }
        } else {
            current_horiz_speed.max(6.0)
        };

        let forward = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize();
        let right = Vec3::new(-forward.z, 0.0, forward.x).normalize();

        let mut wish_dir = Vec3::ZERO;
        if keys.contains(&KeyCode::KeyW) { wish_dir += forward; }
        if keys.contains(&KeyCode::KeyS) { wish_dir -= forward; }
        if keys.contains(&KeyCode::KeyA) { wish_dir -= right; }
        if keys.contains(&KeyCode::KeyD) { wish_dir += right; }
        if wish_dir.length_squared() > 0.0 { wish_dir = wish_dir.normalize(); }

        if self.is_flying {
            let target_vel = wish_dir * max_speed;
            let t = (10.0 * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_vel.x - self.velocity.x) * t;
            self.velocity.z += (target_vel.z - self.velocity.z) * t;

            if keys.contains(&KeyCode::Space) { self.velocity.y += 60.0 * dt; }
            if keys.contains(&KeyCode::ControlLeft) { self.velocity.y -= 60.0 * dt; }
            self.velocity.y *= (1.0 - 5.0 * dt).max(0.0);

            self.apply_movement(dt, colliders, false);
            self.on_ground = false;
        } else {
            let target_vel = wish_dir * max_speed;
            let accel_rate = if self.on_ground { 15.0 } else { 3.0 }; 
            let t = (accel_rate * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_vel.x - self.velocity.x) * t;
            self.velocity.z += (target_vel.z - self.velocity.z) * t;
            self.velocity.y -= GRAVITY * dt;

            if self.on_ground && keys.contains(&KeyCode::Space) {
                self.velocity.y = JUMP_FORCE;
                self.on_ground = false;
            }
            self.apply_movement(dt, colliders, true);
        }

        let horiz_speed = Vec3::new(self.velocity.x, 0.0, self.velocity.z).length();
        
        // НЕЛИНЕЙНАЯ КРИВАЯ FOV
        // При ходьбе (6.0) ratio = 0.5, дельта FOV составит всего ~2.5 градуса (почти незаметно).
        // При беге (12.0) ratio = 1.0, дельта FOV составит 20 градусов (сильный эффект).
        let speed_ratio = (horiz_speed / 12.0).clamp(0.0, 1.0);
        let fov_bonus = (speed_ratio * speed_ratio * speed_ratio) * 20.0; 
        
        let target_fov = 45.0 + fov_bonus; 
        self.current_fov += (target_fov - self.current_fov) * dt * 8.0;

        // ИСПРАВЛЕНИЕ: Плавное затухание headbob
        let target_intensity = if self.on_ground { (horiz_speed / 6.0).min(1.0) } else { 0.0 };
        self.bob_intensity += (target_intensity - self.bob_intensity) * dt * 10.0;

        if self.on_ground && horiz_speed > 0.1 {
            self.bob_time += dt * (horiz_speed * 0.8 + 1.0);
        }

        if keys.contains(&KeyCode::ArrowLeft) { self.yaw -= rot_speed; }
        if keys.contains(&KeyCode::ArrowRight) { self.yaw += rot_speed; }
        if keys.contains(&KeyCode::ArrowUp) { self.pitch += rot_speed; }
        if keys.contains(&KeyCode::ArrowDown) { self.pitch -= rot_speed; }
        self.pitch = self.pitch.clamp(-89.0f32.to_radians(), 89.0f32.to_radians());
    }

    fn apply_movement(&mut self, dt: f32, colliders: &[Collider], apply_gravity: bool) {
        if self.is_flying {
            self.position += self.velocity * dt;
            return;
        }
        let next_x = self.position.x + self.velocity.x * dt;
        if !self.check_collision(Vec3::new(next_x, self.position.y, self.position.z), colliders) {
            self.position.x = next_x;
        } else { self.velocity.x = 0.0; }

        let next_z = self.position.z + self.velocity.z * dt;
        if !self.check_collision(Vec3::new(self.position.x, self.position.y, next_z), colliders) {
            self.position.z = next_z;
        } else { self.velocity.z = 0.0; }

        let next_y = self.position.y + self.velocity.y * dt;
        let mut collided_y = false;
        let prev_y = self.position.y;
        let p_min = Vec3::new(self.position.x - PLAYER_RADIUS, next_y - PLAYER_HEIGHT, self.position.z - PLAYER_RADIUS);
        let p_max = Vec3::new(self.position.x + PLAYER_RADIUS, next_y + 0.1, self.position.z + PLAYER_RADIUS);

        for col in colliders {
            if (p_min.x <= col.max.x && p_max.x >= col.min.x) && (p_min.y <= col.max.y && p_max.y >= col.min.y) && (p_min.z <= col.max.z && p_max.z >= col.min.z) {
                if self.velocity.y <= 0.0 && (prev_y - PLAYER_HEIGHT) >= col.max.y - 0.1 {
                    self.position.y = col.max.y + PLAYER_HEIGHT + 0.001; 
                    self.velocity.y = 0.0;
                    self.on_ground = true;
                    collided_y = true;
                    break;
                } else if self.velocity.y > 0.0 && (prev_y + 0.1) <= col.min.y + 0.1 {
                    self.position.y = col.min.y - 0.1 - 0.001;
                    self.velocity.y = 0.0;
                    self.on_ground = false;
                    collided_y = true;
                    break;
                }
            }
        }

        if !collided_y {
            self.position.y = next_y;
            if apply_gravity {
                if self.velocity.y < 0.0 {
                    let g_min = Vec3::new(self.position.x - PLAYER_RADIUS, self.position.y - PLAYER_HEIGHT - 0.1, self.position.z - PLAYER_RADIUS);
                    let g_max = Vec3::new(self.position.x + PLAYER_RADIUS, self.position.y - PLAYER_HEIGHT, self.position.z + PLAYER_RADIUS);
                    let mut on_ground = false;
                    for col in colliders {
                        if (g_min.x <= col.max.x && g_max.x >= col.min.x) && (g_min.y <= col.max.y && g_max.y >= col.min.y) && (g_min.z <= col.max.z && g_max.z >= col.min.z) {
                            on_ground = true; break;
                        }
                    }
                    self.on_ground = on_ground;
                } else {
                    self.on_ground = false;
                }
            }
        }
    }

    fn check_collision(&self, pos: Vec3, colliders: &[Collider]) -> bool {
        let e = 0.001;
        let p_min = Vec3::new(pos.x - PLAYER_RADIUS + e, pos.y - PLAYER_HEIGHT + e, pos.z - PLAYER_RADIUS + e);
        let p_max = Vec3::new(pos.x + PLAYER_RADIUS - e, pos.y + 0.1 - e, pos.z + PLAYER_RADIUS - e);
        for col in colliders {
            if (p_min.x <= col.max.x && p_max.x >= col.min.x) && (p_min.y <= col.max.y && p_max.y >= col.min.y) && (p_min.z <= col.max.z && p_max.z >= col.min.z) {
                return true;
            }
        }
        false
    }
}
