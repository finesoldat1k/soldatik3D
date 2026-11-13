use glam::{Mat4, Vec3};

/// A right-handed first-person style camera.
///
/// The camera contains no movement logic: gameplay code owns that. This type
/// only answers "which view and projection matrices does this camera produce".
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    /// Rotation around the Y axis, in radians. 0 looks down -Z.
    pub yaw: f32,
    /// Rotation around the local X axis, in radians.
    pub pitch: f32,
    pub fov_y_degrees: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,
            fov_y_degrees: 60.0,
            z_near: 0.1,
            z_far: 100.0,
        }
    }
}

impl Camera {
    /// A camera at `position` looking down −Z with the default lens.
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    pub fn with_yaw_degrees(mut self, degrees: f32) -> Self {
        self.yaw = degrees.to_radians();
        self
    }

    pub fn with_pitch_degrees(mut self, degrees: f32) -> Self {
        self.pitch = degrees.to_radians();
        self
    }

    pub fn with_fov_degrees(mut self, degrees: f32) -> Self {
        self.fov_y_degrees = degrees;
        self
    }

    pub fn with_range(mut self, near: f32, far: f32) -> Self {
        self.z_near = near;
        self.z_far = far;
        self
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn forward_flat(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin()).normalize()
    }

    pub fn right_flat(&self) -> Vec3 {
        let forward = self.forward_flat();
        Vec3::new(-forward.z, 0.0, forward.x)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(
            self.fov_y_degrees.max(1.0).to_radians(),
            aspect.max(0.01),
            self.z_near.max(0.001),
            self.z_far.max(self.z_near + 0.001),
        )
    }
}
