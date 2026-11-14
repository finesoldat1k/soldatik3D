use anyhow::{Context, Result};
use ash::{Device, vk};
use assets::{font_atlas_rgba8, generate_environment_cubemap};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec4};
use render::{BackendKind, FrameData, RenderStats, Renderer, VsyncMode, WindowSurface};
use scene::{Aabb, LightUniformBuffer, Scene, Vertex};
use ui::UiVertex;

use crate::buffer::GpuBuffer;
use crate::commands::{CommandPool, ImmediateCommands};
use crate::context::VulkanContext;
use crate::descriptor::{self, DescriptorPool, DescriptorSetLayout};
use crate::image::GpuImage;
use crate::pipeline::{self, Pipeline};
use crate::render_pass::{self, RenderPass};
use crate::scene::GpuScene;
use crate::swapchain::Swapchain;
use crate::sync::SyncObjects;

const MAX_FRAMES_IN_FLIGHT: usize = 1;
const HDR_FORMAT: vk::Format = vk::Format::R16G16B16A16_SFLOAT;
const SHADOW_MAP_SIZE: u32 = 2048;
const INITIAL_UI_CAPACITY: usize = 16_384;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WorldPushConstants {
    mvp: [f32; 16],
    light_view_projection: [f32; 16],
    light_position: [f32; 4],
    light_color: [f32; 4],
    camera_position: [f32; 4],
    screen_and_debug: [f32; 4],
    material: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SsaoPushConstants {
    projection: [f32; 16],
    inverse_projection: [f32; 16],
}

/// All swapchain-sized attachments. The struct exists so resize can replace
/// the whole set atomically and destroy framebuffers before their attachments.
struct SwapchainFrame {
    device: Device,
    swapchain: Swapchain,
    depth: GpuImage,
    hdr: GpuImage,
    hdr_framebuffer: vk::Framebuffer,
    bloom_extent: vk::Extent2D,
    bloom_a: GpuImage,
    bloom_b: GpuImage,
    bloom_a_framebuffer: vk::Framebuffer,
    bloom_b_framebuffer: vk::Framebuffer,
    ssao_a: GpuImage,
    ssao_b: GpuImage,
    ssao_a_framebuffer: vk::Framebuffer,
    ssao_b_framebuffer: vk::Framebuffer,
    swapchain_framebuffers: Vec<vk::Framebuffer>,
}

impl SwapchainFrame {
    #[allow(clippy::too_many_arguments)]
    fn new(
        context: &VulkanContext,
        size: (u32, u32),
        format: vk::Format,
        hdr_pass: vk::RenderPass,
        bloom_pass: vk::RenderPass,
        ssao_pass: vk::RenderPass,
        swapchain_pass: vk::RenderPass,
        old_swapchain: vk::SwapchainKHR,
        vsync: VsyncMode,
    ) -> Result<Self> {
        let swapchain = Swapchain::new(context, size, Some(format), old_swapchain, vsync)?;
        let extent = swapchain.extent;
        let depth = GpuImage::depth_target(context, extent.width, extent.height, false)?;
        let hdr = GpuImage::color_target(context, extent.width, extent.height, HDR_FORMAT)?;
        let hdr_framebuffer =
            render_pass::framebuffer(&context.device, hdr_pass, &[hdr.view, depth.view], extent)?;

        let bloom_extent = vk::Extent2D {
            width: (extent.width / 2).max(1),
            height: (extent.height / 2).max(1),
        };
        let bloom_a =
            GpuImage::color_target(context, bloom_extent.width, bloom_extent.height, HDR_FORMAT)?;
        let bloom_b =
            GpuImage::color_target(context, bloom_extent.width, bloom_extent.height, HDR_FORMAT)?;
        let bloom_a_framebuffer =
            render_pass::framebuffer(&context.device, bloom_pass, &[bloom_a.view], bloom_extent)?;
        let bloom_b_framebuffer =
            render_pass::framebuffer(&context.device, bloom_pass, &[bloom_b.view], bloom_extent)?;

        let ssao_a = GpuImage::color_target(
            context,
            bloom_extent.width,
            bloom_extent.height,
            vk::Format::R8_UNORM,
        )?;
        let ssao_b = GpuImage::color_target(
            context,
            bloom_extent.width,
            bloom_extent.height,
            vk::Format::R8_UNORM,
        )?;
        let ssao_a_framebuffer =
            render_pass::framebuffer(&context.device, ssao_pass, &[ssao_a.view], bloom_extent)?;
        let ssao_b_framebuffer =
            render_pass::framebuffer(&context.device, ssao_pass, &[ssao_b.view], bloom_extent)?;

        let mut swapchain_framebuffers = Vec::with_capacity(swapchain.views.len());
        for &view in &swapchain.views {
            match render_pass::framebuffer(
                &context.device,
                swapchain_pass,
                &[view, depth.view],
                extent,
            ) {
                Ok(framebuffer) => swapchain_framebuffers.push(framebuffer),
                Err(error) => {
                    unsafe {
                        for framebuffer in swapchain_framebuffers.drain(..) {
                            context.device.destroy_framebuffer(framebuffer, None);
                        }
                    }
                    return Err(error);
                }
            }
        }

        Ok(Self {
            device: context.device.clone(),
            swapchain,
            depth,
            hdr,
            hdr_framebuffer,
            bloom_extent,
            bloom_a,
            bloom_b,
            bloom_a_framebuffer,
            bloom_b_framebuffer,
            ssao_a,
            ssao_b,
            ssao_a_framebuffer,
            ssao_b_framebuffer,
            swapchain_framebuffers,
        })
    }
}

impl Drop for SwapchainFrame {
    fn drop(&mut self) {
        unsafe {
            for &framebuffer in &self.swapchain_framebuffers {
                self.device.destroy_framebuffer(framebuffer, None);
            }
            self.device.destroy_framebuffer(self.hdr_framebuffer, None);
            self.device
                .destroy_framebuffer(self.bloom_a_framebuffer, None);
            self.device
                .destroy_framebuffer(self.bloom_b_framebuffer, None);
            self.device
                .destroy_framebuffer(self.ssao_a_framebuffer, None);
            self.device
                .destroy_framebuffer(self.ssao_b_framebuffer, None);
        }
    }
}

struct ShadowTarget {
    device: Device,
    image: GpuImage,
    framebuffer: vk::Framebuffer,
}

impl ShadowTarget {
    fn new(context: &VulkanContext, render_pass: vk::RenderPass) -> Result<Self> {
        let image = GpuImage::depth_target(context, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE, true)?;
        let framebuffer = render_pass::framebuffer(
            &context.device,
            render_pass,
            &[image.view],
            vk::Extent2D {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
            },
        )?;
        Ok(Self {
            device: context.device.clone(),
            image,
            framebuffer,
        })
    }
}

impl Drop for ShadowTarget {
    fn drop(&mut self) {
        unsafe { self.device.destroy_framebuffer(self.framebuffer, None) };
    }
}

/// Vulkan implementation of the engine renderer.
///
/// Some fields are only kept alive for resource lifetime ordering (descriptor
/// pools, layouts, immutable images and the command pool) and are never read
/// after initialization.
#[allow(dead_code)]
pub struct VulkanRenderer {
    // Descriptor pools must die before the resources they reference.
    world_pool: DescriptorPool,
    bright_pool: DescriptorPool,
    blur_pool: DescriptorPool,
    ssao_pool: DescriptorPool,
    ssao_blur_pool: DescriptorPool,
    post_pool: DescriptorPool,
    ui_pool: DescriptorPool,

    world_set: vk::DescriptorSet,
    bright_set: vk::DescriptorSet,
    blur_sets: [vk::DescriptorSet; 2],
    ssao_set: vk::DescriptorSet,
    ssao_blur_sets: [vk::DescriptorSet; 2],
    post_set: vk::DescriptorSet,
    ui_set: vk::DescriptorSet,

    shadow_target: ShadowTarget,
    frame: SwapchainFrame,

    world_pipeline: Pipeline,
    shadow_pipeline: Pipeline,
    post_pipeline: Pipeline,
    ui_pipeline: Pipeline,
    bright_pipeline: Pipeline,
    blur_pipeline: Pipeline,
    ssao_pipeline: Pipeline,
    ssao_blur_pipeline: Pipeline,

    hdr_render_pass: RenderPass,
    bloom_render_pass: RenderPass,
    ssao_render_pass: RenderPass,
    swapchain_render_pass: RenderPass,
    shadow_render_pass: RenderPass,

    world_layout: DescriptorSetLayout,
    single_texture_layout: DescriptorSetLayout,
    composite_layout: DescriptorSetLayout,

    scene: GpuScene,
    ibl_irradiance: GpuImage,
    ibl_prefiltered: GpuImage,
    font: GpuImage,
    light_buffer: GpuBuffer,
    ui_buffer: GpuBuffer,
    ui_capacity: usize,
    ui_vertex_count: u32,

    sync: SyncObjects,
    command_pool: CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    shadow_command_buffers: Vec<vk::CommandBuffer>,
    current_frame: usize,

    stats: RenderStats,
    size: (u32, u32),
    context: VulkanContext,
    vsync: VsyncMode,
    capture_requested: bool,
}

impl VulkanRenderer {
    pub fn new(surface: WindowSurface, scene: &Scene, vsync: VsyncMode) -> Result<Self> {
        let size = surface.size();
        let context = VulkanContext::new(surface)?;
        let uploader = ImmediateCommands::new(
            &context.device,
            context.graphics_family,
            context.graphics_queue,
        )?;

        let surface_format = choose_initial_format(&context)?;
        let hdr_render_pass = render_pass::hdr(&context.device, HDR_FORMAT)?;
        let bloom_render_pass = render_pass::offscreen(&context.device, HDR_FORMAT)?;
        let ssao_render_pass = render_pass::offscreen(&context.device, vk::Format::R8_UNORM)?;
        let swapchain_render_pass = render_pass::swapchain(&context.device, surface_format)?;
        let shadow_render_pass = render_pass::shadow(&context.device)?;

        let shadow_target = ShadowTarget::new(&context, shadow_render_pass.handle)?;
        let frame = SwapchainFrame::new(
            &context,
            size,
            surface_format,
            hdr_render_pass.handle,
            bloom_render_pass.handle,
            ssao_render_pass.handle,
            swapchain_render_pass.handle,
            vk::SwapchainKHR::null(),
            vsync,
        )?;

        let scene_gpu = GpuScene::upload(&context, &uploader, scene)?;

        let world_layout = DescriptorSetLayout::world(&context.device)?;
        let single_texture_layout = DescriptorSetLayout::single_texture(&context.device)?;
        let composite_layout = DescriptorSetLayout::composite(&context.device)?;

        let world_pool = DescriptorPool::new(&context.device, 1, 4, 1)?;
        let bright_pool = DescriptorPool::new(&context.device, 1, 1, 0)?;
        let blur_pool = DescriptorPool::new(&context.device, 2, 2, 0)?;
        let ssao_pool = DescriptorPool::new(&context.device, 1, 1, 0)?;
        let ssao_blur_pool = DescriptorPool::new(&context.device, 2, 2, 0)?;
        let post_pool = DescriptorPool::new(&context.device, 1, 3, 0)?;
        let ui_pool = DescriptorPool::new(&context.device, 1, 1, 0)?;

        let world_set = world_pool.allocate(&context.device, world_layout.handle, 1)?[0];
        let bright_set = bright_pool.allocate(&context.device, single_texture_layout.handle, 1)?[0];
        let blur_sets_vec = blur_pool.allocate(&context.device, single_texture_layout.handle, 2)?;
        let blur_sets = [blur_sets_vec[0], blur_sets_vec[1]];
        let ssao_set = ssao_pool.allocate(&context.device, single_texture_layout.handle, 1)?[0];
        let ssao_blur_sets_vec =
            ssao_blur_pool.allocate(&context.device, single_texture_layout.handle, 2)?;
        let ssao_blur_sets = [ssao_blur_sets_vec[0], ssao_blur_sets_vec[1]];
        let post_set = post_pool.allocate(&context.device, composite_layout.handle, 1)?[0];
        let ui_set = ui_pool.allocate(&context.device, single_texture_layout.handle, 1)?[0];

        let light_buffer = GpuBuffer::host_visible(
            &context.device,
            &context.instance,
            context.physical_device,
            std::mem::size_of::<LightUniformBuffer>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;

        let irradiance_pixels = generate_environment_cubemap(128);
        let ibl_irradiance =
            GpuImage::cubemap_rgba32f(&context, &uploader, 128, &irradiance_pixels)?;
        let ibl_prefiltered =
            GpuImage::cubemap_rgba32f(&context, &uploader, 128, &irradiance_pixels)?;

        let font_pixels = font_atlas_rgba8();
        let font_size = (font_pixels.len() / 4).isqrt() as u32;
        let font = GpuImage::texture_2d(
            &context,
            &uploader,
            font_size,
            font_size,
            &font_pixels,
            true,
            false,
        )?;

        let ui_capacity = INITIAL_UI_CAPACITY;
        let ui_buffer = GpuBuffer::host_visible(
            &context.device,
            &context.instance,
            context.physical_device,
            (std::mem::size_of::<UiVertex>() * ui_capacity) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;

        descriptor::update_world(
            &context.device,
            world_set,
            (scene_gpu.albedo.sampler, scene_gpu.albedo.view),
            (shadow_target.image.sampler, shadow_target.image.view),
            (ibl_irradiance.sampler, ibl_irradiance.view),
            (ibl_prefiltered.sampler, ibl_prefiltered.view),
            light_buffer.handle,
            std::mem::size_of::<LightUniformBuffer>() as u64,
        );
        descriptor::update_single(
            &context.device,
            bright_set,
            frame.hdr.sampler,
            frame.hdr.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &context.device,
            blur_sets[0],
            frame.bloom_a.sampler,
            frame.bloom_a.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &context.device,
            blur_sets[1],
            frame.bloom_b.sampler,
            frame.bloom_b.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &context.device,
            ssao_set,
            frame.depth.sampler,
            frame.depth.view,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &context.device,
            ssao_blur_sets[0],
            frame.ssao_a.sampler,
            frame.ssao_a.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &context.device,
            ssao_blur_sets[1],
            frame.ssao_b.sampler,
            frame.ssao_b.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_composite(
            &context.device,
            post_set,
            (frame.hdr.sampler, frame.hdr.view),
            (frame.bloom_a.sampler, frame.bloom_a.view),
            (frame.ssao_a.sampler, frame.ssao_a.view),
        );
        descriptor::update_single(
            &context.device,
            ui_set,
            font.sampler,
            font.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        let world_pipeline = make_pipeline(
            &context.device,
            "world",
            include_str!("../shaders/shader.vert"),
            include_str!("../shaders/shader.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::world(
                    &context.device,
                    hdr_render_pass.handle,
                    world_layout.handle,
                    vertex,
                    fragment,
                )
            },
        )?;
        let shadow_pipeline = make_pipeline(
            &context.device,
            "shadow",
            include_str!("../shaders/shadow.vert"),
            include_str!("../shaders/shadow.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::shadow(&context.device, shadow_render_pass.handle, vertex, fragment)
            },
        )?;
        let post_pipeline = make_pipeline(
            &context.device,
            "post_process",
            include_str!("../shaders/post_process.vert"),
            include_str!("../shaders/post_process.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::post_process(
                    &context.device,
                    swapchain_render_pass.handle,
                    composite_layout.handle,
                    vertex,
                    fragment,
                )
            },
        )?;
        let ui_pipeline = make_pipeline(
            &context.device,
            "ui",
            include_str!("../shaders/ui.vert"),
            include_str!("../shaders/ui.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::ui(
                    &context.device,
                    swapchain_render_pass.handle,
                    single_texture_layout.handle,
                    vertex,
                    fragment,
                )
            },
        )?;
        let bright_pipeline = make_pipeline(
            &context.device,
            "bloom_bright",
            include_str!("../shaders/post_process.vert"),
            include_str!("../shaders/bloom_bright.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::offscreen(
                    &context.device,
                    bloom_render_pass.handle,
                    single_texture_layout.handle,
                    vertex,
                    fragment,
                    0,
                )
            },
        )?;
        let blur_pipeline = make_pipeline(
            &context.device,
            "blur",
            include_str!("../shaders/post_process.vert"),
            include_str!("../shaders/blur.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::offscreen(
                    &context.device,
                    bloom_render_pass.handle,
                    single_texture_layout.handle,
                    vertex,
                    fragment,
                    8,
                )
            },
        )?;
        let ssao_pipeline = make_pipeline(
            &context.device,
            "ssao",
            include_str!("../shaders/post_process.vert"),
            include_str!("../shaders/ssao.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::offscreen(
                    &context.device,
                    ssao_render_pass.handle,
                    single_texture_layout.handle,
                    vertex,
                    fragment,
                    128,
                )
            },
        )?;
        let ssao_blur_pipeline = make_pipeline(
            &context.device,
            "ssao_blur",
            include_str!("../shaders/post_process.vert"),
            include_str!("../shaders/ssao_blur.frag"),
            shaderc::ShaderKind::Vertex,
            shaderc::ShaderKind::Fragment,
            |vertex, fragment| {
                pipeline::offscreen(
                    &context.device,
                    ssao_render_pass.handle,
                    single_texture_layout.handle,
                    vertex,
                    fragment,
                    8,
                )
            },
        )?;

        let sync = SyncObjects::new(
            &context.device,
            frame.swapchain.images.len(),
            MAX_FRAMES_IN_FLIGHT,
        )?;
        let command_pool = CommandPool::new(&context.device, context.graphics_family)?;
        let command_buffers = command_pool.allocate(MAX_FRAMES_IN_FLIGHT as u32)?;
        let shadow_command_buffers = command_pool.allocate(MAX_FRAMES_IN_FLIGHT as u32)?;

        let stats = RenderStats {
            vertices: scene_gpu.total_vertices,
            triangles: scene_gpu.total_vertices / 3,
            draw_calls: 0,
        };

        println!(
            "[engine-vulkan] ready: {} vertices in {} chunks, {} lights",
            scene_gpu.total_vertices,
            scene_gpu.meshes.len(),
            scene_gpu.lights.len()
        );

        Ok(Self {
            world_pool,
            bright_pool,
            blur_pool,
            ssao_pool,
            ssao_blur_pool,
            post_pool,
            ui_pool,
            world_set,
            bright_set,
            blur_sets,
            ssao_set,
            ssao_blur_sets,
            post_set,
            ui_set,
            shadow_target,
            frame,
            world_pipeline,
            shadow_pipeline,
            post_pipeline,
            ui_pipeline,
            bright_pipeline,
            blur_pipeline,
            ssao_pipeline,
            ssao_blur_pipeline,
            hdr_render_pass,
            bloom_render_pass,
            ssao_render_pass,
            swapchain_render_pass,
            shadow_render_pass,
            world_layout,
            single_texture_layout,
            composite_layout,
            scene: scene_gpu,
            ibl_irradiance,
            ibl_prefiltered,
            font,
            light_buffer,
            ui_buffer,
            ui_capacity,
            ui_vertex_count: 0,
            sync,
            command_pool,
            command_buffers,
            shadow_command_buffers,
            current_frame: 0,
            stats,
            size,
            context,
            vsync,
            capture_requested: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.size = (width, height);
        if width == 0 || height == 0 {
            return Ok(());
        }
        let extent = self.frame.swapchain.extent;
        if width == extent.width && height == extent.height {
            return Ok(());
        }
        self.recreate_swapchain()
    }

    pub fn set_vsync(&mut self, vsync: VsyncMode) -> Result<()> {
        if self.vsync == vsync {
            return Ok(());
        }
        self.vsync = vsync;
        self.recreate_swapchain()
    }

    pub fn render(&mut self, frame: &FrameData<'_>) -> Result<()> {
        let frame_index = self.current_frame;
        let fence = self.sync.in_flight[frame_index];
        unsafe {
            self.context
                .device
                .wait_for_fences(&[fence], true, u64::MAX)?
        };

        let image_available = self.sync.image_available[frame_index];
        let image_index = match self.frame.swapchain.acquire(image_available)? {
            Some(index) => index,
            None => {
                self.recreate_swapchain()?;
                return Ok(());
            }
        };

        unsafe { self.context.device.reset_fences(&[fence])? };
        let command_buffer = self.command_buffers[frame_index];

        self.upload_per_frame_data(frame, frame_index)?;
        self.record_shadow_pass(frame_index)?;
        self.record_main_pass(command_buffer, image_index, frame)?;

        let shadow_cmd = self.shadow_command_buffers[frame_index];
        let wait_semaphores = [image_available];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.sync.render_finished[image_index as usize]];
        // Shadow and main work are submitted together. A pipeline barrier at
        // the end of the shadow command buffer orders the shadow-map writes
        // before the forward pass samples them.
        let command_buffers = [shadow_cmd, command_buffer];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.context
                .device
                .queue_submit(self.context.graphics_queue, &[submit], fence)?
        };

        // A screenshot is captured from the image we have just rendered into.
        // Waiting for the frame fence guarantees that the color writes are
        // complete; the image is still owned by us until `present` below.
        let capture_result = if self.capture_requested {
            self.capture_requested = false;
            match unsafe {
                self.context
                    .device
                    .wait_for_fences(&[fence], true, u64::MAX)
            } {
                Ok(()) => self.capture_image(image_index),
                Err(error) => Err(error.into()),
            }
        } else {
            Ok(())
        };

        let out_of_date = self.frame.swapchain.present(
            self.context.present_queue,
            image_index,
            self.sync.render_finished[image_index as usize],
        )?;
        if out_of_date {
            self.recreate_swapchain()?;
        }
        capture_result?;

        self.stats.vertices = self.scene.total_vertices;
        self.stats.triangles = self.scene.total_vertices / 3;
        self.current_frame += 1;
        if self.current_frame >= MAX_FRAMES_IN_FLIGHT {
            self.current_frame = 0;
        }
        Ok(())
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.context.device.device_wait_idle()? };
        Ok(())
    }

    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    pub fn extent(&self) -> (u32, u32) {
        self.size
    }

    pub fn screenshot(&mut self) -> Result<()> {
        // Capture happens in `render` while the swapchain image is acquired.
        self.capture_requested = true;
        Ok(())
    }

    /// Copies the rendered swapchain image into host memory and writes a PNG.
    ///
    /// Called from `render` after the frame fence has been waited on and before
    /// the image is presented, so the image is still owned by the application
    /// and contains the frame that was just rendered.
    fn capture_image(&mut self, image_index: u32) -> Result<()> {
        if !self.frame.swapchain.transfer_src {
            anyhow::bail!(
                "the swapchain was not created with TRANSFER_SRC usage; screenshots are unsupported"
            );
        }
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("screenshot_{timestamp}.png");

        let device = &self.context.device;
        let queue = self.context.graphics_queue;
        let image = self.frame.swapchain.images[image_index as usize];
        let extent = self.frame.swapchain.extent;
        let format = self.frame.swapchain.format;

        let buffer_size = (extent.width * extent.height * 4) as u64;
        let staging_buffer = GpuBuffer::new(
            &self.context.device,
            &self.context.instance,
            self.context.physical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let commands = ImmediateCommands::new(device, self.context.graphics_family, queue)?;
        commands.run(|cmd_buf| {
            let subresource = vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };
            unsafe {
                // The render pass left the image in PRESENT_SRC_KHR. The
                // application still owns it, so transition it for the copy.
                let to_transfer = vk::ImageMemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::empty())
                    .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
                    .old_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .image(image)
                    .subresource_range(subresource);
                device.cmd_pipeline_barrier(
                    cmd_buf,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    std::slice::from_ref(&to_transfer),
                );

                let copy_region = vk::BufferImageCopy::default()
                    .buffer_offset(0)
                    .buffer_row_length(extent.width)
                    .buffer_image_height(extent.height)
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                    .image_extent(vk::Extent3D {
                        width: extent.width,
                        height: extent.height,
                        depth: 1,
                    });
                device.cmd_copy_image_to_buffer(
                    cmd_buf,
                    image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    staging_buffer.handle,
                    std::slice::from_ref(&copy_region),
                );

                let to_present = vk::ImageMemoryBarrier::default()
                    .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(image)
                    .subresource_range(subresource);
                device.cmd_pipeline_barrier(
                    cmd_buf,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    std::slice::from_ref(&to_present),
                );
            }
            Ok(())
        })?;

        let data = unsafe {
            let ptr = device.map_memory(
                staging_buffer.memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            let slice = std::slice::from_raw_parts(ptr as *const u8, buffer_size as usize);
            let vec = slice.to_vec();
            device.unmap_memory(staging_buffer.memory);
            vec
        };

        // Vulkan framebuffer row 0 is the top row, which is exactly the order
        // PNG expects; only the BGRA byte order may need swapping.
        let bgra = matches!(
            format,
            vk::Format::B8G8R8A8_UNORM | vk::Format::B8G8R8A8_SRGB
        );
        let mut rgba = vec![0u8; buffer_size as usize];
        for (src, dst) in data.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
            if bgra {
                dst[0] = src[2];
                dst[1] = src[1];
                dst[2] = src[0];
            } else {
                dst[0] = src[0];
                dst[1] = src[1];
                dst[2] = src[2];
            }
            dst[3] = src[3];
        }

        let mut file = std::fs::File::create(&filename)
            .with_context(|| format!("failed to create {filename}"))?;
        let mut encoder = png::Encoder::new(&mut file, extent.width, extent.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .context("failed to write PNG header")?;
        writer
            .write_image_data(&rgba)
            .context("failed to write PNG image data")?;

        eprintln!("[engine-vulkan] screenshot saved to {filename}");
        Ok(())
    }

    fn upload_per_frame_data(&mut self, frame: &FrameData<'_>, _frame_index: usize) -> Result<()> {
        let lights = LightUniformBuffer::from_nearest(&self.scene.lights, frame.camera.position);
        self.light_buffer.write_pod(&lights)?;

        if frame.ui_vertices.len() > self.ui_capacity {
            let new_capacity = frame.ui_vertices.len().next_power_of_two();
            self.ui_buffer = GpuBuffer::host_visible(
                &self.context.device,
                &self.context.instance,
                self.context.physical_device,
                (std::mem::size_of::<UiVertex>() * new_capacity) as u64,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            )?;
            self.ui_capacity = new_capacity;
        }
        if !frame.ui_vertices.is_empty() {
            self.ui_buffer.write_slice(frame.ui_vertices)?;
        }
        self.ui_vertex_count = frame.ui_vertices.len() as u32;
        Ok(())
    }

    fn record_shadow_pass(&mut self, frame_index: usize) -> Result<()> {
        let command_buffer = self.shadow_command_buffers[frame_index];
        let light = self.scene.primary_light;
        let light_view_projection = vulkan_projection(light.view_projection());
        let light_bytes = light_view_projection.to_cols_array();

        let planes = extract_frustum_planes(light_view_projection);
        let visible: Vec<(vk::Buffer, u32)> = self
            .scene
            .meshes
            .iter()
            .filter(|mesh| frustum_aabb_intersect(&planes, &mesh.bounds))
            .map(|mesh| (mesh.buffer.handle, mesh.vertex_count))
            .collect();

        unsafe {
            self.context
                .device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
            let begin = vk::CommandBufferBeginInfo::default();
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin)?;

            let clear = [vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            }];
            let extent = vk::Extent2D {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
            };
            let render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            let render_pass_begin = vk::RenderPassBeginInfo::default()
                .render_pass(self.shadow_render_pass.handle)
                .framebuffer(self.shadow_target.framebuffer)
                .render_area(render_area)
                .clear_values(&clear);
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = render_area;
            self.context.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.shadow_pipeline.handle,
            );
            self.context
                .device
                .cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.context
                .device
                .cmd_set_scissor(command_buffer, 0, &[scissor]);
            self.context.device.cmd_push_constants(
                command_buffer,
                self.shadow_pipeline.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                bytemuck::cast_slice(&light_bytes),
            );
            for (buffer, count) in visible {
                self.context
                    .device
                    .cmd_bind_vertex_buffers(command_buffer, 0, &[buffer], &[0]);
                self.context.device.cmd_draw(command_buffer, count, 1, 0, 0);
            }
            self.context.device.cmd_end_render_pass(command_buffer);

            // Make the shadow map visible to the forward fragment shader.
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.shadow_target.image.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::DEPTH,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            self.context.device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            self.context.device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    fn record_main_pass(
        &mut self,
        command_buffer: vk::CommandBuffer,
        image_index: u32,
        frame: &FrameData<'_>,
    ) -> Result<()> {
        let extent = self.frame.swapchain.extent;
        let bloom_extent = self.frame.bloom_extent;

        let projection = vulkan_projection(frame.camera.projection);
        let view_projection = projection * frame.camera.view;
        let light = self.scene.primary_light;
        let light_view_projection = vulkan_projection(light.view_projection());

        let primary_color = light.color * light.intensity;
        let push = WorldPushConstants {
            mvp: view_projection.to_cols_array(),
            light_view_projection: light_view_projection.to_cols_array(),
            light_position: [
                light.position.x,
                light.position.y,
                light.position.z,
                10_000.0,
            ],
            light_color: [primary_color.x, primary_color.y, primary_color.z, 1.0],
            camera_position: [
                frame.camera.position.x,
                frame.camera.position.y,
                frame.camera.position.z,
                1.0,
            ],
            screen_and_debug: [
                extent.width as f32,
                extent.height as f32,
                frame.debug_view.as_u32() as f32,
                0.0,
            ],
            material: [self.scene.metallic, self.scene.roughness, 0.0, 0.0],
        };
        let push_bytes = bytemuck::bytes_of(&push);

        let camera_planes = extract_frustum_planes(view_projection);
        let visible: Vec<(vk::Buffer, u32)> = self
            .scene
            .meshes
            .iter()
            .filter(|mesh| frustum_aabb_intersect(&camera_planes, &mesh.bounds))
            .map(|mesh| (mesh.buffer.handle, mesh.vertex_count))
            .collect();
        self.stats.draw_calls = visible.len() as u32;

        let inverse_projection = projection.inverse();
        let ssao_push = SsaoPushConstants {
            projection: projection.to_cols_array(),
            inverse_projection: inverse_projection.to_cols_array(),
        };
        let ssao_bytes = bytemuck::bytes_of(&ssao_push);

        let full_viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let full_scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let bloom_viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: bloom_extent.width as f32,
            height: bloom_extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let bloom_scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: bloom_extent,
        };

        unsafe {
            let begin = vk::CommandBufferBeginInfo::default();
            self.context
                .device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
            self.context
                .device
                .begin_command_buffer(command_buffer, &begin)?;

            // 1. Scene -> HDR + depth.
            {
                let clears = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: self.scene.clear_color,
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ];
                let begin = vk::RenderPassBeginInfo::default()
                    .render_pass(self.hdr_render_pass.handle)
                    .framebuffer(self.frame.hdr_framebuffer)
                    .render_area(full_scissor)
                    .clear_values(&clears);
                self.context.device.cmd_begin_render_pass(
                    command_buffer,
                    &begin,
                    vk::SubpassContents::INLINE,
                );
                self.context
                    .device
                    .cmd_set_viewport(command_buffer, 0, &[full_viewport]);
                self.context
                    .device
                    .cmd_set_scissor(command_buffer, 0, &[full_scissor]);
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.world_pipeline.handle,
                );
                self.context.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.world_pipeline.layout,
                    0,
                    &[self.world_set],
                    &[],
                );
                self.context.device.cmd_push_constants(
                    command_buffer,
                    self.world_pipeline.layout,
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    push_bytes,
                );
                for (buffer, count) in &visible {
                    self.context.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[*buffer],
                        &[0],
                    );
                    self.context
                        .device
                        .cmd_draw(command_buffer, *count, 1, 0, 0);
                }
                self.context.device.cmd_end_render_pass(command_buffer);
            }

            // 2. SSAO from depth.
            {
                let clear = [vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [1.0, 1.0, 1.0, 1.0],
                    },
                }];
                let begin = vk::RenderPassBeginInfo::default()
                    .render_pass(self.ssao_render_pass.handle)
                    .framebuffer(self.frame.ssao_a_framebuffer)
                    .render_area(bloom_scissor)
                    .clear_values(&clear);
                self.context.device.cmd_begin_render_pass(
                    command_buffer,
                    &begin,
                    vk::SubpassContents::INLINE,
                );
                self.context
                    .device
                    .cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
                self.context
                    .device
                    .cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.ssao_pipeline.handle,
                );
                self.context.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.ssao_pipeline.layout,
                    0,
                    &[self.ssao_set],
                    &[],
                );
                self.context.device.cmd_push_constants(
                    command_buffer,
                    self.ssao_pipeline.layout,
                    vk::ShaderStageFlags::FRAGMENT,
                    0,
                    ssao_bytes,
                );
                self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0);
                self.context.device.cmd_end_render_pass(command_buffer);
            }

            self.ssao_blur_pass(
                command_buffer,
                self.frame.ssao_b_framebuffer,
                self.ssao_blur_pipeline.handle,
                self.ssao_blur_pipeline.layout,
                self.ssao_blur_sets[0],
                [1.0, 0.0],
                bloom_viewport,
                bloom_scissor,
            );
            self.ssao_blur_pass(
                command_buffer,
                self.frame.ssao_a_framebuffer,
                self.ssao_blur_pipeline.handle,
                self.ssao_blur_pipeline.layout,
                self.ssao_blur_sets[1],
                [0.0, 1.0],
                bloom_viewport,
                bloom_scissor,
            );

            // 3. Bloom bright + separable blur.
            {
                let clear = [vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                }];
                let begin = vk::RenderPassBeginInfo::default()
                    .render_pass(self.bloom_render_pass.handle)
                    .framebuffer(self.frame.bloom_a_framebuffer)
                    .render_area(bloom_scissor)
                    .clear_values(&clear);
                self.context.device.cmd_begin_render_pass(
                    command_buffer,
                    &begin,
                    vk::SubpassContents::INLINE,
                );
                self.context
                    .device
                    .cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
                self.context
                    .device
                    .cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.bright_pipeline.handle,
                );
                self.context.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.bright_pipeline.layout,
                    0,
                    &[self.bright_set],
                    &[],
                );
                self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0);
                self.context.device.cmd_end_render_pass(command_buffer);
            }

            self.bloom_blur_pass(
                command_buffer,
                self.frame.bloom_b_framebuffer,
                self.blur_sets[0],
                [1.0, 0.0],
                bloom_viewport,
                bloom_scissor,
            );
            self.bloom_blur_pass(
                command_buffer,
                self.frame.bloom_a_framebuffer,
                self.blur_sets[1],
                [0.0, 1.0],
                bloom_viewport,
                bloom_scissor,
            );

            // 4. Composite + UI -> swapchain.
            {
                let clears = [
                    vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    },
                    vk::ClearValue {
                        depth_stencil: vk::ClearDepthStencilValue {
                            depth: 1.0,
                            stencil: 0,
                        },
                    },
                ];
                let begin = vk::RenderPassBeginInfo::default()
                    .render_pass(self.swapchain_render_pass.handle)
                    .framebuffer(self.frame.swapchain_framebuffers[image_index as usize])
                    .render_area(full_scissor)
                    .clear_values(&clears);
                self.context.device.cmd_begin_render_pass(
                    command_buffer,
                    &begin,
                    vk::SubpassContents::INLINE,
                );
                self.context
                    .device
                    .cmd_set_viewport(command_buffer, 0, &[full_viewport]);
                self.context
                    .device
                    .cmd_set_scissor(command_buffer, 0, &[full_scissor]);
                self.context.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.post_pipeline.handle,
                );
                self.context.device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.post_pipeline.layout,
                    0,
                    &[self.post_set],
                    &[],
                );
                self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0);

                if self.ui_vertex_count > 0 {
                    self.context.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.ui_pipeline.handle,
                    );
                    self.context.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.ui_pipeline.layout,
                        0,
                        &[self.ui_set],
                        &[],
                    );
                    self.context.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[self.ui_buffer.handle],
                        &[0],
                    );
                    self.context
                        .device
                        .cmd_draw(command_buffer, self.ui_vertex_count, 1, 0, 0);
                }
                self.context.device.cmd_end_render_pass(command_buffer);
            }

            self.context.device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn ssao_blur_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        framebuffer: vk::Framebuffer,
        pipeline: vk::Pipeline,
        layout: vk::PipelineLayout,
        descriptor_set: vk::DescriptorSet,
        direction: [f32; 2],
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        let clear = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }];
        let begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.ssao_render_pass.handle)
            .framebuffer(framebuffer)
            .render_area(scissor)
            .clear_values(&clear);
        unsafe {
            self.context.device.cmd_begin_render_pass(
                command_buffer,
                &begin,
                vk::SubpassContents::INLINE,
            );
            self.context
                .device
                .cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.context
                .device
                .cmd_set_scissor(command_buffer, 0, &[scissor]);
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );
            self.context.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                layout,
                0,
                &[descriptor_set],
                &[],
            );
            self.context.device.cmd_push_constants(
                command_buffer,
                layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&direction),
            );
            self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.context.device.cmd_end_render_pass(command_buffer);
        }
    }

    fn bloom_blur_pass(
        &self,
        command_buffer: vk::CommandBuffer,
        framebuffer: vk::Framebuffer,
        descriptor_set: vk::DescriptorSet,
        direction: [f32; 2],
        viewport: vk::Viewport,
        scissor: vk::Rect2D,
    ) {
        let clear = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];
        let begin = vk::RenderPassBeginInfo::default()
            .render_pass(self.bloom_render_pass.handle)
            .framebuffer(framebuffer)
            .render_area(scissor)
            .clear_values(&clear);
        unsafe {
            self.context.device.cmd_begin_render_pass(
                command_buffer,
                &begin,
                vk::SubpassContents::INLINE,
            );
            self.context
                .device
                .cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.context
                .device
                .cmd_set_scissor(command_buffer, 0, &[scissor]);
            self.context.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.blur_pipeline.handle,
            );
            self.context.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.blur_pipeline.layout,
                0,
                &[descriptor_set],
                &[],
            );
            self.context.device.cmd_push_constants(
                command_buffer,
                self.blur_pipeline.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&direction),
            );
            self.context.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.context.device.cmd_end_render_pass(command_buffer);
        }
    }

    fn recreate_swapchain(&mut self) -> Result<()> {
        unsafe { self.context.device.device_wait_idle()? };

        if self.size.0 == 0 || self.size.1 == 0 {
            return Ok(());
        }

        let new_frame = SwapchainFrame::new(
            &self.context,
            self.size,
            self.frame.swapchain.format,
            self.hdr_render_pass.handle,
            self.bloom_render_pass.handle,
            self.ssao_render_pass.handle,
            self.swapchain_render_pass.handle,
            self.frame.swapchain.handle,
            self.vsync,
        )?;

        self.sync
            .recreate_render_finished(new_frame.swapchain.images.len())?;

        descriptor::update_single(
            &self.context.device,
            self.bright_set,
            new_frame.hdr.sampler,
            new_frame.hdr.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &self.context.device,
            self.blur_sets[0],
            new_frame.bloom_a.sampler,
            new_frame.bloom_a.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &self.context.device,
            self.blur_sets[1],
            new_frame.bloom_b.sampler,
            new_frame.bloom_b.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &self.context.device,
            self.ssao_set,
            new_frame.depth.sampler,
            new_frame.depth.view,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &self.context.device,
            self.ssao_blur_sets[0],
            new_frame.ssao_a.sampler,
            new_frame.ssao_a.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_single(
            &self.context.device,
            self.ssao_blur_sets[1],
            new_frame.ssao_b.sampler,
            new_frame.ssao_b.view,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );
        descriptor::update_composite(
            &self.context.device,
            self.post_set,
            (new_frame.hdr.sampler, new_frame.hdr.view),
            (new_frame.bloom_a.sampler, new_frame.bloom_a.view),
            (new_frame.ssao_a.sampler, new_frame.ssao_a.view),
        );

        self.frame = new_frame;
        Ok(())
    }
}

