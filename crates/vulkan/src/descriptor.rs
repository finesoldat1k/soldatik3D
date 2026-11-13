use anyhow::Result;
use ash::{Device, vk};

pub struct DescriptorSetLayout {
    device: Device,
    pub handle: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    fn new(device: &Device, handle: vk::DescriptorSetLayout) -> Self {
        Self {
            device: device.clone(),
            handle,
        }
    }

    pub fn single_texture(device: &Device) -> Result<Self> {
        Self::from_bindings(
            device,
            &[vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)],
        )
    }

    pub fn composite(device: &Device) -> Result<Self> {
        let bindings = [sampler_binding(0), sampler_binding(1), sampler_binding(2)];
        Self::from_bindings(device, &bindings)
    }

    pub fn world(device: &Device) -> Result<Self> {
        let bindings = [
            sampler_binding(0),
            sampler_binding(1),
            sampler_binding(2),
            sampler_binding(3),
            vk::DescriptorSetLayoutBinding::default()
                .binding(4)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT),
        ];
        Self::from_bindings(device, &bindings)
    }

    fn from_bindings(
        device: &Device,
        bindings: &[vk::DescriptorSetLayoutBinding<'_>],
    ) -> Result<Self> {
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
        let handle = unsafe { device.create_descriptor_set_layout(&info, None)? };
        Ok(Self::new(device, handle))
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_set_layout(self.handle, None) };
    }
}

pub struct DescriptorPool {
    device: Device,
    pub handle: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(
        device: &Device,
        max_sets: u32,
        sampler_count: u32,
        uniform_buffer_count: u32,
    ) -> Result<Self> {
        let mut sizes = Vec::with_capacity(2);
        if sampler_count > 0 {
            sizes.push(
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .descriptor_count(sampler_count),
            );
        }
        if uniform_buffer_count > 0 {
            sizes.push(
                vk::DescriptorPoolSize::default()
                    .ty(vk::DescriptorType::UNIFORM_BUFFER)
                    .descriptor_count(uniform_buffer_count),
            );
        }
        let info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(max_sets)
            .pool_sizes(&sizes);
        let handle = unsafe { device.create_descriptor_pool(&info, None)? };
        Ok(Self {
            device: device.clone(),
            handle,
        })
    }

    pub fn allocate(
        &self,
        device: &Device,
        layout: vk::DescriptorSetLayout,
        count: u32,
    ) -> Result<Vec<vk::DescriptorSet>> {
        let layouts = vec![layout; count as usize];
        let info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.handle)
            .set_layouts(&layouts);
        Ok(unsafe { device.allocate_descriptor_sets(&info)? })
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_descriptor_pool(self.handle, None) };
    }
}

fn sampler_binding(binding: u32) -> vk::DescriptorSetLayoutBinding<'static> {
    vk::DescriptorSetLayoutBinding::default()
        .binding(binding)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
}

pub fn update_single(
    device: &Device,
    set: vk::DescriptorSet,
    sampler: vk::Sampler,
    view: vk::ImageView,
    layout: vk::ImageLayout,
) {
    let image = [vk::DescriptorImageInfo::default()
        .sampler(sampler)
        .image_view(view)
        .image_layout(layout)];
    let write = vk::WriteDescriptorSet::default()
        .dst_set(set)
        .dst_binding(0)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(&image);
    unsafe { device.update_descriptor_sets(&[write], &[]) };
}

pub fn update_composite(
    device: &Device,
    set: vk::DescriptorSet,
    hdr: (vk::Sampler, vk::ImageView),
    bloom: (vk::Sampler, vk::ImageView),
    ssao: (vk::Sampler, vk::ImageView),
) {
    let hdr_info = [vk::DescriptorImageInfo::default()
        .sampler(hdr.0)
        .image_view(hdr.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let bloom_info = [vk::DescriptorImageInfo::default()
        .sampler(bloom.0)
        .image_view(bloom.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let ssao_info = [vk::DescriptorImageInfo::default()
        .sampler(ssao.0)
        .image_view(ssao.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let writes = [
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&hdr_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&bloom_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&ssao_info),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]) };
}

#[allow(clippy::too_many_arguments)]
pub fn update_world(
    device: &Device,
    set: vk::DescriptorSet,
    albedo: (vk::Sampler, vk::ImageView),
    shadow: (vk::Sampler, vk::ImageView),
    irradiance: (vk::Sampler, vk::ImageView),
    prefiltered: (vk::Sampler, vk::ImageView),
    light_buffer: vk::Buffer,
    light_buffer_size: u64,
) {
    let albedo_info = [vk::DescriptorImageInfo::default()
        .sampler(albedo.0)
        .image_view(albedo.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let shadow_info = [vk::DescriptorImageInfo::default()
        .sampler(shadow.0)
        .image_view(shadow.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let irradiance_info = [vk::DescriptorImageInfo::default()
        .sampler(irradiance.0)
        .image_view(irradiance.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let prefiltered_info = [vk::DescriptorImageInfo::default()
        .sampler(prefiltered.0)
        .image_view(prefiltered.1)
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)];
    let buffer_info = [vk::DescriptorBufferInfo::default()
        .buffer(light_buffer)
        .offset(0)
        .range(light_buffer_size)];

    let writes = [
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&albedo_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&shadow_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&irradiance_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(3)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&prefiltered_info),
        vk::WriteDescriptorSet::default()
            .dst_set(set)
            .dst_binding(4)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_info),
    ];
    unsafe { device.update_descriptor_sets(&writes, &[]) };
}
