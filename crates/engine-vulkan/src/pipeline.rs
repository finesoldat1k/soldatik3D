use anyhow::Result;
use ash::{Device, vk};
use std::ffi::CStr;

/// Owned graphics pipeline and its pipeline layout.
pub struct Pipeline {
    device: Device,
    pub handle: vk::Pipeline,
    pub layout: vk::PipelineLayout,
}

impl Pipeline {
    fn new(device: &Device, handle: vk::Pipeline, layout: vk::PipelineLayout) -> Self {
        Self {
            device: device.clone(),
            handle,
            layout,
        }
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.handle, None);
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

const MAIN_NAME: &CStr = c"main";

pub fn world(
    device: &Device,
    render_pass: vk::RenderPass,
    descriptor_layout: vk::DescriptorSetLayout,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> Result<Pipeline> {
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(engine_core::Vertex::STRIDE)
        .input_rate(vk::VertexInputRate::VERTEX);
    let attributes = [
        attribute(
            0,
            0,
            vk::Format::R32G32B32_SFLOAT,
            engine_core::Vertex::POSITION_OFFSET,
        ),
        attribute(
            1,
            0,
            vk::Format::R32G32B32_SFLOAT,
            engine_core::Vertex::COLOR_OFFSET,
        ),
        attribute(
            2,
            0,
            vk::Format::R32G32B32_SFLOAT,
            engine_core::Vertex::NORMAL_OFFSET,
        ),
        attribute(
            3,
            0,
            vk::Format::R32G32_SFLOAT,
            engine_core::Vertex::UV_OFFSET,
        ),
        attribute(
            4,
            0,
            vk::Format::R32_SFLOAT,
            engine_core::Vertex::EMISSION_OFFSET,
        ),
    ];
    let bindings = [binding];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);
    let push = push_range(
        vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        208,
    );
    let layout = pipeline_layout(device, &[descriptor_layout], &[push])?;
    let pipeline = create(
        device,
        render_pass,
        layout,
        vertex,
        fragment,
        &vertex_input,
        true,
        vk::CullModeFlags::NONE,
        208,
    )?;
    Ok(Pipeline::new(device, pipeline, layout))
}

pub fn shadow(
    device: &Device,
    render_pass: vk::RenderPass,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> Result<Pipeline> {
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(engine_core::Vertex::STRIDE)
        .input_rate(vk::VertexInputRate::VERTEX);
    let attributes = [attribute(
        0,
        0,
        vk::Format::R32G32B32_SFLOAT,
        engine_core::Vertex::POSITION_OFFSET,
    )];
    let bindings = [binding];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);
    let push = push_range(vk::ShaderStageFlags::VERTEX, 64);
    let layout = pipeline_layout(device, &[], &[push])?;
    // Front-face culling reduces shadow acne for closed interior geometry.
    let pipeline = create(
        device,
        render_pass,
        layout,
        vertex,
        fragment,
        &vertex_input,
        true,
        vk::CullModeFlags::FRONT,
        64,
    )?;
    Ok(Pipeline::new(device, pipeline, layout))
}

pub fn post_process(
    device: &Device,
    render_pass: vk::RenderPass,
    descriptor_layout: vk::DescriptorSetLayout,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> Result<Pipeline> {
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let layout = pipeline_layout(device, &[descriptor_layout], &[])?;
    let pipeline = create(
        device,
        render_pass,
        layout,
        vertex,
        fragment,
        &vertex_input,
        false,
        vk::CullModeFlags::NONE,
        0,
    )?;
    Ok(Pipeline::new(device, pipeline, layout))
}

pub fn ui(
    device: &Device,
    render_pass: vk::RenderPass,
    descriptor_layout: vk::DescriptorSetLayout,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> Result<Pipeline> {
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(engine_core::UiVertex::STRIDE)
        .input_rate(vk::VertexInputRate::VERTEX);
    let attributes = [
        attribute(0, 0, vk::Format::R32G32_SFLOAT, 0),
        attribute(1, 0, vk::Format::R32G32B32A32_SFLOAT, 8),
        attribute(2, 0, vk::Format::R32G32_SFLOAT, 24),
    ];
    let bindings = [binding];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&bindings)
        .vertex_attribute_descriptions(&attributes);
    let layout = pipeline_layout(device, &[descriptor_layout], &[])?;

    let stages = stages(vertex, fragment);
    let assembly = assembly();
    let viewport = viewport_state();
    let raster = rasterizer(vk::CullModeFlags::NONE);
    let multisample = multisample();
    let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);
    let blend_attachments = [blend_attachment];
    let blending = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
    let depth = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&assembly)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&blending)
        .depth_stencil_state(&depth)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipeline =
        unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) }
            .map_err(|(_, error)| anyhow::anyhow!("UI pipeline creation failed: {error:?}"))?[0];
    Ok(Pipeline::new(device, pipeline, layout))
}

