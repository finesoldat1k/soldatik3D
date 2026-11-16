use anyhow::{Result, ensure};
use ash::{Device, vk};

use crate::buffer::GpuBuffer;
use crate::commands::ImmediateCommands;
use crate::context::VulkanContext;
use crate::memory::allocate_image;

/// An owned sampled image together with its view, memory and sampler.
pub struct GpuImage {
    device: Device,
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    #[allow(dead_code)]
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub layers: u32,
    pub mip_levels: u32,
}

impl GpuImage {
    pub fn texture_2d(
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        width: u32,
        height: u32,
        pixels: &[u8],
        nearest_filter: bool,
        repeat_address: bool,
    ) -> Result<Self> {
        ensure!(
            pixels.len() == (width * height * 4) as usize,
            "RGBA texture data has an invalid size"
        );
        let mut image = Self::allocate_2d(
            context,
            width,
            height,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageAspectFlags::COLOR,
            if nearest_filter {
                vk::Filter::NEAREST
            } else {
                vk::Filter::LINEAR
            },
            if repeat_address {
                vk::SamplerAddressMode::REPEAT
            } else {
                vk::SamplerAddressMode::CLAMP_TO_EDGE
            },
            vk::BorderColor::FLOAT_OPAQUE_BLACK,
        )?;
        image.upload_rgba8(context, uploader, pixels)?;
        Ok(image)
    }

