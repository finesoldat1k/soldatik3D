use anyhow::Result;
use ash::{vk, Device};

pub const SHADOW_MAP_SIZE: u32 = 2048;

pub fn create_shadow_resources(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler)> {
    let format = vk::Format::D32_SFLOAT;
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width: SHADOW_MAP_SIZE, height: SHADOW_MAP_SIZE, depth: 1 })
        .mip_levels(1).array_layers(1).format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let image = unsafe { device.create_image(&image_info, None)? };
    let mem_reqs = unsafe { device.get_image_memory_requirements(image) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let memory_type_index = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL))
        .unwrap();
    let alloc_info = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(memory_type_index);
    let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::DEPTH, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    // CLAMP_TO_BORDER с белой рамкой (за пределами карты — "на свету")
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_BORDER)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_BORDER)
        .border_color(vk::BorderColor::FLOAT_OPAQUE_WHITE);
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };
    Ok((image, memory, view, sampler))
}

pub fn create_shadow_render_pass(device: &Device) -> Result<vk::RenderPass> {
    let depth = vk::AttachmentDescription::default()
        .format(vk::Format::D32_SFLOAT).samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR).store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE).stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED).final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
    let depth_ref = vk::AttachmentReference::default().attachment(0).layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    let subpass = vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).depth_stencil_attachment(&depth_ref);
    let dep = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL).dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
        .dst_stage_mask(vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE);
    let attachments = [depth]; let subpasses = [subpass]; let deps = [dep];
    let info = vk::RenderPassCreateInfo::default().attachments(&attachments).subpasses(&subpasses).dependencies(&deps);
    Ok(unsafe { device.create_render_pass(&info, None)? })
}

pub fn create_shadow_framebuffer(device: &Device, render_pass: vk::RenderPass, view: vk::ImageView) -> Result<vk::Framebuffer> {
    let attachments = [view];
    let info = vk::FramebufferCreateInfo::default()
        .render_pass(render_pass).attachments(&attachments)
        .width(SHADOW_MAP_SIZE).height(SHADOW_MAP_SIZE).layers(1);
    Ok(unsafe { device.create_framebuffer(&info, None)? })
}

pub fn create_shadow_pipeline(device: &Device, render_pass: vk::RenderPass, vert: vk::ShaderModule, frag: vk::ShaderModule) -> Result<(vk::Pipeline, vk::PipelineLayout)> {
    let bindings = [vk::VertexInputBindingDescription::default().binding(0).stride(48).input_rate(vk::VertexInputRate::VERTEX)];
    let attrs = [vk::VertexInputAttributeDescription::default().binding(0).location(0).format(vk::Format::R32G32B32_SFLOAT).offset(0)];
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default().vertex_binding_descriptions(&bindings).vertex_attribute_descriptions(&attrs);
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default().topology(vk::PrimitiveTopology::TRIANGLE_LIST);
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state = vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);
    let viewport_state = vk::PipelineViewportStateCreateInfo::default().viewport_count(1).scissor_count(1);
    let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::FRONT) 
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .line_width(1.0)
        .depth_bias_enable(false);
    let multisampling = vk::PipelineMultisampleStateCreateInfo::default().rasterization_samples(vk::SampleCountFlags::TYPE_1);
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default().depth_test_enable(true).depth_write_enable(true).depth_compare_op(vk::CompareOp::LESS);
    let color_blend = vk::PipelineColorBlendStateCreateInfo::default();
    let push_ranges = [vk::PushConstantRange::default().stage_flags(vk::ShaderStageFlags::VERTEX).offset(0).size(64)];
    let layout_info = vk::PipelineLayoutCreateInfo::default().push_constant_ranges(&push_ranges);
    let pipeline_layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };
    let stages = [
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::VERTEX).module(vert).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
        vk::PipelineShaderStageCreateInfo::default().stage(vk::ShaderStageFlags::FRAGMENT).module(frag).name(std::ffi::CStr::from_bytes_with_nul(b"main\0")?),
    ];
    let info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages).vertex_input_state(&vertex_input).input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state).rasterization_state(&rasterizer).multisample_state(&multisampling)
        .color_blend_state(&color_blend).depth_stencil_state(&depth_stencil).dynamic_state(&dynamic_state)
        .layout(pipeline_layout).render_pass(render_pass).subpass(0);
    let pipelines = unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &[info], None) };
    let pipeline = pipelines.map_err(|(_, e)| anyhow::anyhow!("Shadow pipeline failed: {:?}", e))?[0];
    Ok((pipeline, pipeline_layout))
}
