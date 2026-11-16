//! Procedural sky and CPU-side image-based-lighting textures.
//!
//! The resources are generated once at startup. Diffuse lighting uses a small
//! cosine-convolved irradiance cubemap, while reflections sample a GGX
//! prefiltered cubemap whose mip chain maps roughness to a LOD level. Keeping
//! both maps smooth removes the sharp, screen-following sun squares produced by
//! sampling an unfiltered environment cubemap for every surface.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

/// Resolution of the shared source sky cubemap.
pub const ENVIRONMENT_SIZE: u32 = 128;
/// Resolution of the convolved diffuse irradiance cubemap.
pub const IRRADIANCE_SIZE: u32 = 32;
/// Base resolution of the prefiltered specular cubemap.
pub const PREFILTERED_SIZE: u32 = 128;
/// Number of mip levels in the prefiltered specular cubemap.
///
/// The world shaders hardcode `PREFILTERED_MAX_LOD` to `levels - 1`; change
/// both together.
pub const PREFILTERED_MIP_LEVELS: u32 = 8;

/// Face basis `(right, up, forward)` in the GL/Vulkan cubemap layer order:
/// +X, -X, +Y, -Y, +Z, -Z. The generated texel direction is
/// `right * u - up * v + forward`, matching the usual cubemap conventions.
fn face_bases() -> [(Vec3, Vec3, Vec3); 6] {
    [
        (Vec3::NEG_Z, Vec3::NEG_Y, Vec3::X),
        (Vec3::Z, Vec3::NEG_Y, Vec3::NEG_X),
        (Vec3::X, Vec3::Z, Vec3::Y),
        (Vec3::X, Vec3::NEG_Z, Vec3::NEG_Y),
        (Vec3::X, Vec3::NEG_Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::NEG_Y, Vec3::NEG_Z),
    ]
}

/// Generates the default outdoor sky cubemap.
pub fn generate_environment_cubemap(size: u32) -> Vec<f32> {
    let sun_direction = Vec3::new(0.4, 0.8, 0.5).normalize();
    generate_environment_cubemap_with_sun(size, sun_direction)
}

/// Generates a sky cubemap whose sun highlight follows `sun_direction`.
///
/// Renderers pass the scene's primary light direction so reflections line up
/// with the direct light and its shadows instead of moving independently.
/// The returned buffer contains six RGBA32F faces in GL/Vulkan layer order,
/// each `size * size` texels.
pub fn generate_environment_cubemap_with_sun(size: u32, sun_direction: Vec3) -> Vec<f32> {
    let size = size.max(1);
    let sun_direction = sun_direction.normalize_or_zero();
    let sun_direction = if sun_direction.length_squared() < 0.5 {
        Vec3::Y
    } else {
        sun_direction
    };
    let mut pixels = Vec::with_capacity((size * size * 6 * 4) as usize);

    for (right, up, forward) in face_bases() {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let direction = (right * u - up * v + forward).normalize();

                // A cool zenith fading into a warm horizon, with a darker
                // bounce-light ground below it.
                let horizon = Vec3::new(0.58, 0.68, 0.84);
                let zenith = Vec3::new(0.16, 0.32, 0.64);
                let ground = Vec3::new(0.10, 0.11, 0.13);
                let sky = if direction.y >= 0.0 {
                    horizon.lerp(zenith, direction.y.powf(0.6))
                } else {
                    horizon.lerp(ground, (-direction.y).powf(0.35))
                };

                // A broad glow plus a small, soft solar disk. The disk is wide
                // enough to cover several source texels, which keeps the GGX
                // prefilter stable instead of producing single-texel sparkles.
                let sun_dot = direction.dot(sun_direction).max(0.0);
                let disk = smoothstep(0.9986, 0.99985, sun_dot) * 18.0;
                let glow = sun_dot.powf(24.0) * 0.35;
                let sun = Vec3::new(1.0, 0.86, 0.68) * (disk + glow);
                let color = sky + sun;

                pixels.extend_from_slice(&[color.x, color.y, color.z, 1.0]);
            }
        }
    }

    pixels
}

