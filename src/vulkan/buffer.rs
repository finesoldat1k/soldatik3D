use anyhow::{Result, ensure};
use ash::{vk, Device};
use crate::ui::UiVertex;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub emission: f32,
}

pub fn create_vertex_buffer(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice, vertices: &[Vertex],
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    ensure!(!vertices.is_empty(), "Vertex array is empty");
    let buffer_size = (std::mem::size_of::<Vertex>() * vertices.len()) as u64;
    let buffer_info = vk::BufferCreateInfo::default().size(buffer_size).usage(vk::BufferUsageFlags::VERTEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let memory_type_index = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT))
        .ok_or_else(|| anyhow::anyhow!("No suitable memory type found"))?;
    let alloc_info = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(memory_type_index);
    let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_buffer_memory(buffer, memory, 0)?; }
    let data_ptr = unsafe { device.map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())? } as *mut Vertex;
    unsafe { data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()); device.unmap_memory(memory); }
    Ok((buffer, memory))
}

// === НОВОЕ: 8 источников света ===
pub const MAX_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct LightData {
    pub position_radius: [f32; 4],  // xyz = позиция, w = радиус
    pub color_intensity: [f32; 4],  // rgb = цвет, a = интенсивность
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LightBuffer {
    pub lights: [LightData; MAX_LIGHTS],
}

pub fn create_light_buffer(
    instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_size = std::mem::size_of::<LightBuffer>() as u64;
    let buffer_info = vk::BufferCreateInfo::default()
        .size(buffer_size)
        .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let memory_type_index = (0..mem_props.memory_type_count)
        .find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT))
        .ok_or_else(|| anyhow::anyhow!("No light UBO memory"))?;
    let alloc_info = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(memory_type_index);
    let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_buffer_memory(buffer, memory, 0)?; }
    Ok((buffer, memory))
}

pub fn update_light_buffer(device: &Device, memory: vk::DeviceMemory, buffer: &LightBuffer) -> Result<()> {
    let size = std::mem::size_of::<LightBuffer>() as u64;
    let ptr = unsafe { device.map_memory(memory, 0, size, vk::MemoryMapFlags::empty())? } as *mut LightBuffer;
    unsafe {
        std::ptr::copy_nonoverlapping(buffer as *const LightBuffer, ptr, 1);
        device.unmap_memory(memory);
    }
    Ok(())
}

// === Старое: UI буферы ===
pub fn create_ui_buffer(instance: &ash::Instance, device: &Device, physical_device: vk::PhysicalDevice, capacity: usize) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let buffer_size = (std::mem::size_of::<UiVertex>() * capacity) as u64;
    let buffer_info = vk::BufferCreateInfo::default().size(buffer_size).usage(vk::BufferUsageFlags::VERTEX_BUFFER).sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
    let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
    let mem_props = unsafe { instance.get_physical_device_memory_properties(physical_device) };
    let memory_type_index = (0..mem_props.memory_type_count).find(|&i| (mem_reqs.memory_type_bits & (1 << i)) != 0 && mem_props.memory_types[i as usize].property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT)).unwrap();
    let alloc_info = vk::MemoryAllocateInfo::default().allocation_size(mem_reqs.size).memory_type_index(memory_type_index);
    let memory = unsafe { device.allocate_memory(&alloc_info, None)? };
    unsafe { device.bind_buffer_memory(buffer, memory, 0)?; }
    Ok((buffer, memory))
}

pub fn update_ui_buffer(device: &Device, memory: vk::DeviceMemory, vertices: &[UiVertex]) -> Result<()> {
    if vertices.is_empty() { return Ok(()); }
    let buffer_size = (std::mem::size_of::<UiVertex>() * vertices.len()) as u64;
    let data_ptr = unsafe { device.map_memory(memory, 0, buffer_size, vk::MemoryMapFlags::empty())? } as *mut UiVertex;
    unsafe { data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len()); device.unmap_memory(memory); }
    Ok(())
}
