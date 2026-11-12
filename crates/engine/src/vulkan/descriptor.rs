use anyhow::Result;
use ash::{vk, Device};

// Мир: 4 текстуры (albedo, shadow, ibl-diffuse, ibl-specular)
pub fn create_world_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [
        vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(3).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

pub fn update_world_descriptor_set(
    device: &Device, set: vk::DescriptorSet,
    world_sampler: vk::Sampler, world_view: vk::ImageView,
    shadow_sampler: vk::Sampler, shadow_view: vk::ImageView,
    irradiance_sampler: vk::Sampler, irradiance_view: vk::ImageView,
    prefiltered_sampler: vk::Sampler, prefiltered_view: vk::ImageView,
) {
    let w = [vk::DescriptorImageInfo::default().sampler(world_sampler).image_view(world_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let s = [vk::DescriptorImageInfo::default().sampler(shadow_sampler).image_view(shadow_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let irr = [vk::DescriptorImageInfo::default().sampler(irradiance_sampler).image_view(irradiance_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let pre = [vk::DescriptorImageInfo::default().sampler(prefiltered_sampler).image_view(prefiltered_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let writes = [
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&w),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&s),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&irr),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(3).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&pre),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]); }
}

// Одиночная текстура (для bright pass и blur)
pub fn create_single_texture_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT)];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

pub fn update_single_texture(device: &Device, set: vk::DescriptorSet, sampler: vk::Sampler, view: vk::ImageView) {
    let info = [vk::DescriptorImageInfo::default()
        .sampler(sampler).image_view(view)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let write = vk::WriteDescriptorSet::default()
        .dst_set(set).dst_binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&info);
    unsafe { device.update_descriptor_sets(&[write], &[]); }
}

// SSAO: одна текстура (depth)
pub fn create_ssao_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT)];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

// Composite: 3 текстуры (hdr + bloom + ssao) — замените существующую
pub fn create_composite_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [
        vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

pub fn update_composite_set(
    device: &Device, set: vk::DescriptorSet,
    hdr_sampler: vk::Sampler, hdr_view: vk::ImageView,
    bloom_sampler: vk::Sampler, bloom_view: vk::ImageView,
    ssao_sampler: vk::Sampler, ssao_view: vk::ImageView,
) {
    let h = [vk::DescriptorImageInfo::default().sampler(hdr_sampler).image_view(hdr_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let b = [vk::DescriptorImageInfo::default().sampler(bloom_sampler).image_view(bloom_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let s = [vk::DescriptorImageInfo::default().sampler(ssao_sampler).image_view(ssao_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let writes = [
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&h),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&b),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&s),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]); }
}

// UI: одна текстура (шрифт)
pub fn create_ui_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [vk::DescriptorSetLayoutBinding::default()
        .binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT)];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

pub fn update_ui_descriptor_set(device: &Device, set: vk::DescriptorSet, sampler: vk::Sampler, view: vk::ImageView) {
    let image_info = [vk::DescriptorImageInfo::default()
        .sampler(sampler).image_view(view)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let write = vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&image_info);
    unsafe { device.update_descriptor_sets(&[write], &[]); }
}

// Хелпер: создать пул и один набор
pub fn create_descriptor_pool_and_set(
    device: &Device, layout: vk::DescriptorSetLayout, count: u32,
) -> Result<(vk::DescriptorPool, vk::DescriptorSet)> {
    let pool_sizes = [vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(count)];
    let pool_info = vk::DescriptorPoolCreateInfo::default().max_sets(1).pool_sizes(&pool_sizes);
    let pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };
    let set_layouts = [layout];
    let alloc_info = vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&set_layouts);
    let set = unsafe { device.allocate_descriptor_sets(&alloc_info)?[0] };
    Ok((pool, set))
}

pub fn update_single_texture_with_layout(
    device: &Device, set: vk::DescriptorSet,
    sampler: vk::Sampler, view: vk::ImageView, layout: vk::ImageLayout,
) {
    let info = [vk::DescriptorImageInfo::default()
        .sampler(sampler).image_view(view).image_layout(layout)];
    let write = vk::WriteDescriptorSet::default()
        .dst_set(set).dst_binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&info);
    unsafe { device.update_descriptor_sets(&[write], &[]); }
}

// Обновление world-дескриптора с UBO для 8 источников света
pub fn update_world_descriptor_set_with_lights(
    device: &Device, set: vk::DescriptorSet,
    world_sampler: vk::Sampler, world_view: vk::ImageView,
    shadow_sampler: vk::Sampler, shadow_view: vk::ImageView,
    irradiance_sampler: vk::Sampler, irradiance_view: vk::ImageView,
    prefiltered_sampler: vk::Sampler, prefiltered_view: vk::ImageView,
    light_buffer: vk::Buffer, light_buffer_size: u64,
) {
    let w = [vk::DescriptorImageInfo::default().sampler(world_sampler).image_view(world_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let s = [vk::DescriptorImageInfo::default().sampler(shadow_sampler).image_view(shadow_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let irr = [vk::DescriptorImageInfo::default().sampler(irradiance_sampler).image_view(irradiance_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let pre = [vk::DescriptorImageInfo::default().sampler(prefiltered_sampler).image_view(prefiltered_view).image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let light_info = [vk::DescriptorBufferInfo::default().buffer(light_buffer).offset(0).range(light_buffer_size)];

    let writes = [
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&w),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&s),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&irr),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(3).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).image_info(&pre),
        vk::WriteDescriptorSet::default().dst_set(set).dst_binding(4).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER).buffer_info(&light_info),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]); }
}

// Layout для world (5 bindings, binding 4 = UBO для света)
pub fn create_world_descriptor_set_layout_with_lights(device: &Device) -> Result<vk::DescriptorSetLayout> {
    let bindings = [
        vk::DescriptorSetLayoutBinding::default().binding(0).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(1).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(2).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(3).descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
        vk::DescriptorSetLayoutBinding::default().binding(4).descriptor_type(vk::DescriptorType::UNIFORM_BUFFER).descriptor_count(1).stage_flags(vk::ShaderStageFlags::FRAGMENT),
    ];
    let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    Ok(unsafe { device.create_descriptor_set_layout(&info, None)? })
}

// Пул с поддержкой UBO + всех сэмплеров
pub fn create_world_descriptor_pool_and_set(device: &Device, layout: vk::DescriptorSetLayout) -> Result<(vk::DescriptorPool, vk::DescriptorSet)> {
    let pool_sizes = [
        vk::DescriptorPoolSize::default().ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER).descriptor_count(4),
        vk::DescriptorPoolSize::default().ty(vk::DescriptorType::UNIFORM_BUFFER).descriptor_count(1),
    ];
    let pool_info = vk::DescriptorPoolCreateInfo::default().max_sets(1).pool_sizes(&pool_sizes);
    let pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };
    let set_layouts = [layout];
    let alloc_info = vk::DescriptorSetAllocateInfo::default().descriptor_pool(pool).set_layouts(&set_layouts);
    let set = unsafe { device.allocate_descriptor_sets(&alloc_info)?[0] };
    Ok((pool, set))
}
