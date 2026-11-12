use anyhow::{anyhow, Result};
use ash::{khr, vk, Entry, Instance};
use winit::window::Window;

use crate::camera::Camera;
use crate::ui::DebugMenu;
use super::{buffer, commands, depth, descriptor, device, instance, pipeline, render_pass, shader, shadow, surface, swapchain, synchronization, texture};

const MAX_FRAMES_IN_FLIGHT: usize = 2;
const HDR_CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const SWAPCHAIN_CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

pub struct VulkanContext {
    instance: Instance,
    surface_loader: khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    swapchain_loader: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_format: vk::Format,
    swapchain_extent: vk::Extent2D,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,

    depth_image: vk::Image,
    depth_memory: vk::DeviceMemory,
    depth_image_view: vk::ImageView,

    render_pass: vk::RenderPass,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    image_available_semaphores: Vec<vk::Semaphore>,
    render_finished_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,
    current_frame: usize,

    hdr_image: vk::Image,
    hdr_memory: vk::DeviceMemory,
    hdr_view: vk::ImageView,
    hdr_sampler: vk::Sampler,
    hdr_render_pass: vk::RenderPass,
    hdr_framebuffer: vk::Framebuffer,

    post_pipeline: vk::Pipeline,
    post_pipeline_layout: vk::PipelineLayout,
    post_descriptor_layout: vk::DescriptorSetLayout,
    post_descriptor_pool: vk::DescriptorPool,
    post_descriptor_set: vk::DescriptorSet,

    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    vertex_buffers: Vec<(vk::Buffer, vk::DeviceMemory, u32)>,
    chunk_bounds: Vec<crate::world::Collider>,
    total_vertices: u32,

    ui_pipeline: vk::Pipeline,
    ui_pipeline_layout: vk::PipelineLayout,
    ui_vertex_buffer: vk::Buffer,
    ui_vertex_memory: vk::DeviceMemory,
    ui_vertex_count: u32,
    ui_buffer_capacity: usize,
    font_image: vk::Image,
    font_memory: vk::DeviceMemory,
    font_view: vk::ImageView,
    font_sampler: vk::Sampler,
    ui_descriptor_layout: vk::DescriptorSetLayout,
    ui_descriptor_pool: vk::DescriptorPool,
    ui_descriptor_set: vk::DescriptorSet,

    world_texture_image: vk::Image,
    world_texture_memory: vk::DeviceMemory,
    world_texture_view: vk::ImageView,
    world_texture_sampler: vk::Sampler,
    world_descriptor_layout: vk::DescriptorSetLayout,
    world_descriptor_pool: vk::DescriptorPool,
    world_descriptor_set: vk::DescriptorSet,
    light_buffer: vk::Buffer,
    light_buffer_memory: vk::DeviceMemory,
    light_list: Vec<(glam::Vec3, glam::Vec3, f32)>, // (pos, color, radius)

    shadow_image: vk::Image,
    shadow_memory: vk::DeviceMemory,
    shadow_view: vk::ImageView,
    shadow_sampler: vk::Sampler,
    shadow_render_pass: vk::RenderPass,
    shadow_framebuffer: vk::Framebuffer,
    shadow_pipeline: vk::Pipeline,
    shadow_pipeline_layout: vk::PipelineLayout,
    shadow_cmds: Vec<vk::CommandBuffer>,

    #[allow(dead_code)] ibl_irradiance_image: vk::Image,
    #[allow(dead_code)] ibl_irradiance_memory: vk::DeviceMemory,
    ibl_irradiance_view: vk::ImageView,
    #[allow(dead_code)] ibl_irradiance_sampler: vk::Sampler,
    #[allow(dead_code)] ibl_prefiltered_image: vk::Image,
    #[allow(dead_code)] ibl_prefiltered_memory: vk::DeviceMemory,
    ibl_prefiltered_view: vk::ImageView,
    #[allow(dead_code)] ibl_prefiltered_sampler: vk::Sampler,

    // Bloom
    bloom_a_image: vk::Image,
    bloom_a_memory: vk::DeviceMemory,
    bloom_a_view: vk::ImageView,
    bloom_a_sampler: vk::Sampler,
    bloom_b_sampler: vk::Sampler,
    bloom_a_framebuffer: vk::Framebuffer,
    bloom_b_image: vk::Image,
    bloom_b_memory: vk::DeviceMemory,
    bloom_b_view: vk::ImageView,
    bloom_b_framebuffer: vk::Framebuffer,
    bloom_bright_pipeline: vk::Pipeline,
    bloom_bright_layout: vk::PipelineLayout,
    bloom_bright_desc_layout: vk::DescriptorSetLayout,
    bloom_bright_desc_pool: vk::DescriptorPool,
    bloom_bright_desc: vk::DescriptorSet,
    blur_pipeline: vk::Pipeline,
    blur_layout: vk::PipelineLayout,
    blur_desc_layout: vk::DescriptorSetLayout,
    blur_desc_pool: vk::DescriptorPool,
    blur_desc_a: vk::DescriptorSet,
    blur_desc_b: vk::DescriptorSet,
    bloom_render_pass: vk::RenderPass,

    // SSAO
    depth_sampler: vk::Sampler,
    ssao_a_image: vk::Image,
    ssao_a_memory: vk::DeviceMemory,
    ssao_a_view: vk::ImageView,
    ssao_b_image: vk::Image,
    ssao_b_memory: vk::DeviceMemory,
    ssao_b_view: vk::ImageView,
    ssao_sampler: vk::Sampler,
    ssao_a_framebuffer: vk::Framebuffer,
    ssao_b_framebuffer: vk::Framebuffer,
    ssao_pipeline: vk::Pipeline,
    ssao_layout: vk::PipelineLayout,
    ssao_desc_layout: vk::DescriptorSetLayout,
    ssao_desc_pool: vk::DescriptorPool,
    ssao_desc: vk::DescriptorSet,
    ssao_blur_pipeline: vk::Pipeline,
    ssao_blur_layout: vk::PipelineLayout,
    ssao_blur_desc_layout: vk::DescriptorSetLayout,
    ssao_blur_pool: vk::DescriptorPool,
    ssao_blur_desc_a: vk::DescriptorSet,
    ssao_blur_desc_b: vk::DescriptorSet,
    ssao_render_pass: vk::RenderPass,

    visible_for_light: Vec<(vk::Buffer, u32)>,
    visible_for_camera: Vec<(vk::Buffer, u32)>,

    _entry: Entry,
}

// ИСПРАВЛЕНО: убрана точка с запятой после `?` в конце функции
fn create_hdr_image(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    extent: vk::Extent2D,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView)> {
    let format = vk::Format::R16G16B16A16_SFLOAT;
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
        .mip_levels(1).array_layers(1).format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let image = unsafe { device.create_image(&info, None)? };
    let mem_reqs = unsafe { device.get_image_memory_requirements(image) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let mem_type = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0
            && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL))
        .ok_or_else(|| anyhow!("No HDR memory type"))?;

    let alloc = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? }; // ИСПРАВЛЕНО: без ;

    Ok((image, memory, view))
}

