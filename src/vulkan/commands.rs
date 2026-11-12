use anyhow::Result;
use ash::{vk, Device};

pub fn create_command_pool(device: &Device, queue_family_index: u32) -> Result<vk::CommandPool> {
    let create_info = vk::CommandPoolCreateInfo::default().queue_family_index(queue_family_index).flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
    let pool = unsafe { device.create_command_pool(&create_info, None)? };
    Ok(pool)
}

pub fn create_command_buffers(device: &Device, command_pool: vk::CommandPool, count: u32) -> Result<Vec<vk::CommandBuffer>> {
    let alloc_info = vk::CommandBufferAllocateInfo::default().command_pool(command_pool).level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(count);
    let buffers = unsafe { device.allocate_command_buffers(&alloc_info)? };
    Ok(buffers)
}

pub fn record_frame(
    device: &Device, command_buffer: vk::CommandBuffer,
    hdr_render_pass: vk::RenderPass, hdr_framebuffer: vk::Framebuffer,
    bright_render_pass: vk::RenderPass, bloom_a_framebuffer: vk::Framebuffer,
    blur_render_pass: vk::RenderPass, bloom_b_framebuffer: vk::Framebuffer,
    swapchain_render_pass: vk::RenderPass, swapchain_framebuffer: vk::Framebuffer,
    // SSAO
    ssao_render_pass: vk::RenderPass, ssao_a_framebuffer: vk::Framebuffer, ssao_b_framebuffer: vk::Framebuffer,
    ssao_pipeline: vk::Pipeline, ssao_layout: vk::PipelineLayout, ssao_desc: vk::DescriptorSet,
    ssao_blur_pipeline: vk::Pipeline, ssao_blur_layout: vk::PipelineLayout,
    ssao_blur_desc_a: vk::DescriptorSet, ssao_blur_desc_b: vk::DescriptorSet,
    ssao_push: &[u8], // 2 mat4 = 128 bytes
    extent: vk::Extent2D, bloom_extent: vk::Extent2D,
    hdr_clear: [f32; 4], swapchain_clear: [f32; 4],
    world_pipeline: vk::Pipeline, world_layout: vk::PipelineLayout, world_desc: vk::DescriptorSet,
    visible_chunks: &[(vk::Buffer, u32)], push_data: &[u8],
    bright_pipeline: vk::Pipeline, bright_layout: vk::PipelineLayout, bright_desc: vk::DescriptorSet,
    blur_pipeline: vk::Pipeline, blur_layout: vk::PipelineLayout,
    blur_desc_a: vk::DescriptorSet, blur_desc_b: vk::DescriptorSet,
    post_pipeline: vk::Pipeline, post_layout: vk::PipelineLayout, post_desc: vk::DescriptorSet,
    ui_pipeline: vk::Pipeline, ui_layout: vk::PipelineLayout,
    ui_buffer: vk::Buffer, ui_count: u32, ui_desc: vk::DescriptorSet,
) -> Result<()> {
    unsafe { device.begin_command_buffer(command_buffer, &vk::CommandBufferBeginInfo::default())?; }

    let full_viewport = vk::Viewport { x: 0.0, y: 0.0, width: extent.width as f32, height: extent.height as f32, min_depth: 0.0, max_depth: 1.0 };
    let full_scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent };
    let bloom_viewport = vk::Viewport { x: 0.0, y: 0.0, width: bloom_extent.width as f32, height: bloom_extent.height as f32, min_depth: 0.0, max_depth: 1.0 };
    let bloom_scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: bloom_extent };

    // === 1. SCENE → HDR (с depth) ===
    {
        let cv = [
            vk::ClearValue { color: vk::ClearColorValue { float32: hdr_clear } },
            vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];
        let rp = vk::RenderPassBeginInfo::default().render_pass(hdr_render_pass).framebuffer(hdr_framebuffer).render_area(full_scissor).clear_values(&cv);
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[full_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[full_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, world_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, world_layout, 0, &[world_desc], &[]);
            device.cmd_push_constants(command_buffer, world_layout, vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, 0, push_data);
            for (buf, count) in visible_chunks {
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[*buf], &[0]);
                device.cmd_draw(command_buffer, *count, 1, 0, 0);
            }
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 2. SSAO PASS (depth → ssao_a) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [1.0, 1.0, 1.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(ssao_render_pass).framebuffer(ssao_a_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_layout, 0, &[ssao_desc], &[]);
            device.cmd_push_constants(command_buffer, ssao_layout, vk::ShaderStageFlags::FRAGMENT, 0, ssao_push);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 3. SSAO BLUR H (ssao_a → ssao_b) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [1.0, 1.0, 1.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(ssao_render_pass).framebuffer(ssao_b_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        let dir: [f32; 2] = [1.0, 0.0];
        let dir_bytes: &[u8] = unsafe { std::slice::from_raw_parts(dir.as_ptr() as *const u8, 8) };
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_blur_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_blur_layout, 0, &[ssao_blur_desc_a], &[]);
            device.cmd_push_constants(command_buffer, ssao_blur_layout, vk::ShaderStageFlags::FRAGMENT, 0, dir_bytes);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 4. SSAO BLUR V (ssao_b → ssao_a) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [1.0, 1.0, 1.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(ssao_render_pass).framebuffer(ssao_a_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        let dir: [f32; 2] = [0.0, 1.0];
        let dir_bytes: &[u8] = unsafe { std::slice::from_raw_parts(dir.as_ptr() as *const u8, 8) };
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_blur_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, ssao_blur_layout, 0, &[ssao_blur_desc_b], &[]);
            device.cmd_push_constants(command_buffer, ssao_blur_layout, vk::ShaderStageFlags::FRAGMENT, 0, dir_bytes);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 5. BRIGHT PASS (hdr → bloom_a) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(bright_render_pass).framebuffer(bloom_a_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, bright_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, bright_layout, 0, &[bright_desc], &[]);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 6. BLUR H (bloom_a → bloom_b) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(blur_render_pass).framebuffer(bloom_b_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        let dir: [f32; 2] = [1.0, 0.0];
        let dir_bytes: &[u8] = unsafe { std::slice::from_raw_parts(dir.as_ptr() as *const u8, 8) };
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, blur_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, blur_layout, 0, &[blur_desc_a], &[]);
            device.cmd_push_constants(command_buffer, blur_layout, vk::ShaderStageFlags::FRAGMENT, 0, dir_bytes);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 7. BLUR V (bloom_b → bloom_a) ===
    {
        let cv = [vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }];
        let rp = vk::RenderPassBeginInfo::default().render_pass(blur_render_pass).framebuffer(bloom_a_framebuffer).render_area(bloom_scissor).clear_values(&cv);
        let dir: [f32; 2] = [0.0, 1.0];
        let dir_bytes: &[u8] = unsafe { std::slice::from_raw_parts(dir.as_ptr() as *const u8, 8) };
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[bloom_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[bloom_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, blur_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, blur_layout, 0, &[blur_desc_b], &[]);
            device.cmd_push_constants(command_buffer, blur_layout, vk::ShaderStageFlags::FRAGMENT, 0, dir_bytes);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(command_buffer);
        }
    }

    // === 8. COMPOSITE + UI → swapchain ===
    {
        let cv = [
            vk::ClearValue { color: vk::ClearColorValue { float32: swapchain_clear } },
            vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } },
        ];
        let rp = vk::RenderPassBeginInfo::default().render_pass(swapchain_render_pass).framebuffer(swapchain_framebuffer).render_area(full_scissor).clear_values(&cv);
        unsafe {
            device.cmd_begin_render_pass(command_buffer, &rp, vk::SubpassContents::INLINE);
            device.cmd_set_viewport(command_buffer, 0, &[full_viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[full_scissor]);
            device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, post_pipeline);
            device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, post_layout, 0, &[post_desc], &[]);
            device.cmd_draw(command_buffer, 3, 1, 0, 0);
            if ui_count > 0 && ui_buffer != vk::Buffer::null() {
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, ui_pipeline);
                device.cmd_bind_descriptor_sets(command_buffer, vk::PipelineBindPoint::GRAPHICS, ui_layout, 0, &[ui_desc], &[]);
                device.cmd_bind_vertex_buffers(command_buffer, 0, &[ui_buffer], &[0]);
                device.cmd_draw(command_buffer, ui_count, 1, 0, 0);
            }
            device.cmd_end_render_pass(command_buffer);
        }
    }

    unsafe { device.end_command_buffer(command_buffer)?; }
    Ok(())
}