pub fn offscreen(
    device: &Device,
    render_pass: vk::RenderPass,
    descriptor_layout: vk::DescriptorSetLayout,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    push_constant_size: u32,
) -> Result<Pipeline> {
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
    let push_ranges: Vec<vk::PushConstantRange> = if push_constant_size > 0 {
        vec![push_range(
            vk::ShaderStageFlags::FRAGMENT,
            push_constant_size,
        )]
    } else {
        Vec::new()
    };
    let layout = pipeline_layout(device, &[descriptor_layout], &push_ranges)?;
    let pipeline = create(
        device,
        render_pass,
        layout,
        vertex,
        fragment,
        &vertex_input,
        false,
        vk::CullModeFlags::NONE,
        push_constant_size,
    )?;
    Ok(Pipeline::new(device, pipeline, layout))
}

fn attribute(
    location: u32,
    binding: u32,
    format: vk::Format,
    offset: u32,
) -> vk::VertexInputAttributeDescription {
    vk::VertexInputAttributeDescription::default()
        .location(location)
        .binding(binding)
        .format(format)
        .offset(offset)
}

fn push_range(stage_flags: vk::ShaderStageFlags, size: u32) -> vk::PushConstantRange {
    vk::PushConstantRange::default()
        .stage_flags(stage_flags)
        .offset(0)
        .size(size)
}

fn pipeline_layout(
    device: &Device,
    set_layouts: &[vk::DescriptorSetLayout],
    push_ranges: &[vk::PushConstantRange],
) -> Result<vk::PipelineLayout> {
    let info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(set_layouts)
        .push_constant_ranges(push_ranges);
    Ok(unsafe { device.create_pipeline_layout(&info, None)? })
}

fn stages(
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
) -> [vk::PipelineShaderStageCreateInfo<'static>; 2] {
    [
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertex)
            .name(MAIN_NAME),
        vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragment)
            .name(MAIN_NAME),
    ]
}

fn assembly() -> vk::PipelineInputAssemblyStateCreateInfo<'static> {
    vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
}

fn viewport_state() -> vk::PipelineViewportStateCreateInfo<'static> {
    vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1)
}

fn rasterizer(cull_mode: vk::CullModeFlags) -> vk::PipelineRasterizationStateCreateInfo<'static> {
    vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(cull_mode)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false)
}

fn multisample() -> vk::PipelineMultisampleStateCreateInfo<'static> {
    vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
}

#[allow(clippy::too_many_arguments)]
fn create(
    device: &Device,
    render_pass: vk::RenderPass,
    layout: vk::PipelineLayout,
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    vertex_input: &vk::PipelineVertexInputStateCreateInfo<'_>,
    depth_test: bool,
    cull_mode: vk::CullModeFlags,
    _push_constant_size: u32,
) -> Result<vk::Pipeline> {
    let stages = stages(vertex, fragment);
    let assembly = assembly();
    let viewport = viewport_state();
    let raster = rasterizer(cull_mode);
    let multisample = multisample();
    let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(false);
    let blend_attachments = [blend_attachment];
    let blending = vk::PipelineColorBlendStateCreateInfo::default().attachments(&blend_attachments);
    let depth = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(depth_test)
        .depth_write_enable(depth_test)
        .depth_compare_op(vk::CompareOp::LESS);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(vertex_input)
        .input_assembly_state(&assembly)
        .viewport_state(&viewport)
        .rasterization_state(&raster)
        .multisample_state(&multisample)
        .color_blend_state(&blending)
        .depth_stencil_state(&depth)
        .dynamic_state(&dynamic)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0);
    let pipelines =
        unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) }
            .map_err(|(_, error)| {
                anyhow::anyhow!("graphics pipeline creation failed: {error:?}")
            })?;
    Ok(pipelines[0])
}
