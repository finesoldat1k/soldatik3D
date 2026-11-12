use anyhow::{Context, Result, anyhow};
use ash::{Entry, Instance, khr, vk};
use engine_render::WindowSurface;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};

use crate::swapchain;

/// Owns the Vulkan instance, surface, physical device and logical device.
pub struct VulkanContext {
    /// Kept alive for the lifetime of the instance.
    #[allow(dead_code)]
    pub entry: Entry,
    pub instance: Instance,
    pub surface_loader: khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub graphics_family: u32,
    pub present_family: u32,
    #[allow(dead_code)]
    pub device_name: String,
}

impl VulkanContext {
    pub fn new(window: WindowSurface) -> Result<Self> {
        if window.is_empty() {
            return Err(anyhow!(
                "cannot create a Vulkan surface for a zero-sized window"
            ));
        }
        let entry = unsafe { Entry::load().context("failed to load the Vulkan loader")? };
        let instance = create_instance(&entry, window)?;
        let surface_loader = khr::surface::Instance::new(&entry, &instance);
        let surface = create_surface(&entry, &instance, window)?;

        let physical_device = match pick_physical_device(&instance, &surface_loader, surface) {
            Ok(device) => device,
            Err(error) => {
                unsafe {
                    surface_loader.destroy_surface(surface, None);
                    instance.destroy_instance(None);
                }
                return Err(error);
            }
        };
        let (graphics_family, present_family) =
            find_queue_families(&instance, &surface_loader, surface, physical_device)?;
        let device_name = unsafe {
            instance
                .get_physical_device_properties(physical_device)
                .device_name_as_c_str()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "<unknown>".to_string())
        };

        let (device, graphics_queue, present_queue) = match create_logical_device(
            &instance,
            physical_device,
            graphics_family,
            present_family,
        ) {
            Ok(result) => result,
            Err(error) => {
                unsafe {
                    surface_loader.destroy_surface(surface, None);
                    instance.destroy_instance(None);
                }
                return Err(error);
            }
        };

        println!(
            "[engine-vulkan] device: {device_name} (graphics queue {graphics_family}, present queue {present_family})"
        );

        Ok(Self {
            entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            graphics_family,
            present_family,
            device_name,
        })
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}

fn create_instance(entry: &Entry, window: WindowSurface) -> Result<Instance> {
    let app_name = CString::new("Soldatik3D")?;
    let engine_name = CString::new("Soldatik3D Engine")?;
    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_2);

    let validation_layer = CString::new("VK_LAYER_KHRONOS_validation")?;
    let mut layers: Vec<CString> = Vec::new();
    if cfg!(debug_assertions) {
        let available = unsafe { entry.enumerate_instance_layer_properties()? };
        let supported = available.iter().any(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            name == validation_layer.as_c_str()
        });
        if supported {
            layers.push(validation_layer);
        } else {
            println!("[engine-vulkan] validation layer is not installed");
        }
    }
    let layer_ptrs: Vec<*const i8> = layers.iter().map(|layer| layer.as_ptr()).collect();

    let surface_extensions = ash_window::enumerate_required_extensions(window.display)?;
    let extension_ptrs: Vec<*const i8> = surface_extensions.to_vec();

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&extension_ptrs);
    Ok(unsafe { entry.create_instance(&create_info, None)? })
}

fn create_surface(
    entry: &Entry,
    instance: &Instance,
    window: WindowSurface,
) -> Result<vk::SurfaceKHR> {
    unsafe {
        ash_window::create_surface(entry, instance, window.display, window.window, None)
            .map_err(Into::into)
    }
}

fn pick_physical_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<vk::PhysicalDevice> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    let mut fallback = None;

    for device in devices {
        let queues = find_queue_families(instance, surface_loader, surface, device);
        if queues.is_err() {
            continue;
        }
        if !supports_swapchain(instance, device) {
            continue;
        }
        let support = match swapchain::query_support(device, surface_loader, surface) {
            Ok(support) => support,
            Err(_) => continue,
        };
        if support.formats.is_empty() || support.present_modes.is_empty() {
            continue;
        }

        let properties = unsafe { instance.get_physical_device_properties(device) };
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            return Ok(device);
        }
        if fallback.is_none() {
            fallback = Some(device);
        }
    }

    fallback.ok_or_else(|| anyhow!("no Vulkan-capable GPU with a presentable surface found"))
}

fn supports_swapchain(instance: &Instance, device: vk::PhysicalDevice) -> bool {
    let extensions = match unsafe { instance.enumerate_device_extension_properties(device) } {
        Ok(extensions) => extensions,
        Err(_) => return false,
    };
    extensions.iter().any(|extension| {
        extension
            .extension_name_as_c_str()
            .map(|name| name == khr::swapchain::NAME)
            .unwrap_or(false)
    })
}

fn find_queue_families(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> Result<(u32, u32)> {
    let families = unsafe { instance.get_physical_device_queue_family_properties(device) };
    let mut graphics = None;
    let mut present = None;

    for (index, family) in families.iter().enumerate() {
        let index = index as u32;
        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
            graphics = Some(index);
        }
        if unsafe { surface_loader.get_physical_device_surface_support(device, index, surface)? }
            && present.is_none()
        {
            present = Some(index);
        }
        if graphics.is_some() && present.is_some() {
            break;
        }
    }

    Ok((
        graphics.ok_or_else(|| anyhow!("no graphics queue family"))?,
        present.ok_or_else(|| anyhow!("no present queue family"))?,
    ))
}

fn create_logical_device(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
    graphics_family: u32,
    present_family: u32,
) -> Result<(ash::Device, vk::Queue, vk::Queue)> {
    let unique_families: BTreeSet<u32> = [graphics_family, present_family].into_iter().collect();
    let priorities = [1.0f32];
    let queue_infos: Vec<vk::DeviceQueueCreateInfo<'_>> = unique_families
        .iter()
        .map(|family| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(*family)
                .queue_priorities(&priorities)
        })
        .collect();

    let extensions = [khr::swapchain::NAME.as_ptr()];
    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&extensions);
    let device = unsafe { instance.create_device(physical_device, &create_info, None)? };
    let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family, 0) };
    Ok((device, graphics_queue, present_queue))
}