impl Renderer for VulkanRenderer {
    fn backend(&self) -> BackendKind {
        BackendKind::Vulkan
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        VulkanRenderer::resize(self, width, height)
    }

    fn render(&mut self, frame: &FrameData<'_>) -> Result<()> {
        VulkanRenderer::render(self, frame)
    }

    fn wait_idle(&self) -> Result<()> {
        VulkanRenderer::wait_idle(self)
    }

    fn stats(&self) -> RenderStats {
        VulkanRenderer::stats(self)
    }

    fn extent(&self) -> (u32, u32) {
        VulkanRenderer::extent(self)
    }

    fn set_vsync(&mut self, vsync: VsyncMode) -> Result<()> {
        VulkanRenderer::set_vsync(self, vsync)
    }

    fn vsync(&self) -> VsyncMode {
        self.vsync
    }

    fn screenshot(&mut self) -> Result<()> {
        VulkanRenderer::screenshot(self)
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        // All child resources (pools, framebuffers, buffers) are destroyed by
        // their own Drop implementations after this function returns, so the
        // device must be idle first.
        unsafe {
            let _ = self.context.device.device_wait_idle();
        }
    }
}

fn choose_initial_format(context: &VulkanContext) -> Result<vk::Format> {
    let support = crate::swapchain::query_support(
        context.physical_device,
        &context.surface_loader,
        context.surface,
    )?;
    Ok(crate::swapchain::choose_surface_format(&support.formats, None).format)
}

