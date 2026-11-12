use anyhow::Result;
use ash::{vk, Entry, Instance};
use std::ffi::{CStr, CString};
use winit::{raw_window_handle::HasDisplayHandle, window::Window};

pub fn create_instance(entry: &Entry, window: &Window) -> Result<Instance> {
    let app_name = CString::new("Soldatik3D")?;
    let engine_name = CString::new("Soldatik3D Engine")?;

    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_2); // Немного поднимем версию для безопасности

    let enable_validation = cfg!(debug_assertions);
    let validation_layer = CString::new("VK_LAYER_KHRONOS_validation")?;
    
    let validation_available = if enable_validation {
        let available_layers = unsafe { entry.enumerate_instance_layer_properties()? };
        available_layers.iter().any(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            name == validation_layer.as_c_str()
        })
    } else { false };

    let enabled_layers: Vec<CString> = if validation_available {
        vec![validation_layer]
    } else { Vec::new() };
    let layer_ptrs: Vec<*const i8> = enabled_layers.iter().map(|l| l.as_ptr()).collect();

    // Добавляем расширение для HDR (широкоцветные форматы)
    let display_handle = window.display_handle()?.as_raw();
    let surface_extensions = ash_window::enumerate_required_extensions(display_handle)?;
    let mut extension_ptrs: Vec<*const i8> = surface_extensions.to_vec();
    
    let hdr_extension = CString::new("VK_EXT_swapchain_colorspace")?;
    extension_ptrs.push(hdr_extension.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&layer_ptrs)
        .enabled_extension_names(&extension_ptrs);

    let instance = unsafe { entry.create_instance(&create_info, None)? };
    Ok(instance)
}