impl VulkanContext {
    pub fn new(window: &Window, chunks: &[(Vec<buffer::Vertex>, crate::world::Collider)]) -> Result<Self> {
        let entry = unsafe { Entry::load()? };
        let instance = instance::create_instance(&entry, window)?;
        let (surface_loader, surface) = surface::create_surface(&entry, &instance, window)?;
        let physical_device = device::pick_physical_device(&instance, &surface_loader, surface)?;

        let indices = device::find_queue_families(&instance, &surface_loader, surface, physical_device)?;
        let graphics_queue_family = indices.graphics_family.ok_or_else(|| anyhow!("No graphics queue family"))?;

        let (device, graphics_queue, present_queue) = device::create_logical_device(&instance, physical_device, &indices)?;

        let (swapchain_loader, swapchain, swapchain_format, swapchain_extent, swapchain_images) =
            swapchain::create_swapchain(&instance, &device, physical_device, &surface_loader, surface, window, &indices)?;
        let swapchain_image_views = swapchain::create_image_views(&device, &swapchain_images, swapchain_format)?;
        let (depth_image, depth_memory, depth_image_view, depth_sampler) = depth::create_depth_resources(&instance, &device, physical_device, swapchain_extent)?;

        let render_pass = render_pass::create_render_pass(&device, swapchain_format)?;
        let framebuffers = render_pass::create_framebuffers(&device, render_pass, &swapchain_image_views, depth_image_view, swapchain_extent)?;

        // HDR ресурсы
        let hdr_format = vk::Format::R16G16B16A16_SFLOAT;
        let (hdr_image, hdr_memory, hdr_view) = create_hdr_image(&instance, &device, physical_device, swapchain_extent)?;

        let hdr_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let hdr_sampler = unsafe { device.create_sampler(&hdr_sampler_info, None)? }; // ИСПРАВЛЕНО: без ;

        let hdr_render_pass = render_pass::create_hdr_render_pass(&device, hdr_format)?;
        let hdr_framebuffer = render_pass::create_framebuffers(&device, hdr_render_pass, &[hdr_view], depth_image_view, swapchain_extent)?[0];

        // === Bloom ===
        let bloom_extent = vk::Extent2D {
            width: swapchain_extent.width.max(1) / 2,
            height: swapchain_extent.height.max(1) / 2,
        };
        let (bloom_a_image, bloom_a_memory, bloom_a_view) =
            texture::create_hdr_offscreen(&instance, &device, physical_device, bloom_extent.width, bloom_extent.height)?;
        let (bloom_b_image, bloom_b_memory, bloom_b_view) =
            texture::create_hdr_offscreen(&instance, &device, physical_device, bloom_extent.width, bloom_extent.height)?;

        let bloom_a_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let bloom_a_sampler = unsafe { device.create_sampler(&bloom_a_sampler_info, None)? };
        let bloom_b_sampler = unsafe { device.create_sampler(&bloom_a_sampler_info, None)? };

        let bloom_render_pass = render_pass::create_offscreen_render_pass(&device, hdr_format)?;
        let ssao_render_pass = render_pass::create_ssao_render_pass(&device)?; 
        let bloom_a_framebuffer = render_pass::create_offscreen_framebuffer(&device, bloom_render_pass, bloom_a_view, bloom_extent)?;
        let bloom_b_framebuffer = render_pass::create_offscreen_framebuffer(&device, bloom_render_pass, bloom_b_view, bloom_extent)?;

        // Bright pass pipeline
        let bright_vert_src = include_str!("../../shaders/post_process.vert");
        let bright_frag_src = include_str!("../../shaders/bloom_bright.frag");
        let bright_vert_spv = shader::compile_glsl_to_spirv(bright_vert_src, shaderc::ShaderKind::Vertex)?;
        let bright_frag_spv = shader::compile_glsl_to_spirv(bright_frag_src, shaderc::ShaderKind::Fragment)?;
        let bright_vert_m = shader::create_shader_module(&device, &bright_vert_spv)?;
        let bright_frag_m = shader::create_shader_module(&device, &bright_frag_spv)?;

        let bloom_bright_desc_layout = descriptor::create_single_texture_layout(&device)?;
        let (bloom_bright_desc_pool, bloom_bright_desc) =
            descriptor::create_descriptor_pool_and_set(&device, bloom_bright_desc_layout, 1)?;
        descriptor::update_single_texture(&device, bloom_bright_desc, hdr_sampler, hdr_view);

        let (bloom_bright_pipeline, bloom_bright_layout) = pipeline::create_offscreen_pipeline(
            &device, bloom_render_pass, bright_vert_m, bright_frag_m, bloom_bright_desc_layout, 0
        )?;
        unsafe { device.destroy_shader_module(bright_vert_m, None); device.destroy_shader_module(bright_frag_m, None); }

        // Blur pipeline
        let blur_frag_src = include_str!("../../shaders/blur.frag");
        let blur_frag_spv = shader::compile_glsl_to_spirv(blur_frag_src, shaderc::ShaderKind::Fragment)?;
        let blur_vert_m = shader::create_shader_module(&device, &bright_vert_spv)?;
        let blur_frag_m = shader::create_shader_module(&device, &blur_frag_spv)?;

        let blur_desc_layout = descriptor::create_single_texture_layout(&device)?;
        let blur_pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(2)];
        let blur_pool_info = vk::DescriptorPoolCreateInfo::default().max_sets(2).pool_sizes(&blur_pool_sizes);
        let blur_desc_pool = unsafe { device.create_descriptor_pool(&blur_pool_info, None)? };
        let blur_set_layouts = [blur_desc_layout, blur_desc_layout];
        let blur_alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(blur_desc_pool).set_layouts(&blur_set_layouts);
        let blur_sets = unsafe { device.allocate_descriptor_sets(&blur_alloc)? };
        let (blur_desc_a, blur_desc_b) = (blur_sets[0], blur_sets[1]);
        descriptor::update_single_texture(&device, blur_desc_a, bloom_a_sampler, bloom_a_view);
        descriptor::update_single_texture(&device, blur_desc_b, bloom_b_sampler, bloom_b_view);

        let (blur_pipeline, blur_layout) = pipeline::create_offscreen_pipeline(
            &device, bloom_render_pass, blur_vert_m, blur_frag_m, blur_desc_layout, 8
        )?;
        unsafe { device.destroy_shader_module(blur_vert_m, None); device.destroy_shader_module(blur_frag_m, None); }

                // === SSAO ===
        let depth_sampler = depth_sampler.clone();
        
        let (ssao_a_image, ssao_a_memory, ssao_a_view) =
            texture::create_ssao_offscreen(&instance, &device, physical_device, bloom_extent.width, bloom_extent.height)?;
        let (ssao_b_image, ssao_b_memory, ssao_b_view) =
            texture::create_ssao_offscreen(&instance, &device, physical_device, bloom_extent.width, bloom_extent.height)?;

        let ssao_sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let ssao_sampler = unsafe { device.create_sampler(&ssao_sampler_info, None)? };