fn make_pipeline<F>(
    device: &Device,
    name: &str,
    vertex_source: &str,
    fragment_source: &str,
    vertex_kind: shaderc::ShaderKind,
    fragment_kind: shaderc::ShaderKind,
    create: F,
) -> Result<Pipeline>
where
    F: FnOnce(vk::ShaderModule, vk::ShaderModule) -> Result<Pipeline>,
{
    let vertex_code = crate::shader::compile(vertex_source, vertex_kind, name)?;
    let fragment_code = crate::shader::compile(fragment_source, fragment_kind, name)?;
    let vertex = crate::shader::module(device, &vertex_code)
        .with_context(|| format!("creating vertex module for {name}"))?;
    let fragment = crate::shader::module(device, &fragment_code)
        .with_context(|| format!("creating fragment module for {name}"))?;
    let result = create(vertex, fragment);
    unsafe {
        device.destroy_shader_module(vertex, None);
        device.destroy_shader_module(fragment, None);
    }
    result
}

/// Converts an engine projection (OpenGL convention: y up, depth `[-1, 1]`)
/// into a Vulkan clip-space projection (y down, depth `[0, 1]`).
///
/// The z remap is `z' = 0.5 * z + 0.5 * w`, which maps NDC −1..1 to 0..1
/// without changing the perspective divide. Forgetting it leaves shadow depth
/// comparisons and SSAO depth reconstruction wrong.
fn vulkan_projection(mut projection: Mat4) -> Mat4 {
    // Work on matrix rows: clip-space coordinates are row · vertex, so the
    // Y flip and the depth remap must modify whole rows, not single columns.
    let y_row = -projection.row(1);
    let z_row = projection.row(2) * 0.5 + projection.row(3) * 0.5;
    projection.x_axis.y = y_row.x;
    projection.y_axis.y = y_row.y;
    projection.z_axis.y = y_row.z;
    projection.w_axis.y = y_row.w;
    projection.x_axis.z = z_row.x;
    projection.y_axis.z = z_row.y;
    projection.z_axis.z = z_row.z;
    projection.w_axis.z = z_row.w;
    projection
}

