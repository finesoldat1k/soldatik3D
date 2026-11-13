use anyhow::Result;
use ash::{Device, vk};

/// Command pool used for frame command buffers.
pub struct CommandPool {
    device: Device,
    pub handle: vk::CommandPool,
}

impl CommandPool {
    pub fn new(device: &Device, queue_family: u32) -> Result<Self> {
        let info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let handle = unsafe { device.create_command_pool(&info, None)? };
        Ok(Self {
            device: device.clone(),
            handle,
        })
    }

    pub fn allocate(&self, count: u32) -> Result<Vec<vk::CommandBuffer>> {
        let info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.handle)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);
        Ok(unsafe { self.device.allocate_command_buffers(&info)? })
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.handle, None) };
    }
}

/// One-shot command submission helper used for uploads during initialization.
pub struct ImmediateCommands {
    device: Device,
    pool: vk::CommandPool,
    queue: vk::Queue,
}

impl ImmediateCommands {
    pub fn new(device: &Device, queue_family: u32, queue: vk::Queue) -> Result<Self> {
        let info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family)
            .flags(vk::CommandPoolCreateFlags::TRANSIENT);
        let pool = unsafe { device.create_command_pool(&info, None)? };
        Ok(Self {
            device: device.clone(),
            pool,
            queue,
        })
    }

    pub fn run<F>(&self, record: F) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer) -> Result<()>,
    {
        let allocation = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffer = unsafe { self.device.allocate_command_buffers(&allocation)?[0] };

        let begin = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe { self.device.begin_command_buffer(command_buffer, &begin)? };
        if let Err(error) = record(command_buffer) {
            unsafe {
                self.device
                    .free_command_buffers(self.pool, &[command_buffer]);
            }
            return Err(error);
        }
        unsafe { self.device.end_command_buffer(command_buffer)? };

        let command_buffers = [command_buffer];
        let submit = vk::SubmitInfo::default().command_buffers(&command_buffers);
        unsafe {
            self.device
                .queue_submit(self.queue, &[submit], vk::Fence::null())?;
            self.device.queue_wait_idle(self.queue)?;
            self.device
                .free_command_buffers(self.pool, &[command_buffer]);
        }
        Ok(())
    }
}

impl Drop for ImmediateCommands {
    fn drop(&mut self) {
        unsafe { self.device.destroy_command_pool(self.pool, None) };
    }
}
