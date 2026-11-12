use anyhow::Result;
use ash::{vk, Device};
use glam::Vec3;

pub fn create_texture(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool, queue: vk::Queue,
    width: u32, height: u32, pixels: &[u8], is_ui: bool,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler)> {
    let format = vk::Format::R8G8B8A8_UNORM;
    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width, height, depth: 1 })
        .mip_levels(1).array_layers(1).format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
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

    let buffer_size = (width * height * 4) as u64;
    let buffer_info = vk::BufferCreateInfo::default().size(buffer_size).usage(vk::BufferUsageFlags::TRANSFER_SRC).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let staging_buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let staging_mem_reqs = unsafe { device.get_buffer_memory_requirements(staging_buffer) };
    let staging_mem_type = (0..mem_props.memory_type_count)
        .find(|&i| (staging_mem_reqs.memory_type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT))
        .unwrap();
    let staging_alloc = vk::MemoryAllocateInfo::default().allocation_size(staging_mem_reqs.size).memory_type_index(staging_mem_type);
    let staging_memory = unsafe { device.allocate_memory(&staging_alloc, None)? };
    unsafe { device.bind_buffer_memory(staging_buffer, staging_memory, 0)?; }

    let data_ptr = unsafe { device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())? } as *mut u8;
    unsafe { data_ptr.copy_from_nonoverlapping(pixels.as_ptr(), pixels.len()); device.unmap_memory(staging_memory); }

    let cmd_alloc = vk::CommandBufferAllocateInfo::default().command_pool(command_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd = unsafe { device.allocate_command_buffers(&cmd_alloc)?[0] };
    let begin = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe { device.begin_command_buffer(cmd, &begin)?; }

    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::UNDEFINED).new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED).dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image).subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .src_access_mask(vk::AccessFlags::empty()).dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
    unsafe { device.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier]); }

    let region = vk::BufferImageCopy::default()
        .buffer_offset(0).buffer_row_length(0).buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 1 })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 }).image_extent(vk::Extent3D { width, height, depth: 1 });
    unsafe { device.cmd_copy_buffer_to_image(cmd, staging_buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]); }

    let barrier2 = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL).new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED).dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image).subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 })
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::SHADER_READ);
    unsafe { device.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[barrier2]); }

    unsafe { device.end_command_buffer(cmd)?; }
    let cmd_buffers = [cmd];
    let submit = vk::SubmitInfo::default().command_buffers(&cmd_buffers);
    unsafe { device.queue_submit(queue, &[submit], vk::Fence::null())?; device.queue_wait_idle(queue)?; device.free_command_buffers(command_pool, &cmd_buffers); }

    unsafe { device.destroy_buffer(staging_buffer, None); device.free_memory(staging_memory, None); }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1 });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    let address_mode = if is_ui { vk::SamplerAddressMode::CLAMP_TO_EDGE } else { vk::SamplerAddressMode::REPEAT };
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::NEAREST).min_filter(vk::Filter::NEAREST)
        .address_mode_u(address_mode).address_mode_v(address_mode);
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

    Ok((image, memory, view, sampler))
}

// Процедурный скайбокс: небо + солнце
fn generate_skybox_pixels(size: u32) -> Vec<f32> {
    let mut pixels = Vec::with_capacity((size * size * 6 * 4) as usize);
    let sun_dir = Vec3::new(0.4, 0.8, 0.5).normalize();

    // 6 граней: +X, -X, +Y, -Y, +Z, -Z
    // (right, up, forward) — координатные оси для каждой грани
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),  // +X
        (Vec3::new(0.0, 0.0, 1.0),  Vec3::new(0.0, -1.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // -X
        (Vec3::new(1.0, 0.0, 0.0),  Vec3::new(0.0, 0.0, 1.0),  Vec3::new(0.0, 1.0, 0.0)),  // +Y
        (Vec3::new(1.0, 0.0, 0.0),  Vec3::new(0.0, 0.0, -1.0), Vec3::new(0.0, -1.0, 0.0)), // -Y
        (Vec3::new(1.0, 0.0, 0.0),  Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),  // +Z
        (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 0.0, -1.0)), // -Z
    ];

    for (right, up, forward) in faces {
        for y in 0..size {
            for x in 0..size {
                let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let dir = (right * u - up * v + forward).normalize();

                // Небо: голубой сверху → тёплый у горизонта → тёмный снизу
                let sky = if dir.y > 0.0 {
                    let t = dir.y;
                    Vec3::new(0.35, 0.55, 0.85) * t + Vec3::new(0.75, 0.65, 0.50) * (1.0 - t)
                } else {
                    Vec3::new(0.08, 0.07, 0.06)
                };

                // Солнце: яркий диск + мягкое свечение
                let sun_dot = dir.dot(sun_dir).max(0.0);
                let sun_disk = sun_dot.powf(2048.0) * 50.0;
                let sun_glow = sun_dot.powf(16.0) * 3.0;

                let color = sky + Vec3::splat(sun_disk + sun_glow);

                pixels.push(color.x);
                pixels.push(color.y);
                pixels.push(color.z);
                pixels.push(1.0);
            }
        }
    }
    pixels
}

