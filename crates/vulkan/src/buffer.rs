use anyhow::{Result, ensure};
use ash::{Device, vk};
use bytemuck::Pod;
use scene::Vertex;

use crate::commands::ImmediateCommands;
use crate::memory::find_memory_type;

/// An owned Vulkan buffer and its memory.
pub struct GpuBuffer {
    device: Device,
    pub handle: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: u64,
    pub usage: vk::BufferUsageFlags,
}

impl GpuBuffer {
    pub fn new(
        device: &Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        size: u64,
        usage: vk::BufferUsageFlags,
        memory_flags: vk::MemoryPropertyFlags,
    ) -> Result<Self> {
        ensure!(size > 0, "cannot create an empty GPU buffer");
        let info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let handle = unsafe { device.create_buffer(&info, None)? };
        let requirements = unsafe { device.get_buffer_memory_requirements(handle) };
        let memory_type = match find_memory_type(
            instance,
            physical_device,
            requirements.memory_type_bits,
            memory_flags,
        ) {
            Ok(index) => index,
            Err(error) => {
                unsafe { device.destroy_buffer(handle, None) };
                return Err(error);
            }
        };
        let allocation = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type);
        let memory = match unsafe { device.allocate_memory(&allocation, None) } {
            Ok(memory) => memory,
            Err(error) => {
                unsafe { device.destroy_buffer(handle, None) };
                return Err(error.into());
            }
        };
        if let Err(error) = unsafe { device.bind_buffer_memory(handle, memory, 0) } {
            unsafe {
                device.free_memory(memory, None);
                device.destroy_buffer(handle, None);
            }
            return Err(error.into());
        }
        Ok(Self {
            device: device.clone(),
            handle,
            memory,
            size,
            usage,
        })
    }

    pub fn host_visible(
        device: &Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        size: u64,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        Self::new(
            device,
            instance,
            physical_device,
            size,
            usage,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
    }

    pub fn vertex(
        device: &Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        uploader: &ImmediateCommands,
        vertices: &[Vertex],
    ) -> Result<Self> {
        ensure!(!vertices.is_empty(), "cannot upload an empty vertex buffer");
        let bytes = bytemuck::cast_slice(vertices);
        Self::upload(
            device,
            instance,
            physical_device,
            uploader,
            bytes,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )
    }

    pub fn upload(
        device: &Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        uploader: &ImmediateCommands,
        data: &[u8],
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        ensure!(!data.is_empty(), "cannot upload an empty buffer");
        let size = data.len() as u64;
        let mut destination = Self::new(
            device,
            instance,
            physical_device,
            size,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let staging = Self::host_visible(
            device,
            instance,
            physical_device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
        )?;
        staging.write_bytes(data)?;

        uploader.run(|command_buffer| {
            let region = [vk::BufferCopy::default()
                .src_offset(0)
                .dst_offset(0)
                .size(size)];
            unsafe {
                device.cmd_copy_buffer(command_buffer, staging.handle, destination.handle, &region)
            };
            Ok(())
        })?;

        // Rewrite the usage flags to the caller's intent; TRANSFER_DST was an
        // implementation detail of the upload path.
        destination.usage = usage;
        Ok(destination)
    }

    pub fn write_bytes(&self, data: &[u8]) -> Result<()> {
        ensure!(
            data.len() as u64 <= self.size,
            "buffer write is larger than the allocated buffer"
        );
        let pointer = unsafe {
            self.device.map_memory(
                self.memory,
                0,
                data.len() as u64,
                vk::MemoryMapFlags::empty(),
            )?
        } as *mut u8;
        unsafe {
            pointer.copy_from_nonoverlapping(data.as_ptr(), data.len());
            self.device.unmap_memory(self.memory);
        }
        Ok(())
    }

    pub fn write_pod<T: Pod>(&self, data: &T) -> Result<()> {
        self.write_bytes(bytemuck::bytes_of(data))
    }

    pub fn write_slice<T: Pod>(&self, data: &[T]) -> Result<()> {
        self.write_bytes(bytemuck::cast_slice(data))
    }
}

impl Drop for GpuBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.handle, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