    /// Single-channel coverage texture used by the UI glyph atlas.
    pub fn texture_2d_r8(
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Result<Self> {
        ensure!(
            pixels.len() == (width * height) as usize,
            "R8 texture data has an invalid size"
        );
        let mut image = Self::allocate_2d(
            context,
            width,
            height,
            vk::Format::R8_UNORM,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageAspectFlags::COLOR,
            vk::Filter::LINEAR,
            vk::SamplerAddressMode::CLAMP_TO_EDGE,
            vk::BorderColor::FLOAT_OPAQUE_BLACK,
        )?;
        image.upload_r8(context, uploader, pixels)?;
        Ok(image)
    }

    /// Replaces the full contents of an existing single-channel texture.
    pub fn upload_r8(
        &mut self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        pixels: &[u8],
    ) -> Result<()> {
        ensure!(
            pixels.len() == (self.extent.width * self.extent.height) as usize,
            "R8 texture data has an invalid size"
        );
        self.upload_bytes(context, uploader, pixels)
    }

    pub fn color_target(
        context: &VulkanContext,
        width: u32,
        height: u32,
        format: vk::Format,
    ) -> Result<Self> {
        Self::allocate_2d(
            context,
            width,
            height,
            format,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::Filter::LINEAR,
            vk::SamplerAddressMode::CLAMP_TO_EDGE,
            vk::BorderColor::FLOAT_OPAQUE_BLACK,
        )
    }

    pub fn depth_target(
        context: &VulkanContext,
        width: u32,
        height: u32,
        shadow_sampler: bool,
    ) -> Result<Self> {
        Self::allocate_2d(
            context,
            width,
            height,
            vk::Format::D32_SFLOAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::DEPTH,
            if shadow_sampler {
                vk::Filter::LINEAR
            } else {
                vk::Filter::NEAREST
            },
            if shadow_sampler {
                vk::SamplerAddressMode::CLAMP_TO_BORDER
            } else {
                vk::SamplerAddressMode::CLAMP_TO_EDGE
            },
            if shadow_sampler {
                vk::BorderColor::FLOAT_OPAQUE_WHITE
            } else {
                vk::BorderColor::FLOAT_OPAQUE_BLACK
            },
        )
    }

    pub fn cubemap_rgba32f(
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        size: u32,
        pixels: &[f32],
    ) -> Result<Self> {
        Self::cubemap_rgba32f_levels(context, uploader, size, &[pixels.to_vec()])
    }

    /// Uploads a full cubemap mip chain. `levels[0]` is the base level and
    /// each following level halves the face size (clamped to 1).
    pub fn cubemap_rgba32f_levels(
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        base_size: u32,
        levels: &[Vec<f32>],
    ) -> Result<Self> {
        ensure!(!levels.is_empty(), "cubemap needs at least one mip level");
        for (mip, pixels) in levels.iter().enumerate() {
            let size = (base_size >> mip).max(1);
            ensure!(
                pixels.len() == (size * size * 6 * 4) as usize,
                "RGBA32F cubemap mip {mip} has an invalid size"
            );
        }
        let mut image = Self::allocate(
            context,
            vk::Extent2D {
                width: base_size,
                height: base_size,
            },
            6,
            levels.len() as u32,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::ImageViewType::CUBE,
            vk::ImageAspectFlags::COLOR,
            vk::Filter::LINEAR,
            vk::SamplerAddressMode::CLAMP_TO_EDGE,
            vk::BorderColor::FLOAT_OPAQUE_BLACK,
            vk::ImageCreateFlags::CUBE_COMPATIBLE,
        )?;
        image.upload_cubemap_levels(context, uploader, levels)?;
        Ok(image)
    }

    #[allow(clippy::too_many_arguments)]
    fn allocate_2d(
        context: &VulkanContext,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        aspect: vk::ImageAspectFlags,
        filter: vk::Filter,
        address_mode: vk::SamplerAddressMode,
        border_color: vk::BorderColor,
    ) -> Result<Self> {
        Self::allocate(
            context,
            vk::Extent2D { width, height },
            1,
            1,
            format,
            usage,
            vk::ImageViewType::TYPE_2D,
            aspect,
            filter,
            address_mode,
            border_color,
            vk::ImageCreateFlags::empty(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn allocate(
        context: &VulkanContext,
        extent: vk::Extent2D,
        layers: u32,
        mip_levels: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        view_type: vk::ImageViewType,
        aspect: vk::ImageAspectFlags,
        filter: vk::Filter,
        address_mode: vk::SamplerAddressMode,
        border_color: vk::BorderColor,
        flags: vk::ImageCreateFlags,
    ) -> Result<Self> {
        let info = vk::ImageCreateInfo::default()
            .flags(flags)
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(mip_levels)
            .array_layers(layers)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let (image, memory) = allocate_image(
            &context.instance,
            &context.device,
            context.physical_device,
            &info,
        )?;
        let view = match create_view(
            &context.device,
            image,
            view_type,
            format,
            aspect,
            layers,
            mip_levels,
        ) {
            Ok(view) => view,
            Err(error) => {
                unsafe {
                    context.device.free_memory(memory, None);
                    context.device.destroy_image(image, None);
                }
                return Err(error);
            }
        };
        let sampler = match create_sampler(
            &context.device,
            filter,
            address_mode,
            border_color,
            mip_levels,
        ) {
            Ok(sampler) => sampler,
            Err(error) => {
                unsafe {
                    context.device.destroy_image_view(view, None);
                    context.device.free_memory(memory, None);
                    context.device.destroy_image(image, None);
                }
                return Err(error);
            }
        };

        Ok(Self {
            device: context.device.clone(),
            image,
            memory,
            view,
            sampler,
            format,
            extent,
            layers,
            mip_levels,
        })
    }

    fn upload_rgba8(
        &mut self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        pixels: &[u8],
    ) -> Result<()> {
        self.upload_bytes(context, uploader, pixels)
    }

    fn upload_bytes(
        &mut self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        pixels: &[u8],
    ) -> Result<()> {
        self.transition_for_upload(context, uploader)?;
        let size = pixels.len() as u64;
        let staging = GpuBuffer::host_visible(
            &context.device,
            &context.instance,
            context.physical_device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
        )?;
        staging.write_bytes(pixels)?;
        uploader.run(|command_buffer| {
            let region = [vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: self.layers,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width: self.extent.width,
                    height: self.extent.height,
                    depth: 1,
                })];
            unsafe {
                context.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging.handle,
                    self.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &region,
                )
            };
            Ok(())
        })?;
        self.transition_after_upload(context, uploader)
    }

    fn upload_cubemap_levels(
        &mut self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
        levels: &[Vec<f32>],
    ) -> Result<()> {
        self.transition_for_upload(context, uploader)?;
        let mut pixels = Vec::new();
        let mut regions = Vec::with_capacity(levels.len());
        let mut offset = 0u64;
        for (mip, level) in levels.iter().enumerate() {
            let size = (self.extent.width >> mip).max(1);
            regions.push(
                vk::BufferImageCopy::default()
                    .buffer_offset(offset)
                    .buffer_row_length(0)
                    .buffer_image_height(0)
                    .image_subresource(vk::ImageSubresourceLayers {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        mip_level: mip as u32,
                        base_array_layer: 0,
                        layer_count: self.layers,
                    })
                    .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                    .image_extent(vk::Extent3D {
                        width: size,
                        height: size,
                        depth: 1,
                    }),
            );
            pixels.extend_from_slice(level);
            offset += std::mem::size_of_val(level.as_slice()) as u64;
        }
        let staging = GpuBuffer::host_visible(
            &context.device,
            &context.instance,
            context.physical_device,
            offset,
            vk::BufferUsageFlags::TRANSFER_SRC,
        )?;
        staging.write_bytes(bytemuck::cast_slice(pixels.as_slice()))?;
        uploader.run(|command_buffer| {
            unsafe {
                context.device.cmd_copy_buffer_to_image(
                    command_buffer,
                    staging.handle,
                    self.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &regions,
                )
            };
            Ok(())
        })?;
        self.transition_after_upload(context, uploader)
    }

    fn transition_for_upload(
        &self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
    ) -> Result<()> {
        uploader.run(|command_buffer| {
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: self.mip_levels,
                    base_array_layer: 0,
                    layer_count: self.layers,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            unsafe {
                context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                )
            };
            Ok(())
        })
    }