        let ssao_a_framebuffer = render_pass::create_offscreen_framebuffer(&device, ssao_render_pass, ssao_a_view, bloom_extent)?;
        let ssao_b_framebuffer = render_pass::create_offscreen_framebuffer(&device, ssao_render_pass, ssao_b_view, bloom_extent)?;

        // SSAO pipeline
        let ssao_frag_src = include_str!("../../shaders/ssao.frag");
        let ssao_frag_spv = shader::compile_glsl_to_spirv(ssao_frag_src, shaderc::ShaderKind::Fragment)?;
        let ssao_vert_m = shader::create_shader_module(&device, &bright_vert_spv)?;
        let ssao_frag_m = shader::create_shader_module(&device, &ssao_frag_spv)?;

        let ssao_desc_layout = descriptor::create_ssao_layout(&device)?;
        let (ssao_desc_pool, ssao_desc) = descriptor::create_descriptor_pool_and_set(&device, ssao_desc_layout, 1)?;
        descriptor::update_single_texture_with_layout(
            &device, ssao_desc, depth_sampler, depth_image_view,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        );

        let (ssao_pipeline, ssao_layout) = pipeline::create_offscreen_pipeline(
            &device, ssao_render_pass, ssao_vert_m, ssao_frag_m, ssao_desc_layout, 128
        )?;
        unsafe { device.destroy_shader_module(ssao_vert_m, None); device.destroy_shader_module(ssao_frag_m, None); }

        // SSAO blur pipeline
        let ssao_blur_frag_src = include_str!("../../shaders/ssao_blur.frag");
        let ssao_blur_frag_spv = shader::compile_glsl_to_spirv(ssao_blur_frag_src, shaderc::ShaderKind::Fragment)?;
        let ssao_blur_vert_m = shader::create_shader_module(&device, &bright_vert_spv)?;
        let ssao_blur_frag_m = shader::create_shader_module(&device, &ssao_blur_frag_spv)?;

                let ssao_blur_desc_layout = descriptor::create_single_texture_layout(&device)?;
        let ssao_blur_pool_sizes = [vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(2)];
        let ssao_blur_pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(2).pool_sizes(&ssao_blur_pool_sizes);
        let ssao_blur_pool = unsafe { device.create_descriptor_pool(&ssao_blur_pool_info, None)? };
        let ssao_blur_set_layouts = [ssao_blur_desc_layout, ssao_blur_desc_layout];
        let ssao_blur_alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(ssao_blur_pool).set_layouts(&ssao_blur_set_layouts);
        let ssao_blur_sets = unsafe { device.allocate_descriptor_sets(&ssao_blur_alloc)? };
        let (ssao_blur_desc_a, ssao_blur_desc_b) = (ssao_blur_sets[0], ssao_blur_sets[1]);
        descriptor::update_single_texture(&device, ssao_blur_desc_a, ssao_sampler, ssao_a_view);
        descriptor::update_single_texture(&device, ssao_blur_desc_b, ssao_sampler, ssao_b_view);

        let (ssao_blur_pipeline, ssao_blur_layout) = pipeline::create_offscreen_pipeline(
            &device, ssao_render_pass, ssao_blur_vert_m, ssao_blur_frag_m, ssao_blur_desc_layout, 8
        )?;
        unsafe { device.destroy_shader_module(ssao_blur_vert_m, None); device.destroy_shader_module(ssao_blur_frag_m, None); }

        let command_pool = commands::create_command_pool(&device, graphics_queue_family)?;
        let command_buffers = commands::create_command_buffers(&device, command_pool, MAX_FRAMES_IN_FLIGHT as u32)?;

        let vert_src = include_str!("../../shaders/shader.vert");
        let frag_src = include_str!("../../shaders/shader.frag");
        let vert_spv = shader::compile_glsl_to_spirv(vert_src, shaderc::ShaderKind::Vertex)?;
        let frag_spv = shader::compile_glsl_to_spirv(frag_src, shaderc::ShaderKind::Fragment)?;
        let vert_module = shader::create_shader_module(&device, &vert_spv)?;
        let frag_module = shader::create_shader_module(&device, &frag_spv)?;

        let world_pixels = crate::world::generate_wall_texture();
        let (world_image, world_memory, world_view, world_sampler) = texture::create_texture(
            &instance, &device, physical_device, command_pool, graphics_queue, 256, 256, &world_pixels, false
        )?;

        let world_descriptor_layout = descriptor::create_world_descriptor_set_layout_with_lights(&device)?;
        let (world_descriptor_pool, world_descriptor_set) = descriptor::create_world_descriptor_pool_and_set(&device, world_descriptor_layout)?;

        // UBO для 8 источников света
        let (light_buffer, light_buffer_memory) = buffer::create_light_buffer(&instance, &device, physical_device)?;

