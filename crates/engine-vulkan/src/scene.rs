use anyhow::Result;
use engine_core::{Aabb, Light, PrimaryLight, Scene};

use crate::buffer::GpuBuffer;
use crate::commands::ImmediateCommands;
use crate::context::VulkanContext;
use crate::image::GpuImage;

pub struct GpuMesh {
    pub buffer: GpuBuffer,
    pub vertex_count: u32,
    pub bounds: Aabb,
}

/// Immutable GPU representation of a scene.
pub struct GpuScene {
    pub meshes: Vec<GpuMesh>,
    pub albedo: GpuImage,
    pub lights: Vec<Light>,
    pub primary_light: PrimaryLight,
    pub metallic: f32,
    pub roughness: f32,
    pub clear_color: [f32; 4],
    pub total_vertices: u64,
}

impl GpuScene {
    pub fn upload(
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        scene: &Scene,
    ) -> Result<Self> {
        let mut meshes = Vec::with_capacity(scene.meshes.len());
        let mut total_vertices = 0u64;
        for mesh in &scene.meshes {
            if mesh.vertices.is_empty() {
                continue;
            }
            let buffer = GpuBuffer::vertex(
                &context.device,
                &context.instance,
                context.physical_device,
                uploader,
                &mesh.vertices,
            )?;
            total_vertices += mesh.vertices.len() as u64;
            meshes.push(GpuMesh {
                buffer,
                vertex_count: mesh.vertices.len() as u32,
                bounds: mesh.bounds,
            });
        }

        let material = &scene.material;
        let albedo = GpuImage::texture_2d(
            context,
            uploader,
            material.albedo.width,
            material.albedo.height,
            &material.albedo.pixels,
            false,
            true,
        )?;

        Ok(Self {
            meshes,
            albedo,
            lights: scene.lights.clone(),
            primary_light: scene.primary_light,
            metallic: material.metallic.clamp(0.0, 1.0),
            roughness: material.roughness.clamp(0.04, 1.0),
            clear_color: scene.clear_color,
            total_vertices,
        })
    }
}
