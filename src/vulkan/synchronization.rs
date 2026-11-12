use anyhow::Result;
use ash::{vk, Device};

pub fn create_semaphores(device: &Device, count: usize) -> Result<Vec<vk::Semaphore>> {
    let create_info = vk::SemaphoreCreateInfo::default();
    (0..count)
        .map(|_| unsafe { device.create_semaphore(&create_info, None).map_err(Into::into) })
        .collect()
}

pub fn create_fences(device: &Device, count: usize, signaled: bool) -> Result<Vec<vk::Fence>> {
    let flags = if signaled {
        vk::FenceCreateFlags::SIGNALED
    } else {
        vk::FenceCreateFlags::empty()
    };
    let create_info = vk::FenceCreateInfo::default().flags(flags);
    (0..count)
        .map(|_| unsafe { device.create_fence(&create_info, None).map_err(Into::into) })
        .collect()
}
