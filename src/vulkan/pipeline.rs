use anyhow::Result;
use ash::{vk, Device};
use std::ffi::CStr;

pub fn create_pipeline(
    device: &Device,
    render_pass: vk::RenderPass,
    _extent: vk::Extent2D,
    vert_module: vk::ShaderModule,
    frag_module: vk::ShaderModule,
     descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    let binding_desc = vk::VertexInputBindingDescription::default().binding(0).stride(48).input_rate(vk::VertexInputRate::VERTEX);
    let attr_descs = [
        vk::VertexInputAttributeDescription::default().binding(0).location(0).format(vk::Format::R32G32B32_SFLOAT).offset(0),
        vk::VertexInputAttributeDescription::default().binding(0).location(1).format(vk::Format::R32G32B32_SFLOAT).offset(12),
        vk::VertexInputAttributeDescription::default().binding(0).location(2).format(vk::Format::R32G32B32_SFLOAT).offset(24),
        vk::VertexInputAttributeDescription::default().binding(0).location(3).format(vk::Format::R32G32_SFLOAT).offset(36),
        vk::VertexInputAttributeDescription::default().binding(0).location(4).format(vk::Format::R32_SFLOAT).offset(44), // Emission
    ];
    let binding_descriptions = [binding_desc];
    let attribute_descriptions = attr_descs;

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&binding_descriptions)
        .vertex_attribute_descriptions(&attribute_descriptions);

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST).primitive_restart_enable(false);

    // ИСПРАВЛЕНИЕ: Убрали POLYGON_MODE
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE) // <-- ВКЛЮЧАЕМ ОТСЕЧЕНИЕ ЗАДНИХ ГРАНЕЙ
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE) // <-- Указываем, что "лицо" — это по часовой стрелке
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default().sample_shading_enable(false).rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default().color_write_mask(vk::ColorComponentFlags::RGBA).blend_enable(false);
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default().logic_op_enable(false).attachments(&color_blend_attachments);

    let push_constant_range = vk::PushConstantRange::default()
        .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
        .offset(0)
        .size(208);

    let push_constant_ranges = [push_constant_range];
    let set_layouts = [descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_constant_ranges);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vert_module).name(CStr::from_bytes_with_nul(b"main\0")?),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(frag_module).name(CStr::from_bytes_with_nul(b"main\0")?),
    ];

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::LESS).depth_bounds_test_enable(false).stencil_test_enable(false);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state_info)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .depth_stencil_state(&depth_stencil)
        .subpass(0);

    let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) };
    let pipeline = pipelines.map_err(|(_, e)| anyhow::anyhow!("Pipeline creation failed: {:?}", e))?[0];
    Ok((pipeline, pipeline_layout))
}

pub fn create_ui_pipeline(
    device: &Device,
    render_pass: vk::RenderPass,
    vert_module: vk::ShaderModule,
    frag_module: vk::ShaderModule,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    let binding_desc = vk::VertexInputBindingDescription::default().binding(0).stride(std::mem::size_of::<crate::ui::UiVertex>() as u32).input_rate(vk::VertexInputRate::VERTEX);
    let attr_descs = [
        vk::VertexInputAttributeDescription::default().binding(0).location(0).format(vk::Format::R32G32_SFLOAT).offset(0),
        vk::VertexInputAttributeDescription::default().binding(0).location(1).format(vk::Format::R32G32B32A32_SFLOAT).offset(8),
        vk::VertexInputAttributeDescription::default().binding(0).location(2).format(vk::Format::R32G32_SFLOAT).offset(24), // UV <-- НОВОЕ
    ];
    let binding_descriptions = [binding_desc];
    let attribute_descriptions = attr_descs;

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&binding_descriptions).vertex_attribute_descriptions(&attribute_descriptions);
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default().polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE).line_width(1.0);
    let multisampling = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA)
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD);

    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachments);

    let set_layouts = [descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

    let shader_stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vert_module).name(CStr::from_bytes_with_nul(b"main\0")?),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(frag_module).name(CStr::from_bytes_with_nul(b"main\0")?),
    ];

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default().depth_test_enable(false).depth_write_enable(false);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterizer)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state_info)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .depth_stencil_state(&depth_stencil)
        .subpass(0);

    let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) };
    let pipeline = pipelines.map_err(|(_, e)| anyhow::anyhow!("UI Pipeline creation failed: {:?}", e))?[0];
    Ok((pipeline, pipeline_layout))
}

pub fn create_post_process_pipeline(
    device: &Device, render_pass: vk::RenderPass,
    vert_module: vk::ShaderModule, frag_module: vk::ShaderModule,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    
    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .line_width(1.0);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    
    // ИСПРАВЛЕНИЕ: сохраняем в переменную
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA).blend_enable(false);
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default().attachments(&color_blend_attachments);
    
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default().depth_test_enable(false).depth_write_enable(false);
    
    let set_layouts = [descriptor_set_layout];
    let layout_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };
    
    let stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vert_module).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(frag_module).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
    ];
    
    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages).vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly).viewport_state(&viewport_state)
        .rasterization_state(&rasterizer).multisample_state(&multisampling)
        .color_blend_state(&color_blending).depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic_state).layout(pipeline_layout)
        .render_pass(render_pass).subpass(0);
    
    let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None) };
    let pipeline = pipelines.map_err(|(_, e)| anyhow::anyhow!("Post-process pipeline failed: {:?}", e))?[0];
    Ok((pipeline, pipeline_layout))
}

// Общий пайплайн для offscreen bloom-проходов (bright / blur)
pub fn create_offscreen_pipeline(
    device: &Device, render_pass: vk::RenderPass,
    vert_module: vk::ShaderModule, frag_module: vk::ShaderModule,
    descriptor_set_layout: vk::DescriptorSetLayout,
    push_constant_size: u32,
) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default();
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1).scissor_count(1);

    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL).cull_mode(vk::CullModeFlags::NONE)
        .line_width(1.0);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .color_write_mask(vk::ColorComponentFlags::RGBA).blend_enable(false);
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .attachments(&color_blend_attachments);

    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false).depth_write_enable(false);

    let set_layouts = [descriptor_set_layout];

    let push_ranges: Vec<vk::PushConstantRange> = if push_constant_size > 0 {
        vec![vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .offset(0).size(push_constant_size)]
    } else { Vec::new() };

    let layout_info = vk::PipelineLayoutCreateInfo::default()
        .set_layouts(&set_layouts)
        .push_constant_ranges(&push_ranges);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

    let stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
    ];

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages).vertex_input_state(&vertex_input_info)
        .input_assembly_state(&input_assembly).viewport_state(&viewport_state)
        .rasterization_state(&rasterizer).multisample_state(&multisampling)
        .color_blend_state(&color_blending).depth_stencil_state(&depth_stencil)
        .dynamic_state(&dynamic_state).layout(pipeline_layout)
        .render_pass(render_pass).subpass(0);

    let pipelines = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    };
    let pipeline = pipelines.map_err(|(_, e)| anyhow::anyhow!("Offscreen pipeline failed: {:?}", e))?[0];
    Ok((pipeline, pipeline_layout))
}