pub fn record_shadow_pass(
    device: &Device, command_buffer: vk::CommandBuffer,
    shadow_render_pass: vk::RenderPass, shadow_framebuffer: vk::Framebuffer,
    shadow_pipeline: vk::Pipeline, shadow_pipeline_layout: vk::PipelineLayout,
    visible_chunks: &[(vk::Buffer, u32)], // <-- Только видимые
    light_vp_bytes: &[u8],
) -> Result<()> {
    unsafe {
        let begin = vk::CommandBufferBeginInfo::default();
        device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
        device.begin_command_buffer(command_buffer, &begin)?;
        
        let clear_values = [vk::ClearValue { depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 } }];
        let size = 2048;
        let rp_begin = vk::RenderPassBeginInfo::default()
            .render_pass(shadow_render_pass).framebuffer(shadow_framebuffer)
            .render_area(vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: size, height: size } })
            .clear_values(&clear_values);
        let viewport = vk::Viewport { x: 0.0, y: 0.0, width: size as f32, height: size as f32, min_depth: 0.0, max_depth: 1.0 };
        let scissor = vk::Rect2D { offset: vk::Offset2D { x: 0, y: 0 }, extent: vk::Extent2D { width: size, height: size } };
        
        device.cmd_begin_render_pass(command_buffer, &rp_begin, vk::SubpassContents::INLINE);
        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, shadow_pipeline);
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(command_buffer, 0, &[scissor]);
        device.cmd_push_constants(command_buffer, shadow_pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, light_vp_bytes);
        
        for (buf, count) in visible_chunks {
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[*buf], &[0]);
            device.cmd_draw(command_buffer, *count, 1, 0, 0);
        }
        
        device.cmd_end_render_pass(command_buffer);
        device.end_command_buffer(command_buffer)?;
    }
    Ok(())
}
