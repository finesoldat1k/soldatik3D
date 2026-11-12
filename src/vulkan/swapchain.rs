use anyhow::Result;
use ash::{khr, vk, Instance};
use winit::window::Window;

use super::device::QueueFamilyIndices;

pub struct SwapchainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

pub fn query_swapchain_support(
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<SwapchainSupportDetails> {
    unsafe {
        Ok(SwapchainSupportDetails {
            capabilities: surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)?,
            formats: surface_loader.get_physical_device_surface_formats(physical_device, surface)?,
            present_modes: surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)?,
        })
    }
}

fn choose_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    // 1. Идеальный вариант: HDR scRGB
    for &f in formats {
        if f.format == vk::Format::R16G16B16A16_SFLOAT 
            && f.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT {
            println!("[swapchain] HDR (scRGB) формат выбран: R16G16B16A16_SFLOAT");
            return f;
        }
    }
    // 2. HDR10 (если scRGB нет)
    for &f in formats {
        if f.format == vk::Format::A2B10G10R10_UNORM_PACK32 
            && f.color_space == vk::ColorSpaceKHR::HDR10_ST2084_EXT {
            println!("[swapchain] HDR10 формат выбран");
            return f;
        }
    }
    // 3. Fallback: обычный SDR
    for &f in formats {
        if f.format == vk::Format::B8G8R8A8_SRGB 
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR {
            println!("[swapchain] HDR не поддерживается, используется SDR");
            return f;
        }
    }
    println!("[swapchain] Используется первый доступный формат");
    formats[0]
}

fn choose_present_mode(modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    if modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    }
}

fn choose_extent(capabilities: &vk::SurfaceCapabilitiesKHR, window: &Window) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        let size = window.inner_size();
        vk::Extent2D {
            width: size.width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: size.height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

pub fn create_swapchain(
    instance: &Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    window: &Window,
    indices: &QueueFamilyIndices,
) -> Result<(
    khr::swapchain::Device,
    vk::SwapchainKHR,
    vk::Format,
    vk::Extent2D,
    Vec<vk::Image>,
)> {
    let support = query_swapchain_support(physical_device, surface_loader, surface)?;
    let surface_format = choose_surface_format(&support.formats);
    let present_mode = choose_present_mode(&support.present_modes);
    let extent = choose_extent(&support.capabilities, window);

    let mut image_count = support.capabilities.min_image_count + 1;
    if support.capabilities.max_image_count > 0 && image_count > support.capabilities.max_image_count
    {
        image_count = support.capabilities.max_image_count;
    }

    let graphics_family = indices.graphics_family.unwrap();
    let present_family = indices.present_family.unwrap();
    let queue_family_array = [graphics_family, present_family];
    let (sharing_mode, queue_family_slice): (vk::SharingMode, &[u32]) =
        if graphics_family != present_family {
            (vk::SharingMode::CONCURRENT, &queue_family_array)
        } else {
            (vk::SharingMode::EXCLUSIVE, &[])
        };

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(sharing_mode)
        .queue_family_indices(queue_family_slice)
        .pre_transform(support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(vk::SwapchainKHR::null());

    let swapchain_loader = khr::swapchain::Device::new(instance, device);
    let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };
    let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

    Ok((
        swapchain_loader,
        swapchain,
        surface_format.format,
        extent,
        images,
    ))
}

pub fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>> {
    images
        .iter()
        .map(|&image| {
            let create_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .components(vk::ComponentMapping::default())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            unsafe { device.create_image_view(&create_info, None).map_err(Into::into) }
        })
        .collect()
}
