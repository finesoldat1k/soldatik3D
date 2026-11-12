use anyhow::{anyhow, Result};
use ash::{khr, vk, Instance};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug)]
pub struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

pub fn find_queue_families(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> Result<QueueFamilyIndices> {
    let mut indices = QueueFamilyIndices {
        graphics_family: None,
        present_family: None,
    };

    let queue_families = unsafe { instance.get_physical_device_queue_family_properties(device) };
    for (i, family) in queue_families.iter().enumerate() {
        let i = i as u32;

        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            indices.graphics_family = Some(i);
        }

        let present_support =
            unsafe { surface_loader.get_physical_device_surface_support(device, i, surface)? };
        if present_support {
            indices.present_family = Some(i);
        }

        if indices.is_complete() {
            break;
        }
    }

    Ok(indices)
}

pub fn pick_physical_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<vk::PhysicalDevice> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    if devices.is_empty() {
        return Err(anyhow!("No Vulkan-capable GPU found"));
    }

    let mut fallback: Option<vk::PhysicalDevice> = None;

    for &device in &devices {
        let indices = find_queue_families(instance, surface_loader, surface, device)?;
        if !indices.is_complete() {
            continue;
        }

        let props = unsafe { instance.get_physical_device_properties(device) };
        let name = props
            .device_name_as_c_str()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "<unknown>".into());

        println!("[device] Candidate: {name:?} ({:?})", props.device_type);

        if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            println!("[device] Selected discrete GPU: {name:?}");
            return Ok(device);
        }
        if fallback.is_none() {
            fallback = Some(device);
        }
    }

    fallback.ok_or_else(|| anyhow!("No suitable GPU with required queue families"))
}

pub fn create_logical_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    indices: &QueueFamilyIndices,
) -> Result<(ash::Device, vk::Queue, vk::Queue)> {
    let graphics_family = indices.graphics_family.unwrap();
    let present_family = indices.present_family.unwrap();

    // Уникальные индексы семейств (если графика и презентация в одном семействе)
    let mut unique_families = HashSet::new();
    unique_families.insert(graphics_family);
    unique_families.insert(present_family);

    let queue_priorities = [1.0f32];
    let queue_create_infos: Vec<_> = unique_families
        .iter()
        .map(|&family| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&queue_priorities)
        })
        .collect();

    let device_extensions = [khr::swapchain::NAME.as_ptr()];

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&device_extensions);

    let device = unsafe { instance.create_device(physical_device, &create_info, None)? };

    let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family, 0) };

    Ok((device, graphics_queue, present_queue))
}