    fn transition_after_upload(
        &self,
        context: &VulkanContext,
        uploader: &ImmediateCommands,
    ) -> Result<()> {
        uploader.run(|command_buffer| {
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(self.image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: self.mip_levels,
                    base_array_layer: 0,
                    layer_count: self.layers,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
            unsafe {
                context.device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier],
                )
            };
            Ok(())
        })
    }
}

impl Drop for GpuImage {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_image_view(self.view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

fn create_view(
    device: &Device,
    image: vk::Image,
    view_type: vk::ImageViewType,
    format: vk::Format,
    aspect: vk::ImageAspectFlags,
    layers: u32,
    mip_levels: u32,
) -> Result<vk::ImageView> {
    let info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(view_type)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: aspect,
            base_mip_level: 0,
            level_count: mip_levels,
            base_array_layer: 0,
            layer_count: layers,
        });
    Ok(unsafe { device.create_image_view(&info, None)? })
}

fn create_sampler(
    device: &Device,
    filter: vk::Filter,
    address_mode: vk::SamplerAddressMode,
    border_color: vk::BorderColor,
    mip_levels: u32,
) -> Result<vk::Sampler> {
    let mipmapped = mip_levels > 1;
    let info = vk::SamplerCreateInfo::default()
        .mag_filter(filter)
        .min_filter(filter)
        .mipmap_mode(if mipmapped {
            vk::SamplerMipmapMode::LINEAR
        } else {
            vk::SamplerMipmapMode::NEAREST
        })
        .min_lod(0.0)
        .max_lod((mip_levels.saturating_sub(1)) as f32)
        .address_mode_u(address_mode)
        .address_mode_v(address_mode)
        .address_mode_w(address_mode)
        .border_color(border_color)
        .compare_enable(false);
    Ok(unsafe { device.create_sampler(&info, None)? })
}
