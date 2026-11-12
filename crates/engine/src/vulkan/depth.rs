use anyhow::Result;
use ash::{vk, Device};

pub fn create_depth_resources(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice, extent: vk::Extent2D,
) -> Result<(vk::Image, vk::DeviceMemory, vk::ImageView, vk::Sampler)> {
    let format = vk::Format::D32_SFLOAT;

    let image_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D { width: extent.width, height: extent.height, depth: 1 })
        .mip_levels(1).array_layers(1).format(format)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        // КЛЮЧЕВОЕ: добавляем SAMPLED, чтобы SSAO мог читать depth как текстуру
        .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let image = unsafe { device.create_image(&image_info, None)? };
    let mem_reqs = unsafe { device.get_image_memory_requirements(image) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let mem_type = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0
            && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL))
        .unwrap();
    let alloc = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(mem_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)?; }

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image).view_type(vk::ImageViewType::TYPE_2D).format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::DEPTH,
            base_mip_level: 0, level_count: 1, base_array_layer: 0, layer_count: 1,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    // Сэмплер для depth (без сравнения — просто читаем значение)
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::NEAREST).min_filter(vk::Filter::NEAREST)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .compare_enable(false);
    let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

    Ok((image, memory, view, sampler))
}
