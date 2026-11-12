use anyhow::Result;
use ash::{Device, vk};

pub struct SyncObjects {
    device: Device,
    pub image_available: Vec<vk::Semaphore>,
    pub render_finished: Vec<vk::Semaphore>,
    pub in_flight: Vec<vk::Fence>,
}

impl SyncObjects {
    pub fn new(
        device: &Device,
        swapchain_images: usize,
        max_frames_in_flight: usize,
    ) -> Result<Self> {
        let image_available = create_semaphores(device, max_frames_in_flight)?;
        let render_finished = create_semaphores(device, swapchain_images)?;
        let in_flight = create_fences(device, max_frames_in_flight, true)?;
        Ok(Self {
            device: device.clone(),
            image_available,
            render_finished,
            in_flight,
        })
    }

    pub fn recreate_render_finished(&mut self, count: usize) -> Result<()> {
        unsafe {
            for semaphore in self.render_finished.drain(..) {
                self.device.destroy_semaphore(semaphore, None);
            }
        }
        self.render_finished = create_semaphores(&self.device, count)?;
        Ok(())
    }
}

impl Drop for SyncObjects {
    fn drop(&mut self) {
        unsafe {
            for semaphore in self.image_available.drain(..) {
                self.device.destroy_semaphore(semaphore, None);
            }
            for semaphore in self.render_finished.drain(..) {
                self.device.destroy_semaphore(semaphore, None);
            }
            for fence in self.in_flight.drain(..) {
                self.device.destroy_fence(fence, None);
            }
        }
    }
}

fn create_semaphores(device: &Device, count: usize) -> Result<Vec<vk::Semaphore>> {
    let info = vk::SemaphoreCreateInfo::default();
    (0..count)
        .map(|_| unsafe { device.create_semaphore(&info, None).map_err(Into::into) })
        .collect()
}

fn create_fences(device: &Device, count: usize, signaled: bool) -> Result<Vec<vk::Fence>> {
    let flags = if signaled {
        vk::FenceCreateFlags::SIGNALED
    } else {
        vk::FenceCreateFlags::empty()
    };
    let info = vk::FenceCreateInfo::default().flags(flags);
    (0..count)
        .map(|_| unsafe { device.create_fence(&info, None).map_err(Into::into) })
        .collect()
}