pub fn create_skybox_cubemap(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool, queue: vk::Queue,
    size: u32,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler)> {
    let format = vk::Format::R32G32B32A32_SFLOAT;
    let array_layers = 6u32;

    let pixels = generate_skybox_pixels(size);

    let image_info = vk::ImageCreateInfo::default()
        .flags(vk::ImageCreateFlags::CUBE_COMPATIBLE)
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width: size, height: size, depth: 1 })
        .mip_levels(1).array_layers(array_layers).format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let image = unsafe { device.create_image(&image_info, None)? };
    let mem_reqs = unsafe { device.get_image_memory_requirements(image) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let mem_type = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0
            && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL))
        .ok_or_else(|| anyhow::anyhow!("No skybox memory"))?;
    let alloc = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    // Staging buffer (f32 data)
    let buffer_size = (size * size * array_layers * 4 * 4) as u64;
    let buffer_info = vk::BufferCreateInfo::default().size(buffer_size)
        .usage(vk::BufferUsageFlags::TRANSFER_SRC).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let staging_buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let staging_mem_reqs = unsafe { device.get_buffer_memory_requirements(staging_buffer) };
    let staging_mem_type = (0..mem_props.memory_type_count)
        .find(|&i| (staging_mem_reqs.memory_type_bits & (1 << i)) != 0
            && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT))
        .unwrap();
    let staging_alloc = vk::MemoryAllocateInfo::default().allocation_size(staging_mem_reqs.size).memory_type_index(staging_mem_type);
    let staging_memory = unsafe { device.allocate_memory(&staging_alloc, None)? };
    unsafe { device.bind_buffer_memory(staging_buffer, staging_memory, 0)?; }

    let data_ptr = unsafe { device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())? } as *mut f32;
    unsafe {
        data_ptr.copy_from_nonoverlapping(pixels.as_ptr(), pixels.len());
        device.unmap_memory(staging_memory);
    }

    let cmd_alloc = vk::CommandBufferAllocateInfo::default().command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1);
    let cmd = unsafe { device.allocate_command_buffers(&cmd_alloc)?[0] };
    let begin = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe { device.begin_command_buffer(cmd, &begin)?; }

    let barrier1 = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::UNDEFINED).new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED).dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image).subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 6 })
        .src_access_mask(vk::AccessFlags::empty()).dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
    unsafe { device.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TOP_OF_PIPE, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[barrier1]); }

    let region = vk::BufferImageCopy::default()
        .buffer_offset(0).buffer_row_length(0).buffer_image_height(0)
        .image_subresource(vk::ImageSubresourceLayers { aspect_mask: vk::ImageAspectFlags::COLOR, mip_level: 0, base_array_layer: 0, layer_count: 6 })
        .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(vk::Extent3D { width: size, height: size, depth: 1 });
    unsafe { device.cmd_copy_buffer_to_image(cmd, staging_buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[region]); }

    let barrier2 = vk::ImageMemoryBarrier::default()
        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL).new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED).dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image).subresource_range(vk::ImageSubresourceRange { aspect_mask: vk::ImageAspectFlags::COLOR, base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 6 })
        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::SHADER_READ);
    unsafe { device.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[barrier2]); }

    unsafe { device.end_command_buffer(cmd)?; }
    let cmd_buffers = [cmd];
    let submit = vk::SubmitInfo::default().command_buffers(&cmd_buffers);
    unsafe {
        device.queue_submit(queue, &[submit], vk::Fence::null())?;
        device.queue_wait_idle(queue)?;
        device.free_command_buffers(command_pool, &cmd_buffers);
        device.destroy_buffer(staging_buffer, None);
        device.free_memory(staging_memory, None);
    }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::CUBE).format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 6,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR).min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

    Ok((image, memory, view, sampler))
}

pub fn create_hdr_offscreen(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
    width: u32, height: u32,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView)> {
    let format = vk::Format::R16G16B16A16_SFLOAT;
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width, height, depth: 1 })
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
        .ok_or_else(|| anyhow::anyhow!("No HDR offscreen memory"))?;
    let alloc = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    Ok((image, memory, view))
}

pub fn create_ssao_offscreen(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
    width: u32, height: u32,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView)> {
    let format = vk::Format::R8_UNORM;
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width, height, depth: 1 })
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
        .ok_or_else(|| anyhow::anyhow!("No SSAO memory"))?;
    let alloc = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? };
    Ok((image, memory, view))
}