/// Cosine-convolved diffuse irradiance cubemap.
///
/// The result is a smooth ambient term suitable for
/// `texture(irradiance_map, normal)`. It intentionally has no mip chain: the
/// lowest detail level is already smooth.
pub fn generate_irradiance_cubemap(
    environment: &[f32],
    environment_size: u32,
    size: u32,
) -> Vec<f32> {
    let environment_size = environment_size.max(1);
    let size = size.max(1);
    debug_assert_eq!(environment.len(), expected_len(environment_size));
    let mut pixels = vec![0.0; expected_len(size)];
    const SAMPLES: u32 = 256;

    for (face, (right, up, forward)) in face_bases().into_iter().enumerate() {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let normal = (right * u - up * v + forward).normalize();
                let (tangent, bitangent) = tangent_frame(normal);

                let mut irradiance = Vec3::ZERO;
                for sample in 0..SAMPLES {
                    let xi = hammersley(sample, SAMPLES);
                    let phi = TAU * xi.0;
                    let cos_theta = (1.0 - xi.1).sqrt();
                    let sin_theta = xi.1.sqrt();
                    let direction = (tangent * phi.cos() * sin_theta
                        + bitangent * phi.sin() * sin_theta
                        + normal * cos_theta)
                        .normalize();
                    irradiance += sample_environment(environment, environment_size, direction);
                }
                // Cosine-weighted importance sampling has PDF = cos/π, so the
                // irradiance integral is π times the average radiance.
                let color = irradiance / SAMPLES as f32 * PI;
                let index = ((face as u32 * size + y) * size + x) as usize * 4;
                pixels[index] = color.x;
                pixels[index + 1] = color.y;
                pixels[index + 2] = color.z;
                pixels[index + 3] = 1.0;
            }
        }
    }

    pixels
}

/// GGX-prefiltered specular cubemap with one roughness level per mip.
///
/// Level `i` uses `roughness = i / (mip_levels - 1)` and a face resolution of
/// `base_size >> i` (clamped to at least 1). The first level is a mirror copy
/// of the environment so polished surfaces still reflect the sky accurately.
pub fn generate_prefiltered_cubemap(
    environment: &[f32],
    environment_size: u32,
    base_size: u32,
    mip_levels: u32,
) -> Vec<Vec<f32>> {
    let environment_size = environment_size.max(1);
    let base_size = base_size.max(1);
    let mip_levels = mip_levels.max(1);
    debug_assert_eq!(environment.len(), expected_len(environment_size));

    let mut levels = Vec::with_capacity(mip_levels as usize);
    for mip in 0..mip_levels {
        let size = (base_size >> mip).max(1);
        let roughness = if mip_levels <= 1 {
            0.0
        } else {
            mip as f32 / (mip_levels - 1) as f32
        };
        let samples = if roughness <= 0.001 { 1 } else { 128 };
        let mut pixels = vec![0.0; expected_len(size)];

        for (face, (right, up, forward)) in face_bases().into_iter().enumerate() {
            for y in 0..size {
                for x in 0..size {
                    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let normal = (right * u - up * v + forward).normalize();
                    let color = if samples == 1 {
                        sample_environment(environment, environment_size, normal)
                    } else {
                        prefilter_sample(environment, environment_size, normal, roughness, samples)
                    };

                    let index = ((face as u32 * size + y) * size + x) as usize * 4;
                    pixels[index] = color.x;
                    pixels[index + 1] = color.y;
                    pixels[index + 2] = color.z;
                    pixels[index + 3] = 1.0;
                }
            }
        }
        levels.push(pixels);
    }
    levels
}

fn prefilter_sample(
    environment: &[f32],
    environment_size: u32,
    normal: Vec3,
    roughness: f32,
    samples: u32,
) -> Vec3 {
    let (tangent, bitangent) = tangent_frame(normal);
    let a = roughness * roughness;
    let a2 = a * a;
    let mut prefiltered = Vec3::ZERO;
    let mut total_weight = 0.0;

    for sample in 0..samples {
        let xi = hammersley(sample, samples);
        let phi = TAU * xi.0;
        let cos_theta = ((1.0 - xi.1) / (1.0 + (a2 - 1.0) * xi.1)).sqrt();
        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();
        let halfway = (tangent * phi.cos() * sin_theta
            + bitangent * phi.sin() * sin_theta
            + normal * cos_theta)
            .normalize();
        let light = (normal * (2.0 * normal.dot(halfway)) - halfway).normalize();
        let ndotl = normal.dot(light).max(0.0);
        if ndotl > 0.0 {
            prefiltered += sample_environment(environment, environment_size, light) * ndotl;
            total_weight += ndotl;
        }
    }

    if total_weight > 0.0 {
        prefiltered / total_weight
    } else {
        Vec3::ZERO
    }
}