        // Список всех ламп в мире (совпадает с add_light_fixture в world.rs)
        let light_list: Vec<(glam::Vec3, glam::Vec3, f32)> = vec![
            (glam::Vec3::new(  0.0, -0.1,   0.0), glam::Vec3::new(1.0, 0.9, 0.7), 14.0),
            (glam::Vec3::new( -5.0, -0.1,  -5.0), glam::Vec3::new(1.0, 0.9, 0.7), 14.0),
            (glam::Vec3::new(  5.0, -0.1,   5.0), glam::Vec3::new(1.0, 0.9, 0.7), 14.0),
            (glam::Vec3::new(  0.0, -4.1,  15.0), glam::Vec3::new(1.0, 0.95, 0.8), 12.0),
            (glam::Vec3::new(  0.0, -4.1,  22.0), glam::Vec3::new(1.0, 0.95, 0.8), 12.0),
            (glam::Vec3::new(  0.0, -4.1,  30.0), glam::Vec3::new(1.0, 0.95, 0.8), 12.0),
            (glam::Vec3::new(-12.0,  4.9,  50.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new(-12.0,  4.9,  65.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new(-18.0,  4.9,  75.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 12.0,  4.9,  50.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 12.0,  4.9,  65.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 18.0,  4.9,  75.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 35.0, -6.1,  60.0), glam::Vec3::new(1.0, 1.0,  1.0), 12.0),
            (glam::Vec3::new( 45.0, -6.1,  60.0), glam::Vec3::new(1.0, 1.0,  1.0), 12.0),
            (glam::Vec3::new( 62.0,  1.9,  45.0), glam::Vec3::new(0.6, 0.8, 1.0), 14.0),
            (glam::Vec3::new( 72.0,  1.9,  60.0), glam::Vec3::new(0.6, 0.8, 1.0), 14.0),
            (glam::Vec3::new( 85.0,  1.9,  75.0), glam::Vec3::new(0.6, 0.8, 1.0), 14.0),
            (glam::Vec3::new( 60.0, -6.1,  90.0), glam::Vec3::new(1.0, 1.0,  1.0), 12.0),
            (glam::Vec3::new( 60.0, -6.1, 105.0), glam::Vec3::new(1.0, 1.0,  1.0), 12.0),
            (glam::Vec3::new( 20.0,  4.9, 115.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 30.0,  4.9, 125.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
            (glam::Vec3::new( 40.0,  4.9, 135.0), glam::Vec3::new(1.0, 0.85, 0.6), 18.0),
        ];

        let (shadow_image, shadow_memory, shadow_view, shadow_sampler) =
            shadow::create_shadow_resources(&instance, &device, physical_device)?;
        let shadow_render_pass = shadow::create_shadow_render_pass(&device)?;
        let shadow_framebuffer = shadow::create_shadow_framebuffer(&device, shadow_render_pass, shadow_view)?;

        let shadow_vert_src = include_str!("../../shaders/shadow.vert");
        let shadow_frag_src = include_str!("../../shaders/shadow.frag");
        let sv = shader::compile_glsl_to_spirv(shadow_vert_src, shaderc::ShaderKind::Vertex)?;
        let sf = shader::compile_glsl_to_spirv(shadow_frag_src, shaderc::ShaderKind::Fragment)?;
        let sv_m = shader::create_shader_module(&device, &sv)?;
        let sf_m = shader::create_shader_module(&device, &sf)?;
        let (shadow_pipeline, shadow_pipeline_layout) = shadow::create_shadow_pipeline(&device, shadow_render_pass, sv_m, sf_m)?;
        unsafe { device.destroy_shader_module(sv_m, None); device.destroy_shader_module(sf_m, None); }

        // Процедурный скайбокс (небо + солнце) для IBL
        let (ibl_irradiance_image, ibl_irradiance_memory, ibl_irradiance_view, ibl_irradiance_sampler) =
            texture::create_skybox_cubemap(&instance, &device, physical_device, command_pool, graphics_queue, 128)?;
        let (ibl_prefiltered_image, ibl_prefiltered_memory, ibl_prefiltered_view, ibl_prefiltered_sampler) =
            texture::create_skybox_cubemap(&instance, &device, physical_device, command_pool, graphics_queue, 128)?;

        descriptor::update_world_descriptor_set_with_lights(
            &device, world_descriptor_set,
            world_sampler, world_view,
            shadow_sampler, shadow_view,
            ibl_irradiance_sampler, ibl_irradiance_view,
            ibl_prefiltered_sampler, ibl_prefiltered_view,
            light_buffer, std::mem::size_of::<buffer::LightBuffer>() as u64,
        );

        let (pipeline, pipeline_layout) = pipeline::create_pipeline(
            &device, hdr_render_pass, swapchain_extent, vert_module, frag_module, world_descriptor_layout
        )?;
        unsafe { device.destroy_shader_module(vert_module, None); device.destroy_shader_module(frag_module, None); }

        let mut vertex_buffers = Vec::new();
        let mut chunk_bounds = Vec::new();
        let mut total_vertices = 0;
        for chunk in chunks {
            if !chunk.0.is_empty() {
                let (buf, mem) = buffer::create_vertex_buffer(&instance, &device, physical_device, &chunk.0)?;
                vertex_buffers.push((buf, mem, chunk.0.len() as u32));
                chunk_bounds.push(crate::world::Collider { min: chunk.1.min, max: chunk.1.max });
                total_vertices += chunk.0.len() as u32;
            }
        }

        let font_pixels = crate::ui::generate_font_atlas();
        let (font_image, font_memory, font_view, font_sampler) = texture::create_texture(
            &instance, &device, physical_device, command_pool, graphics_queue, 128, 128, &font_pixels, true
        )?;

        let ui_descriptor_layout = descriptor::create_ui_descriptor_set_layout(&device)?;
        let (ui_descriptor_pool, ui_descriptor_set) = descriptor::create_descriptor_pool_and_set(&device, ui_descriptor_layout, 1)?;
        descriptor::update_ui_descriptor_set(&device, ui_descriptor_set, font_sampler, font_view);

        let ui_vert_src = include_str!("../../shaders/ui.vert");
        let ui_frag_src = include_str!("../../shaders/ui.frag");
        let ui_vert_spv = shader::compile_glsl_to_spirv(ui_vert_src, shaderc::ShaderKind::Vertex)?;
        let ui_frag_spv = shader::compile_glsl_to_spirv(ui_frag_src, shaderc::ShaderKind::Fragment)?;
        let ui_vert_module = shader::create_shader_module(&device, &ui_vert_spv)?;
        let ui_frag_module = shader::create_shader_module(&device, &ui_frag_spv)?;

        let (ui_pipeline, ui_pipeline_layout) = pipeline::create_ui_pipeline(
            &device, render_pass, ui_vert_module, ui_frag_module, ui_descriptor_layout
        )?;
        unsafe { device.destroy_shader_module(ui_vert_module, None); device.destroy_shader_module(ui_frag_module, None); }

        let post_vert_src = include_str!("../../shaders/post_process.vert");
        let post_frag_src = include_str!("../../shaders/post_process.frag");
        let post_vert_spv = shader::compile_glsl_to_spirv(post_vert_src, shaderc::ShaderKind::Vertex)?;
        let post_frag_spv = shader::compile_glsl_to_spirv(post_frag_src, shaderc::ShaderKind::Fragment)?;
        let post_vert_module = shader::create_shader_module(&device, &post_vert_spv)?;
        let post_frag_module = shader::create_shader_module(&device, &post_frag_spv)?;

        let post_descriptor_layout = descriptor::create_composite_layout(&device)?;
        let (post_descriptor_pool, post_descriptor_set) =
            descriptor::create_descriptor_pool_and_set(&device, post_descriptor_layout, 3)?;
        descriptor::update_composite_set(&device, post_descriptor_set, hdr_sampler, hdr_view, bloom_a_sampler, bloom_a_view, ssao_sampler, ssao_a_view);

        let (post_pipeline, post_pipeline_layout) = pipeline::create_post_process_pipeline(
            &device, render_pass, post_vert_module, post_frag_module, post_descriptor_layout
        )?;
        unsafe { device.destroy_shader_module(post_vert_module, None); device.destroy_shader_module(post_frag_module, None); }

        let ui_buffer_capacity = 5000;
        let (ui_vertex_buffer, ui_vertex_memory) = buffer::create_ui_buffer(&instance, &device, physical_device, ui_buffer_capacity)?;

        let image_available_semaphores = synchronization::create_semaphores(&device, MAX_FRAMES_IN_FLIGHT)?;
        let render_finished_semaphores = synchronization::create_semaphores(&device, swapchain_images.len())?;
        let in_flight_fences = synchronization::create_fences(&device, MAX_FRAMES_IN_FLIGHT, true)?;

        let shadow_cmd_alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let shadow_cmds = unsafe { device.allocate_command_buffers(&shadow_cmd_alloc)? };
    
        println!("Vulkan ready: {}x{}, {} images, {} vertices", swapchain_extent.width, swapchain_extent.height, swapchain_images.len(), total_vertices);

        Ok(Self {
            instance, surface_loader, surface, physical_device, device,
            graphics_queue, present_queue, swapchain_loader, swapchain, swapchain_format,
            swapchain_extent, swapchain_images, swapchain_image_views,
            depth_image, depth_memory, depth_image_view,
            render_pass, framebuffers, command_pool, command_buffers,
            image_available_semaphores, render_finished_semaphores, in_flight_fences, current_frame: 0,
            hdr_image, hdr_memory, hdr_view, hdr_sampler, hdr_render_pass, hdr_framebuffer,
            post_pipeline, post_pipeline_layout,
            post_descriptor_layout, post_descriptor_pool, post_descriptor_set,
            pipeline, pipeline_layout,
            vertex_buffers, chunk_bounds, total_vertices,
            world_texture_image: world_image, world_texture_memory: world_memory,
            world_texture_view: world_view, world_texture_sampler: world_sampler,
            world_descriptor_layout, world_descriptor_pool, world_descriptor_set,
            light_buffer, light_buffer_memory, light_list,
            ui_pipeline, ui_pipeline_layout, ui_vertex_buffer, ui_vertex_memory,
            ui_vertex_count: 0, ui_buffer_capacity,
            font_image, font_memory, font_view, font_sampler,
            ui_descriptor_layout, ui_descriptor_pool, ui_descriptor_set,
            shadow_image, shadow_memory, shadow_view, shadow_sampler,
            shadow_render_pass, shadow_framebuffer,
            shadow_pipeline, shadow_pipeline_layout, shadow_cmds,
            ibl_irradiance_image, ibl_irradiance_memory, ibl_irradiance_view, ibl_irradiance_sampler,
            ibl_prefiltered_image, ibl_prefiltered_memory, ibl_prefiltered_view, ibl_prefiltered_sampler,
            visible_for_light: Vec::with_capacity(16),
            visible_for_camera: Vec::with_capacity(16),
            bloom_a_image, bloom_a_memory, bloom_a_view, bloom_a_sampler, bloom_b_sampler, bloom_a_framebuffer,
            bloom_b_image, bloom_b_memory, bloom_b_view, bloom_b_framebuffer,
            bloom_bright_pipeline, bloom_bright_layout, bloom_bright_desc_layout, bloom_bright_desc_pool, bloom_bright_desc,
            blur_pipeline, blur_layout, blur_desc_layout, blur_desc_pool, blur_desc_a, blur_desc_b,
            bloom_render_pass,
            depth_sampler,
            ssao_a_image, ssao_a_memory, ssao_a_view,
            ssao_b_image, ssao_b_memory, ssao_b_view,
            ssao_sampler, ssao_a_framebuffer, ssao_b_framebuffer,
            ssao_pipeline, ssao_layout, ssao_desc_layout, ssao_desc_pool, ssao_desc,
            ssao_blur_pipeline, ssao_blur_layout, ssao_blur_desc_layout, ssao_blur_pool,
            ssao_blur_desc_a, ssao_blur_desc_b,
            ssao_render_pass,
            _entry: entry,
        })
    }

    pub fn update_ui(&mut self, menu: &DebugMenu) {
        let vertices = menu.generate_ui_vertices(self.swapchain_extent.width as f32, self.swapchain_extent.height as f32);
        self.ui_vertex_count = vertices.len() as u32;
        if self.ui_vertex_count as usize > self.ui_buffer_capacity {
            self.ui_vertex_count = self.ui_buffer_capacity as u32;
        }
        if self.ui_vertex_count > 0 {
            buffer::update_ui_buffer(&self.device, self.ui_vertex_memory, &vertices[..self.ui_vertex_count as usize]).unwrap();
        }
    }

    pub fn draw_frame(&mut self, window: &Window, camera: &Camera, debug_mode: u32, fov: f32) -> Result<()> {
        let frame = self.current_frame;
        let fence = self.in_flight_fences[frame];

        unsafe { self.device.wait_for_fences(&[fence], true, u64::MAX)?; }

        let acquire_result = unsafe { self.swapchain_loader.acquire_next_image(self.swapchain, u64::MAX, self.image_available_semaphores[frame], vk::Fence::null()) };
        let image_index = match acquire_result {
            Ok((idx, _)) => idx as usize,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => { self.recreate_swapchain(window)?; return Ok(()); }
            Err(e) => return Err(anyhow!("acquire_next_image: {e:?}")),
        };

        let command_buffer = self.command_buffers[frame];
        unsafe { self.device.reset_fences(&[fence])?; }

        // === Выбор 8 ближайших ламп ===
        let cam_pos = camera.position;
        let mut ranked: Vec<(f32, usize)> = self.light_list.iter().enumerate()
            .map(|(i, (p, _, _))| (p.distance_squared(cam_pos), i))
            .collect();
        ranked.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut light_buffer_data = buffer::LightBuffer { lights: [buffer::LightData::default(); buffer::MAX_LIGHTS] };
        for (slot, (_, idx)) in ranked.iter().take(buffer::MAX_LIGHTS).enumerate() {
            let (pos, color, radius) = self.light_list[*idx];
            light_buffer_data.lights[slot] = buffer::LightData {
                position_radius: [pos.x, pos.y, pos.z, radius],
                color_intensity: [color.x, color.y, color.z, 1.0],
            };
        }
        buffer::update_light_buffer(&self.device, self.light_buffer_memory, &light_buffer_data)?;
        // SHADOW PASS
        // Свет ВНУТРИ коридора, чуть под потолком (y=0 в корридоре)
        let light_pos = glam::Vec3::new(10.0, -1.5, 50.0);
        let light_target = glam::Vec3::new(10.0, -10.0, 50.0);
        let light_view = glam::Mat4::look_at_rh(light_pos, light_target, glam::Vec3::Z);
        let mut light_proj = glam::Mat4::orthographic_rh(-50.0, 50.0, -50.0, 50.0, 0.1, 100.0);
        light_proj.y_axis.y *= -1.0;
        let light_vp = light_proj * light_view;
        let light_vp_bytes: &[u8] = unsafe { std::slice::from_raw_parts(&light_vp as *const glam::Mat4 as *const u8, 64) };

        self.visible_for_light.clear();
        let light_planes = extract_frustum_planes(light_vp);
        for (i, (buf, _, count)) in self.vertex_buffers.iter().enumerate() {
            if frustum_aabb_intersect(&light_planes, &self.chunk_bounds[i]) {
                self.visible_for_light.push((*buf, *count));
            }
        }

        let shadow_cmd = self.shadow_cmds[frame]; // <-- ИСПОЛЬЗУЕМ frame-based буфер

        commands::record_shadow_pass(
            &self.device, shadow_cmd, self.shadow_render_pass, self.shadow_framebuffer,
            self.shadow_pipeline, self.shadow_pipeline_layout, &self.visible_for_light, light_vp_bytes
        )?;

        let shadow_cmd_arr = [shadow_cmd];
        let shadow_submit = vk::SubmitInfo::default().command_buffers(&shadow_cmd_arr);
        unsafe { self.device.queue_submit(self.graphics_queue, &[shadow_submit], vk::Fence::null())?; }
        // FRAME
        let model = glam::Mat4::IDENTITY;
        let view = camera.view_matrix();
        let mut proj = glam::Mat4::perspective_rh(fov.to_radians(), self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32, 0.1, 100.0);
        proj.y_axis.y *= -1.0;
        let mvp = proj * view * model;

        let mut push_data = [0u8; 208];
        push_data[0..64].copy_from_slice(unsafe { std::slice::from_raw_parts(mvp.to_cols_array().as_ptr() as *const u8, 64) });
        push_data[64..128].copy_from_slice(light_vp_bytes);
        push_data[128..144].copy_from_slice(unsafe { std::slice::from_raw_parts(&glam::Vec4::new(10.0, -1.5, 50.0, 10000.0) as *const glam::Vec4 as *const u8, 16) });
        push_data[144..160].copy_from_slice(unsafe { std::slice::from_raw_parts(&glam::Vec4::new(1.0, 1.0, 1.0, 0.15) as *const glam::Vec4 as *const u8, 16) });
        push_data[160..176].copy_from_slice(unsafe { std::slice::from_raw_parts(&glam::Vec4::new(camera.position.x, camera.position.y, camera.position.z, 1.0) as *const glam::Vec4 as *const u8, 16) });
        push_data[176..192].copy_from_slice(unsafe { std::slice::from_raw_parts(&glam::Vec4::new(320.0, 240.0, debug_mode as f32, 0.0) as *const glam::Vec4 as *const u8, 16) });
        // Единый материал для всей геометрии (улучшим позже через per-chunk данные)
        let material_params = glam::Vec4::new(0.0, 0.95, 0.0, 0.0);  // roughness 0.95 вместо 0.8
        push_data[192..208].copy_from_slice(unsafe {
            std::slice::from_raw_parts(&material_params as *const glam::Vec4 as *const u8, 16)
        });

        let proj_view = proj * view;
        let planes = extract_frustum_planes(proj_view);
        self.visible_for_camera.clear();
        for (i, (buf, _, count)) in self.vertex_buffers.iter().enumerate() {
            if frustum_aabb_intersect(&planes, &self.chunk_bounds[i]) {
                self.visible_for_camera.push((*buf, *count));
            }
        }

        let bloom_extent = vk::Extent2D {
            width: self.swapchain_extent.width.max(1) / 2,
            height: self.swapchain_extent.height.max(1) / 2,
        };

                let mut ssao_push = [0u8; 128];
        let proj_arr = proj.to_cols_array();
        let inv_proj = proj.inverse();
        let inv_proj_arr = inv_proj.to_cols_array();
        unsafe {
            std::ptr::copy_nonoverlapping(proj_arr.as_ptr() as *const u8, ssao_push.as_mut_ptr(), 64);
            std::ptr::copy_nonoverlapping(inv_proj_arr.as_ptr() as *const u8, ssao_push.as_mut_ptr().add(64), 64);
        }
        commands::record_frame(
            &self.device, command_buffer,
            self.hdr_render_pass, self.hdr_framebuffer,
            self.bloom_render_pass, self.bloom_a_framebuffer,
            self.bloom_render_pass, self.bloom_b_framebuffer,
            self.render_pass, self.framebuffers[image_index],
            // SSAO
            self.ssao_render_pass, self.ssao_a_framebuffer, self.ssao_b_framebuffer,
            self.ssao_pipeline, self.ssao_layout, self.ssao_desc,
            self.ssao_blur_pipeline, self.ssao_blur_layout,
            self.ssao_blur_desc_a, self.ssao_blur_desc_b,
            &ssao_push,
            self.swapchain_extent, bloom_extent,
            HDR_CLEAR, SWAPCHAIN_CLEAR,
            self.pipeline, self.pipeline_layout, self.world_descriptor_set,
            &self.visible_for_camera, &push_data,
            self.bloom_bright_pipeline, self.bloom_bright_layout, self.bloom_bright_desc,
            self.blur_pipeline, self.blur_layout,
            self.blur_desc_a, self.blur_desc_b,
            self.post_pipeline, self.post_pipeline_layout, self.post_descriptor_set,
            self.ui_pipeline, self.ui_pipeline_layout,
            self.ui_vertex_buffer, self.ui_vertex_count, self.ui_descriptor_set,
        )?;

        let wait_semaphores = [self.image_available_semaphores[frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished_semaphores[image_index]];
        let cmd_buffers = [command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores).wait_dst_stage_mask(&wait_stages)
            .command_buffers(&cmd_buffers).signal_semaphores(&signal_semaphores);
        unsafe { self.device.queue_submit(self.graphics_queue, &[submit_info], fence)?; }

        let swapchains = [self.swapchain];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores).swapchains(&swapchains).image_indices(&image_indices);
        let present_result = unsafe { self.swapchain_loader.queue_present(self.present_queue, &present_info) };
        match present_result {
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => { self.recreate_swapchain(window)?; }
            Ok(false) => {}
            Err(e) => return Err(anyhow!("queue_present: {e:?}")),
        }

        self.current_frame = (frame + 1) % MAX_FRAMES_IN_FLIGHT;
        Ok(())
    }

    pub fn recreate_swapchain(&mut self, window: &Window) -> Result<()> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 { return Ok(()); }

        unsafe { self.device.device_wait_idle()?; }

        unsafe {
            self.device.destroy_framebuffer(self.hdr_framebuffer, None);
            self.device.destroy_image_view(self.hdr_view, None);
            self.device.destroy_image(self.hdr_image, None);
            self.device.free_memory(self.hdr_memory, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_memory, None);
        }
        self.destroy_swapchain_resources();

        let indices = device::find_queue_families(&self.instance, &self.surface_loader, self.surface, self.physical_device)?;
        let (loader, swapchain, format, extent, images) = swapchain::create_swapchain(&self.instance, &self.device, self.physical_device, &self.surface_loader, self.surface, window, &indices)?;
                let image_views = swapchain::create_image_views(&self.device, &images, format)?;
        let (depth_image, depth_memory, depth_image_view, new_depth_sampler) = depth::create_depth_resources(&self.instance, &self.device, self.physical_device, extent)?;
        // depth_sampler не зависит от размера — сохраняем старый, а новый сразу уничтожаем
        unsafe { self.device.destroy_sampler(new_depth_sampler, None); }
        let framebuffers = render_pass::create_framebuffers(&self.device, self.render_pass, &image_views, depth_image_view, extent)?;

        let (hdr_image, hdr_memory, hdr_view) = create_hdr_image(&self.instance, &self.device, self.physical_device, extent)?;
        let hdr_framebuffer = render_pass::create_framebuffers(&self.device, self.hdr_render_pass, &[hdr_view], depth_image_view, extent)?[0];

        for &sem in &self.render_finished_semaphores { unsafe { self.device.destroy_semaphore(sem, None) }; }
        let render_finished_semaphores = synchronization::create_semaphores(&self.device, images.len())?;

        // Уничтожаем старые bloom-ресурсы
        unsafe {
            self.device.destroy_framebuffer(self.bloom_a_framebuffer, None);
            self.device.destroy_framebuffer(self.bloom_b_framebuffer, None);
            self.device.destroy_image_view(self.bloom_a_view, None);
            self.device.destroy_image(self.bloom_a_image, None);
            self.device.free_memory(self.bloom_a_memory, None);
            self.device.destroy_image_view(self.bloom_b_view, None);
            self.device.destroy_image(self.bloom_b_image, None);
            self.device.free_memory(self.bloom_b_memory, None);
        }

        let bloom_extent = vk::Extent2D {
            width: extent.width.max(1) / 2,
            height: extent.height.max(1) / 2,
        };
        let (bloom_a_image, bloom_a_memory, bloom_a_view) =
            texture::create_hdr_offscreen(&self.instance, &self.device, self.physical_device, bloom_extent.width, bloom_extent.height)?;
        let (bloom_b_image, bloom_b_memory, bloom_b_view) =
            texture::create_hdr_offscreen(&self.instance, &self.device, self.physical_device, bloom_extent.width, bloom_extent.height)?;
        let bloom_a_framebuffer = render_pass::create_offscreen_framebuffer(&self.device, self.bloom_render_pass, bloom_a_view, bloom_extent)?;
        let bloom_b_framebuffer = render_pass::create_offscreen_framebuffer(&self.device, self.bloom_render_pass, bloom_b_view, bloom_extent)?;

        descriptor::update_single_texture(&self.device, self.blur_desc_a, self.bloom_a_sampler, bloom_a_view);
        descriptor::update_single_texture(&self.device, self.blur_desc_b, self.bloom_a_sampler, bloom_b_view);
        // ВОТ ЭТА СТРОКА БЫЛА ПРОПУЩЕНА — без неё bright pass читает уничтоженный hdr_view
        descriptor::update_single_texture(&self.device, self.bloom_bright_desc, self.hdr_sampler, hdr_view);

        self.bloom_a_image = bloom_a_image;
        self.bloom_a_memory = bloom_a_memory;
        self.bloom_a_view = bloom_a_view;
        self.bloom_a_framebuffer = bloom_a_framebuffer;
        self.bloom_b_image = bloom_b_image;
        self.bloom_b_memory = bloom_b_memory;
        self.bloom_b_view = bloom_b_view;
        self.bloom_b_framebuffer = bloom_b_framebuffer;

                // Пересоздать SSAO
        unsafe {
            self.device.destroy_framebuffer(self.ssao_a_framebuffer, None);
            self.device.destroy_framebuffer(self.ssao_b_framebuffer, None);
            self.device.destroy_image_view(self.ssao_a_view, None);
            self.device.destroy_image(self.ssao_a_image, None);
            self.device.free_memory(self.ssao_a_memory, None);
            self.device.destroy_image_view(self.ssao_b_view, None);
            self.device.destroy_image(self.ssao_b_image, None);
            self.device.free_memory(self.ssao_b_memory, None);
        }
        let (ssao_a_image, ssao_a_memory, ssao_a_view) =
            texture::create_ssao_offscreen(&self.instance, &self.device, self.physical_device, bloom_extent.width, bloom_extent.height)?;
        let (ssao_b_image, ssao_b_memory, ssao_b_view) =
            texture::create_ssao_offscreen(&self.instance, &self.device, self.physical_device, bloom_extent.width, bloom_extent.height)?;
        let ssao_a_framebuffer = render_pass::create_offscreen_framebuffer(&self.device, self.ssao_render_pass, ssao_a_view, bloom_extent)?;
        let ssao_b_framebuffer = render_pass::create_offscreen_framebuffer(&self.device, self.ssao_render_pass, ssao_b_view, bloom_extent)?;
        descriptor::update_single_texture(&self.device, self.ssao_blur_desc_a, self.ssao_sampler, ssao_a_view);
        descriptor::update_single_texture(&self.device, self.ssao_blur_desc_b, self.ssao_sampler, ssao_b_view);
        // Обновите depth в SSAO descriptor после пересоздания depth
        descriptor::update_single_texture_with_layout(
            &self.device, self.ssao_desc, self.depth_sampler, depth_image_view,
            vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL,
        );
        descriptor::update_composite_set(&self.device, self.post_descriptor_set, self.hdr_sampler, hdr_view, self.bloom_a_sampler, bloom_a_view, self.ssao_sampler, ssao_a_view);

        self.ssao_a_image = ssao_a_image;
        self.ssao_a_memory = ssao_a_memory;
        self.ssao_a_view = ssao_a_view;
        self.ssao_a_framebuffer = ssao_a_framebuffer;
        self.ssao_b_image = ssao_b_image;
        self.ssao_b_memory = ssao_b_memory;
        self.ssao_b_view = ssao_b_view;
        self.ssao_b_framebuffer = ssao_b_framebuffer;

        self.swapchain_loader = loader; self.swapchain = swapchain; self.swapchain_format = format; self.swapchain_extent = extent;
        self.swapchain_images = images; self.swapchain_image_views = image_views; self.depth_image = depth_image;
        self.depth_memory = depth_memory; self.depth_image_view = depth_image_view; self.framebuffers = framebuffers;
        self.hdr_image = hdr_image; self.hdr_memory = hdr_memory; self.hdr_view = hdr_view; self.hdr_framebuffer = hdr_framebuffer;
        self.render_finished_semaphores = render_finished_semaphores;

        println!("Swapchain recreated: {}x{}", extent.width, extent.height);
        Ok(())
    }

    fn destroy_swapchain_resources(&mut self) {
        unsafe {
            for &fb in &self.framebuffers { self.device.destroy_framebuffer(fb, None); }
            for &iv in &self.swapchain_image_views { self.device.destroy_image_view(iv, None); }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }

    pub fn get_vertex_count(&self) -> u32 { self.total_vertices }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.device.destroy_sampler(self.font_sampler, None);
            self.device.destroy_image_view(self.font_view, None);
            self.device.destroy_image(self.font_image, None);
            self.device.free_memory(self.font_memory, None);
            self.device.destroy_descriptor_pool(self.ui_descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.ui_descriptor_layout, None);
            self.device.destroy_pipeline(self.ui_pipeline, None);
            self.device.destroy_pipeline_layout(self.ui_pipeline_layout, None);
            if self.ui_vertex_buffer != vk::Buffer::null() {
                self.device.destroy_buffer(self.ui_vertex_buffer, None);
                self.device.free_memory(self.ui_vertex_memory, None);
            }

            // Bloom
            self.device.destroy_pipeline(self.bloom_bright_pipeline, None);
            self.device.destroy_pipeline_layout(self.bloom_bright_layout, None);
            self.device.destroy_descriptor_pool(self.bloom_bright_desc_pool, None);
            self.device.destroy_descriptor_set_layout(self.bloom_bright_desc_layout, None);
            self.device.destroy_pipeline(self.blur_pipeline, None);
            self.device.destroy_pipeline_layout(self.blur_layout, None);
            self.device.destroy_descriptor_pool(self.blur_desc_pool, None);
            self.device.destroy_descriptor_set_layout(self.blur_desc_layout, None);
            self.device.destroy_sampler(self.bloom_a_sampler, None);
            self.device.destroy_sampler(self.bloom_b_sampler, None); // <-- нужно сохранить bloom_b_sampler!
            self.device.destroy_framebuffer(self.bloom_a_framebuffer, None);
            self.device.destroy_framebuffer(self.bloom_b_framebuffer, None);
            self.device.destroy_image_view(self.bloom_a_view, None);
            self.device.destroy_image(self.bloom_a_image, None);
            self.device.free_memory(self.bloom_a_memory, None);
            self.device.destroy_image_view(self.bloom_b_view, None);
            self.device.destroy_image(self.bloom_b_image, None);
            self.device.free_memory(self.bloom_b_memory, None);
            self.device.destroy_render_pass(self.bloom_render_pass, None);

            // SSAO
            self.device.destroy_sampler(self.depth_sampler, None);
            self.device.destroy_pipeline(self.ssao_pipeline, None);
            self.device.destroy_pipeline_layout(self.ssao_layout, None);
            self.device.destroy_descriptor_pool(self.ssao_desc_pool, None);
            self.device.destroy_descriptor_set_layout(self.ssao_desc_layout, None);
            self.device.destroy_pipeline(self.ssao_blur_pipeline, None);
            self.device.destroy_pipeline_layout(self.ssao_blur_layout, None);
            self.device.destroy_descriptor_pool(self.ssao_blur_pool, None);
            self.device.destroy_sampler(self.ssao_sampler, None);
            self.device.destroy_framebuffer(self.ssao_a_framebuffer, None);
            self.device.destroy_framebuffer(self.ssao_b_framebuffer, None);
            self.device.destroy_render_pass(self.ssao_render_pass, None);
            self.device.destroy_image_view(self.ssao_a_view, None);
            self.device.destroy_image(self.ssao_a_image, None);
            self.device.free_memory(self.ssao_a_memory, None);
            self.device.destroy_image_view(self.ssao_b_view, None);
            self.device.destroy_image(self.ssao_b_image, None);
            self.device.free_memory(self.ssao_b_memory, None);
            self.device.destroy_descriptor_set_layout(self.ssao_blur_desc_layout, None);

            self.device.destroy_pipeline(self.post_pipeline, None);
            self.device.destroy_pipeline_layout(self.post_pipeline_layout, None);
            self.device.destroy_descriptor_pool(self.post_descriptor_pool, None);
            self.device.destroy_descriptor_set_layout(self.post_descriptor_layout, None);
            self.device.destroy_sampler(self.hdr_sampler, None);
            self.device.destroy_image_view(self.hdr_view, None);
            self.device.destroy_image(self.hdr_image, None);
            self.device.free_memory(self.hdr_memory, None);
            self.device.destroy_framebuffer(self.hdr_framebuffer, None);
            self.device.destroy_render_pass(self.hdr_render_pass, None);

            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            for (buf, mem, _) in &self.vertex_buffers {
                self.device.destroy_buffer(*buf, None);
                self.device.free_memory(*mem, None);
            }

            for &fb in &self.framebuffers { self.device.destroy_framebuffer(fb, None); }
            self.device.destroy_render_pass(self.render_pass, None);
            for &iv in &self.swapchain_image_views { self.device.destroy_image_view(iv, None); }
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
            self.device.destroy_image_view(self.depth_image_view, None);
            self.device.destroy_image(self.depth_image, None);
            self.device.free_memory(self.depth_memory, None);

            for &sem in &self.image_available_semaphores { self.device.destroy_semaphore(sem, None); }
            for &sem in &self.render_finished_semaphores { self.device.destroy_semaphore(sem, None); }
            for &fence in &self.in_flight_fences { self.device.destroy_fence(fence, None); }

            self.device.destroy_sampler(self.shadow_sampler, None);
            self.device.destroy_image_view(self.shadow_view, None);
            self.device.destroy_image(self.shadow_image, None);
            self.device.free_memory(self.shadow_memory, None);
            self.device.destroy_render_pass(self.shadow_render_pass, None);
            self.device.destroy_framebuffer(self.shadow_framebuffer, None);
            self.device.destroy_pipeline(self.shadow_pipeline, None);
            self.device.destroy_pipeline_layout(self.shadow_pipeline_layout, None);

            self.device.destroy_command_pool(self.command_pool, None);

            // IBL
            self.device.destroy_sampler(self.ibl_irradiance_sampler, None);
            self.device.destroy_image_view(self.ibl_irradiance_view, None);
            self.device.destroy_image(self.ibl_irradiance_image, None);
            self.device.free_memory(self.ibl_irradiance_memory, None);
            self.device.destroy_sampler(self.ibl_prefiltered_sampler, None);
            self.device.destroy_image_view(self.ibl_prefiltered_view, None);
            self.device.destroy_image(self.ibl_prefiltered_image, None);
            self.device.free_memory(self.ibl_prefiltered_memory, None);

            self.device.destroy_sampler(self.world_texture_sampler, None);
            self.device.destroy_image_view(self.world_texture_view, None);
            self.device.destroy_image(self.world_texture_image, None);
            self.device.free_memory(self.world_texture_memory, None);
            self.device.destroy_descriptor_pool(self.world_descriptor_pool, None);
            self.device.destroy_buffer(self.light_buffer, None);
            self.device.free_memory(self.light_buffer_memory, None);
            self.device.destroy_descriptor_set_layout(self.world_descriptor_layout, None);

            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

fn extract_frustum_planes(m: glam::Mat4) -> [glam::Vec4; 6] {
    let m = m.to_cols_array_2d();
    let mut planes = [glam::Vec4::ZERO; 6];
    planes[0] = glam::Vec4::new(m[0][3] + m[0][0], m[1][3] + m[1][0], m[2][3] + m[2][0], m[3][3] + m[3][0]);
    planes[1] = glam::Vec4::new(m[0][3] - m[0][0], m[1][3] - m[1][0], m[2][3] - m[2][0], m[3][3] - m[3][0]);
    planes[2] = glam::Vec4::new(m[0][3] + m[0][1], m[1][3] + m[1][1], m[2][3] + m[2][1], m[3][3] + m[3][1]);
    planes[3] = glam::Vec4::new(m[0][3] - m[0][1], m[1][3] - m[1][1], m[2][3] - m[2][1], m[3][3] - m[3][1]);
    planes[4] = glam::Vec4::new(m[0][3] + m[0][2], m[1][3] + m[1][2], m[2][3] + m[2][2], m[3][3] + m[3][2]);
    planes[5] = glam::Vec4::new(m[0][3] - m[0][2], m[1][3] - m[1][2], m[2][3] - m[2][2], m[3][3] - m[3][2]);
    for p in planes.iter_mut() { *p = p.normalize(); }
    planes
}

fn frustum_aabb_intersect(planes: &[glam::Vec4; 6], aabb: &crate::world::Collider) -> bool {
    for p in planes {
        let center = (aabb.min + aabb.max) * 0.5;
        let extents = (aabb.max - aabb.min) * 0.5;
        let r = extents.x * p.x.abs() + extents.y * p.y.abs() + extents.z * p.z.abs();
        let s = p.dot(center.extend(1.0));
        if r + s < 0.0 { return false; }
    }
    true
}
