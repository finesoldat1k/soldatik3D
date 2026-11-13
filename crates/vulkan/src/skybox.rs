use glam::Vec3;

/// Procedural sky used as a stand-in environment for image-based lighting.
/// The engine ships this until a scene/asset-provided environment map exists.
pub fn generate_cubemap_pixels(size: u32) -> Vec<f32> {
    let mut pixels = Vec::with_capacity((size * size * 6 * 4) as usize);
    let sun_dir = Vec3::new(0.4, 0.8, 0.5).normalize();

    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
        ),
        (
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
        ),
        (
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        ),
        (
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
            Vec3::new(0.0, -1.0, 0.0),
        ),
        (
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ),
        (
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        ),
    ];

    for (right, up, forward) in faces {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let direction = (right * u - up * v + forward).normalize();

                let sky = if direction.y > 0.0 {
                    let t = direction.y;
                    Vec3::new(0.35, 0.55, 0.85) * t + Vec3::new(0.75, 0.65, 0.50) * (1.0 - t)
                } else {
                    Vec3::new(0.08, 0.07, 0.06)
                };

                let sun_dot = direction.dot(sun_dir).max(0.0);
                let sun_disk = sun_dot.powf(2048.0) * 50.0;
                let sun_glow = sun_dot.powf(16.0) * 3.0;
                let color = sky + Vec3::splat(sun_disk + sun_glow);

                pixels.extend_from_slice(&[color.x, color.y, color.z, 1.0]);
            }
        }
    }

    pixels
}