fn extract_frustum_planes(matrix: Mat4) -> [Vec4; 6] {
    let m = matrix.to_cols_array_2d();
    let mut planes = [Vec4::ZERO; 6];
    planes[0] = Vec4::new(
        m[0][3] + m[0][0],
        m[1][3] + m[1][0],
        m[2][3] + m[2][0],
        m[3][3] + m[3][0],
    );
    planes[1] = Vec4::new(
        m[0][3] - m[0][0],
        m[1][3] - m[1][0],
        m[2][3] - m[2][0],
        m[3][3] - m[3][0],
    );
    planes[2] = Vec4::new(
        m[0][3] + m[0][1],
        m[1][3] + m[1][1],
        m[2][3] + m[2][1],
        m[3][3] + m[3][1],
    );
    planes[3] = Vec4::new(
        m[0][3] - m[0][1],
        m[1][3] - m[1][1],
        m[2][3] - m[2][1],
        m[3][3] - m[3][1],
    );
    planes[4] = Vec4::new(
        m[0][3] + m[0][2],
        m[1][3] + m[1][2],
        m[2][3] + m[2][2],
        m[3][3] + m[3][2],
    );
    planes[5] = Vec4::new(
        m[0][3] - m[0][2],
        m[1][3] - m[1][2],
        m[2][3] - m[2][2],
        m[3][3] - m[3][2],
    );
    for plane in &mut planes {
        *plane = plane.normalize();
    }
    planes
}

fn frustum_aabb_intersect(planes: &[Vec4; 6], aabb: &Aabb) -> bool {
    let center = aabb.center();
    let extents = aabb.half_extents();
    for plane in planes {
        let radius =
            extents.x * plane.x.abs() + extents.y * plane.y.abs() + extents.z * plane.z.abs();
        if radius + plane.dot(center.extend(1.0)) < 0.0 {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
fn unused(_: Vertex) {}