/// Bilinear lookup of the source environment cubemap.
fn sample_environment(pixels: &[f32], size: u32, direction: Vec3) -> Vec3 {
    let direction = direction.normalize_or_zero();
    let bases = face_bases();
    let mut best_face = 0;
    let mut best_dot = f32::NEG_INFINITY;
    for (index, (_, _, forward)) in bases.iter().enumerate() {
        let dot = direction.dot(*forward);
        if dot > best_dot {
            best_dot = dot;
            best_face = index;
        }
    }

    let (right, up, forward) = bases[best_face];
    let denominator = direction.dot(forward);
    if denominator.abs() < 1e-6 {
        return Vec3::ZERO;
    }
    let u = direction.dot(right) / denominator;
    let v = -direction.dot(up) / denominator;
    let x = (u * 0.5 + 0.5) * size as f32 - 0.5;
    let y = (v * 0.5 + 0.5) * size as f32 - 0.5;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;

    let texel = |x: i32, y: i32| -> Vec3 {
        let x = x.clamp(0, size as i32 - 1) as u32;
        let y = y.clamp(0, size as i32 - 1) as u32;
        let index = ((best_face as u32 * size + y) * size + x) as usize * 4;
        Vec3::new(pixels[index], pixels[index + 1], pixels[index + 2])
    };

    let top = texel(x0, y0).lerp(texel(x0 + 1, y0), tx);
    let bottom = texel(x0, y0 + 1).lerp(texel(x0 + 1, y0 + 1), tx);
    top.lerp(bottom, ty)
}

fn tangent_frame(normal: Vec3) -> (Vec3, Vec3) {
    let up = if normal.y.abs() < 0.999 {
        Vec3::Y
    } else {
        Vec3::Z
    };
    let tangent = up.cross(normal).normalize();
    (tangent, normal.cross(tangent))
}

fn hammersley(index: u32, count: u32) -> (f32, f32) {
    let count = count.max(1);
    (
        (index as f32 + 0.5) / count as f32,
        radical_inverse_vdc(index),
    )
}

fn radical_inverse_vdc(mut bits: u32) -> f32 {
    bits = bits.rotate_right(16);
    bits = ((bits & 0x5555_5555) << 1) | ((bits & 0xAAAA_AAAA) >> 1);
    bits = ((bits & 0x3333_3333) << 2) | ((bits & 0xCCCC_CCCC) >> 2);
    bits = ((bits & 0x0F0F_0F0F) << 4) | ((bits & 0xF0F0_F0F0) >> 4);
    bits = ((bits & 0x00FF_00FF) << 8) | ((bits & 0xFF00_FF00) >> 8);
    bits as f32 * 2.328_306_4e-10
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let t = ((value - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn expected_len(size: u32) -> usize {
    (size as usize) * (size as usize) * 6 * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cubemaps_have_expected_layouts() {
        let environment = generate_environment_cubemap(8);
        assert_eq!(environment.len(), expected_len(8));
        assert!(environment.chunks_exact(4).all(|texel| texel[3] == 1.0));

        let irradiance = generate_irradiance_cubemap(&environment, 8, 4);
        assert_eq!(irradiance.len(), expected_len(4));
        assert!(
            irradiance
                .iter()
                .all(|value| value.is_finite() && *value >= 0.0)
        );

        let levels = generate_prefiltered_cubemap(&environment, 8, 8, 4);
        assert_eq!(levels.len(), 4);
        for (mip, level) in levels.iter().enumerate() {
            assert_eq!(level.len(), expected_len((8u32 >> mip).max(1)));
            assert!(level.iter().all(|value| value.is_finite() && *value >= 0.0));
        }
    }

    #[test]
    fn sun_direction_moves_the_bright_side_of_the_sky() {
        let east = generate_environment_cubemap_with_sun(16, Vec3::X);
        let west = generate_environment_cubemap_with_sun(16, Vec3::NEG_X);
        let east_brightness = sample_environment(&east, 16, Vec3::X);
        let west_brightness = sample_environment(&west, 16, Vec3::X);
        assert!(east_brightness.x > west_brightness.x);
        assert!(east_brightness.y > west_brightness.y);
    }

    #[test]
    fn bilinear_environment_lookup_is_finite() {
        let environment = generate_environment_cubemap(16);
        for direction in [
            Vec3::X,
            Vec3::NEG_X,
            Vec3::Y,
            Vec3::NEG_Y,
            Vec3::Z,
            Vec3::NEG_Z,
            Vec3::new(1.0, 1.0, 1.0),
        ] {
            let color = sample_environment(&environment, 16, direction);
            assert!(color.is_finite());
            assert!(color.max_element() >= 0.0);
        }
    }
}
