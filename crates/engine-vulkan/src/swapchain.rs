use anyhow::{Result, anyhow};
use ash::{khr, vk};

use crate::context::VulkanContext;

pub struct SwapchainSupport {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn query_support(
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<SwapchainSupport> {
    unsafe {
        Ok(SwapchainSupport {
            capabilities: surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)?,
            formats: surface_loader
                .get_physical_device_surface_formats(physical_device, surface)?,
            present_modes: surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)?,
        })
    }
}

pub struct Swapchain {
    device: ash::Device,
    loader: khr::swapchain::Device,
    pub handle: vk::SwapchainKHR,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub views: Vec<vk::ImageView>,
}

impl Swapchain {
    pub fn new(
        context: &VulkanContext,
        size: (u32, u32),
        preferred_format: Option<vk::Format>,
        old_swapchain: vk::SwapchainKHR,
    ) -> Result<Self> {
        let support = query_support(
            context.physical_device,
            &context.surface_loader,
            context.surface,
        )?;
        let surface_format = choose_surface_format(&support.formats, preferred_format);
        let present_mode = choose_present_mode(&support.present_modes);
        let extent = choose_extent(&support.capabilities, size);

        let mut image_count = support.capabilities.min_image_count + 1;
        if support.capabilities.max_image_count > 0 {
            image_count = image_count.min(support.capabilities.max_image_count);
        }

        let queue_families = [context.graphics_family, context.present_family];
        let (sharing_mode, queue_family_indices): (vk::SharingMode, &[u32]) =
            if context.graphics_family != context.present_family {
                (vk::SharingMode::CONCURRENT, &queue_families)
            } else {
                (vk::SharingMode::EXCLUSIVE, &[])
            };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(context.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(sharing_mode)
            .queue_family_indices(queue_family_indices)
            .pre_transform(support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let loader = khr::swapchain::Device::new(&context.instance, &context.device);
        let handle = unsafe { loader.create_swapchain(&create_info, None)? };
        let images = unsafe { loader.get_swapchain_images(handle)? };
        let views = match create_image_views(&context.device, &images, surface_format.format) {
            Ok(views) => views,
            Err(error) => {
                unsafe { loader.destroy_swapchain(handle, None) };
                return Err(error);
            }
        };

        println!(
            "[engine-vulkan] swapchain: {}x{}, format {:?}, {} images, present {:?}",
            extent.width,
            extent.height,
            surface_format.format,
            images.len(),
            present_mode
        );

        Ok(Self {
            device: context.device.clone(),
            loader,
            handle,
            format: surface_format.format,
            extent,
            images,
            views,
        })
    }

    /// Returns `Ok(None)` when the swapchain is out of date and must be
    /// recreated.
    pub fn acquire(&self, image_available: vk::Semaphore) -> Result<Option<u32>> {
        match unsafe {
            self.loader.acquire_next_image(
                self.handle,
                u64::MAX,
                image_available,
                vk::Fence::null(),
            )
        } {
            Ok((index, _)) => Ok(Some(index)),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(None),
            Err(error) => Err(anyhow!("acquire_next_image failed: {error:?}")),
        }
    }

    /// Returns `Ok(true)` when the swapchain is out of date or suboptimal.
    pub fn present(
        &self,
        present_queue: vk::Queue,
        image_index: u32,
        render_finished: vk::Semaphore,
    ) -> Result<bool> {
        let swapchains = [self.handle];
        let image_indices = [image_index];
        let wait = [render_finished];
        let info = vk::PresentInfoKHR::default()
            .wait_semaphores(&wait)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        match unsafe { self.loader.queue_present(present_queue, &info) } {
            Ok(suboptimal) => Ok(suboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(error) => Err(anyhow!("queue_present failed: {error:?}")),
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for view in self.views.drain(..) {
                self.device.destroy_image_view(view, None);
            }
            self.loader.destroy_swapchain(self.handle, None);
        }
    }
}

/// Picks the surface format used by the swapchain. Exposed so the renderer can
/// create a compatible swapchain render pass before the swapchain itself.
pub fn choose_surface_format(
    formats: &[vk::SurfaceFormatKHR],
    preferred: Option<vk::Format>,
) -> vk::SurfaceFormatKHR {
    if let Some(preferred) = preferred
        && let Some(format) = formats.iter().find(|format| format.format == preferred)
    {
        return *format;
    }

    // The post-process pass performs tone mapping and gamma correction itself,
    // so a UNORM swapchain is the predictable choice. SRGB is only a fallback.
    const PREFERRED: [vk::Format; 3] = [
        vk::Format::B8G8R8A8_UNORM,
        vk::Format::R8G8B8A8_UNORM,
        vk::Format::B8G8R8A8_SRGB,
    ];
    for wanted in PREFERRED {
        if let Some(format) = formats.iter().find(|format| format.format == wanted) {
            return *format;
        }
    }
    formats[0]
}

fn choose_present_mode(modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    if modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    }
}

fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR, size: (u32, u32)) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        return capabilities.current_extent;
    }
    vk::Extent2D {
        width: size.0.clamp(
            capabilities.min_image_extent.width,
            capabilities.max_image_extent.width,
        ),
        height: size.1.clamp(
            capabilities.min_image_extent.height,
            capabilities.max_image_extent.height,
        ),
    }
}

fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>> {
    let mut views = Vec::with_capacity(images.len());
    for &image in images {
        let info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        match unsafe { device.create_image_view(&info, None) } {
            Ok(view) => views.push(view),
            Err(error) => {
                unsafe {
                    for view in views.drain(..) {
                        device.destroy_image_view(view, None);
                    }
                }
                return Err(error.into());
            }
        }
    }
    Ok(views)
}
