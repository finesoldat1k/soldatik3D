use anyhow::{Result, bail};
use ash::{Instance, vk};

pub fn find_memory_type(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32> {
    let memory = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    for i in 0..memory.memory_type_count {
        let supported = (type_filter & (1 << i)) != 0;
        let flags = memory.memory_types[i as usize].property_flags;
        if supported && flags.contains(properties) {
            return Ok(i);
        }
    }
    bail!("no Vulkan memory type matching {properties:?}")
}

pub fn allocate_image(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    create_info: &vk::ImageCreateInfo<'_>,
) -> Result<(vk::Image, vk::DeviceMemory)> {
    let image = unsafe { device.create_image(create_info, None)? };
    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory_type = match find_memory_type(
        instance,
        physical_device,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    ) {
        Ok(index) => index,
        Err(error) => {
            unsafe { device.destroy_image(image, None) };
            return Err(error);
        }
    };
    let allocation = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type);
    let memory = match unsafe { device.allocate_memory(&allocation, None) } {
        Ok(memory) => memory,
        Err(error) => {
            unsafe { device.destroy_image(image, None) };
            return Err(error.into());
        }
    };
    if let Err(error) = unsafe { device.bind_image_memory(image, memory, 0) } {
        unsafe {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
        }
        return Err(error.into());
    }
    Ok((image, memory))
}
