//! OpenGL 3.3 core renderer backend.
//!
//! The pipeline mirrors the Vulkan backend: a shadow-map pass, an HDR forward
//! pass, half-resolution SSAO and bloom, an ACES composite and the UI pass.
//! The OpenGL backend consumes engine matrices directly because both use the
//! OpenGL clip-space convention (Y up, depth in `[-1, 1]`).

use anyhow::{Context, Result, anyhow, bail};
use assets::{
    ENVIRONMENT_SIZE, IRRADIANCE_SIZE, PREFILTERED_MIP_LEVELS, PREFILTERED_SIZE,
    generate_environment_cubemap_with_sun, generate_irradiance_cubemap,
    generate_prefiltered_cubemap,
};
use glam::{Mat4, Vec2, Vec4};
use glow::HasContext;
use render::{BackendKind, FrameData, RenderStats, Renderer, UiAtlasFrame, VsyncMode};
use scene::{Aabb, Light, LightUniformBuffer, PrimaryLight, Scene, Vertex};
use ui::UiVertex;

use crate::context::GlContext;
use crate::shader::ShaderProgram;

const SHADOW_MAP_SIZE: u32 = 2048;

struct GpuMesh {
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    vertex_count: u32,
    bounds: Aabb,
}

impl GpuMesh {
    unsafe fn upload(gl: &glow::Context, vertices: &[Vertex]) -> Result<Self> {
        let vao = unsafe { gl.create_vertex_array() }
            .map_err(|error| anyhow!("failed to create a vertex array: {error}"))?;
        let vbo = unsafe { gl.create_buffer() }
            .map_err(|error| anyhow!("failed to create a vertex buffer: {error}"))?;
        unsafe {
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(vertices),
                glow::STATIC_DRAW,
            );

            let stride = Vertex::STRIDE as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(
                0,
                3,
                glow::FLOAT,
                false,
                stride,
                Vertex::POSITION_OFFSET as i32,
            );
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                3,
                glow::FLOAT,
                false,
                stride,
                Vertex::COLOR_OFFSET as i32,
            );
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(
                2,
                3,
                glow::FLOAT,
                false,
                stride,
                Vertex::NORMAL_OFFSET as i32,
            );
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(
                3,
                2,
                glow::FLOAT,
                false,
                stride,
                Vertex::UV_OFFSET as i32,
            );
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_f32(
                4,
                1,
                glow::FLOAT,
                false,
                stride,
                Vertex::EMISSION_OFFSET as i32,
            );
            gl.enable_vertex_attrib_array(5);
            gl.vertex_attrib_pointer_f32(
                5,
                2,
                glow::FLOAT,
                false,
                stride,
                Vertex::MATERIAL_OFFSET as i32,
            );

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
        Ok(Self {
            vao,
            vbo,
            vertex_count: vertices.len() as u32,
            bounds: Aabb::ZERO,
        })
    }

    unsafe fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
        }
    }
}

struct GpuScene {
    meshes: Vec<GpuMesh>,
    albedo: glow::Texture,
    lights: Vec<Light>,
    primary_light: PrimaryLight,
    metallic: f32,
    roughness: f32,
    clear_color: [f32; 4],
    total_vertices: u64,
}

impl GpuScene {
    unsafe fn upload(gl: &glow::Context, scene: &Scene) -> Result<Self> {
        let mut meshes = Vec::with_capacity(scene.meshes.len());
        let mut total_vertices = 0u64;
        for mesh in &scene.meshes {
            if mesh.vertices.is_empty() {
                continue;
            }
            let mut gpu = unsafe { GpuMesh::upload(gl, &mesh.vertices) }?;
            gpu.bounds = mesh.bounds;
            total_vertices += mesh.vertices.len() as u64;
            meshes.push(gpu);
        }

        let albedo = unsafe {
            texture_2d_rgba8(
                gl,
                scene.material.albedo.width,
                scene.material.albedo.height,
                &scene.material.albedo.pixels,
                false,
            )?
        };

        Ok(Self {
            meshes,
            albedo,
            lights: scene.lights.clone(),
            primary_light: scene.primary_light,
            metallic: scene.material.metallic.clamp(0.0, 1.0),
            roughness: scene.material.roughness.clamp(0.04, 1.0),
            clear_color: scene.clear_color,
            total_vertices,
        })
    }
}

/// Texture format used for the HDR intermediate targets.
///
/// Desktop GL supports RGBA16F color attachments directly. OpenGL ES 3.0 only
/// guarantees RGBA16F as a render target when `EXT_color_buffer_half_float`
/// (or `EXT_color_buffer_float`) is present; when neither extension exists the
/// renderer falls back to RGBA8 so that bloom and SSAO still run, albeit without
/// values above 1.0 in the HDR buffer.
#[derive(Clone, Copy)]
struct HdrFormat {
    internal: i32,
    format: u32,
    data_type: u32,
}

fn hdr_format(gl: &glow::Context) -> HdrFormat {
    #[cfg(target_os = "android")]
    {
        let extensions = gl.supported_extensions();
        if extensions.contains("EXT_color_buffer_half_float")
            || extensions.contains("EXT_color_buffer_float")
        {
            return HdrFormat {
                internal: glow::RGBA16F as i32,
                format: glow::RGBA,
                data_type: glow::FLOAT,
            };
        }
        println!(
            "[engine-opengl] EXT_color_buffer_half_float unavailable; using RGBA8 HDR targets"
        );
        return HdrFormat {
            internal: glow::RGBA8 as i32,
            format: glow::RGBA,
            data_type: glow::UNSIGNED_BYTE,
        };
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = gl;
        HdrFormat {
            internal: glow::RGBA16F as i32,
            format: glow::RGBA,
            data_type: glow::FLOAT,
        }
    }
}

/// Swapchain-sized attachments. Recreated atomically on resize.
struct Targets {
    width: u32,
    height: u32,
    half_width: u32,
    half_height: u32,
    hdr_fbo: glow::Framebuffer,
    hdr_color: glow::Texture,
    hdr_depth: glow::Texture,
    bloom_a_fbo: glow::Framebuffer,
    bloom_a: glow::Texture,
    bloom_b_fbo: glow::Framebuffer,
    bloom_b: glow::Texture,
    ssao_a_fbo: glow::Framebuffer,
    ssao_a: glow::Texture,
    ssao_b_fbo: glow::Framebuffer,
    ssao_b: glow::Texture,
}

impl Targets {
    unsafe fn create(gl: &glow::Context, width: u32, height: u32, hdr: HdrFormat) -> Result<Self> {
        let width = width.max(1);
        let height = height.max(1);
        let half_width = (width / 2).max(1);
        let half_height = (height / 2).max(1);

        let hdr_color =
            unsafe { color_texture(gl, width, height, hdr.internal, hdr.format, hdr.data_type)? };
        let hdr_depth = unsafe { depth_texture(gl, width, height) }?;
        let hdr_fbo = unsafe { framebuffer(gl, Some(hdr_color), Some(hdr_depth)) }?;

        let bloom_a = unsafe {
            color_texture(
                gl,
                half_width,
                half_height,
                hdr.internal,
                hdr.format,
                hdr.data_type,
            )?
        };
        let bloom_b = unsafe {
            color_texture(
                gl,
                half_width,
                half_height,
                hdr.internal,
                hdr.format,
                hdr.data_type,
            )?
        };
        let bloom_a_fbo = unsafe { framebuffer(gl, Some(bloom_a), None) }?;
        let bloom_b_fbo = unsafe { framebuffer(gl, Some(bloom_b), None) }?;

        let ssao_a = unsafe {
            color_texture(
                gl,
                half_width,
                half_height,
                glow::R8 as i32,
                glow::RED,
                glow::UNSIGNED_BYTE,
            )?
        };
        let ssao_b = unsafe {
            color_texture(
                gl,
                half_width,
                half_height,
                glow::R8 as i32,
                glow::RED,
                glow::UNSIGNED_BYTE,
            )?
        };
        let ssao_a_fbo = unsafe { framebuffer(gl, Some(ssao_a), None) }?;
        let ssao_b_fbo = unsafe { framebuffer(gl, Some(ssao_b), None) }?;

        Ok(Self {
            width,
            height,
            half_width,
            half_height,
            hdr_fbo,
            hdr_color,
            hdr_depth,
            bloom_a_fbo,
            bloom_a,
            bloom_b_fbo,
            bloom_b,
            ssao_a_fbo,
            ssao_a,
            ssao_b_fbo,
            ssao_b,
        })
    }

    unsafe fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_framebuffer(self.hdr_fbo);
            gl.delete_framebuffer(self.bloom_a_fbo);
            gl.delete_framebuffer(self.bloom_b_fbo);
            gl.delete_framebuffer(self.ssao_a_fbo);
            gl.delete_framebuffer(self.ssao_b_fbo);
            gl.delete_texture(self.hdr_color);
            gl.delete_texture(self.hdr_depth);
            gl.delete_texture(self.bloom_a);
            gl.delete_texture(self.bloom_b);
            gl.delete_texture(self.ssao_a);
            gl.delete_texture(self.ssao_b);
        }
    }
}

/// OpenGL implementation of the engine renderer.
pub struct OpenGlRenderer {
    context: GlContext,
    scene: GpuScene,
    targets: Option<Targets>,

    shadow_fbo: glow::Framebuffer,
    shadow_depth: glow::Texture,
    /// Static geometry is rendered into the shadow map once at startup.
    shadow_ready: bool,

    world_program: ShaderProgram,
    shadow_program: ShaderProgram,
    bright_program: ShaderProgram,
    blur_program: ShaderProgram,
    ssao_program: ShaderProgram,
    ssao_blur_program: ShaderProgram,
    composite_program: ShaderProgram,
    ui_program: ShaderProgram,

    fullscreen_vao: glow::VertexArray,
    ui_vao: glow::VertexArray,
    ui_vbo: glow::Buffer,
    light_ubo: glow::Buffer,

    font: glow::Texture,
    font_size: (u32, u32),
    irradiance: glow::Texture,
    prefiltered: glow::Texture,

    stats: RenderStats,
    size: (u32, u32),
    vsync: VsyncMode,
    capture_requested: bool,
}

impl OpenGlRenderer {
    pub fn new(surface: render::WindowSurface, scene: &Scene, vsync: VsyncMode) -> Result<Self> {
        let size = surface.size();
        let context = GlContext::new(surface)?;
        let gl = context.glow();

        // Apply vsync immediately
        let _ = context.set_vsync(vsync);

        unsafe { gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1) };

        let scene = unsafe { GpuScene::upload(gl, scene) }?;

        let shadow_depth = unsafe { depth_texture(gl, SHADOW_MAP_SIZE, SHADOW_MAP_SIZE) }?;
        let shadow_fbo = unsafe { depth_only_framebuffer(gl, shadow_depth) }?;

        let targets = unsafe { Targets::create(gl, size.0, size.1, hdr_format(gl)) }?;

        let fullscreen_vao = unsafe { gl.create_vertex_array() }
            .map_err(|error| anyhow!("failed to create the fullscreen vertex array: {error}"))?;

        let ui_vao = unsafe { gl.create_vertex_array() }
            .map_err(|error| anyhow!("failed to create the UI vertex array: {error}"))?;
        let ui_vbo = unsafe { gl.create_buffer() }
            .map_err(|error| anyhow!("failed to create the UI vertex buffer: {error}"))?;
        unsafe {
            gl.bind_vertex_array(Some(ui_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(ui_vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &[], glow::DYNAMIC_DRAW);
            let stride = UiVertex::STRIDE as i32;
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 4, glow::FLOAT, false, stride, 8);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 24);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }

        let light_ubo = unsafe { gl.create_buffer() }
            .map_err(|error| anyhow!("failed to create the light UBO: {error}"))?;
        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(light_ubo));
            gl.buffer_data_u8_slice(
                glow::UNIFORM_BUFFER,
                bytemuck::bytes_of(&LightUniformBuffer::default()),
                glow::DYNAMIC_DRAW,
            );
            gl.bind_buffer(glow::UNIFORM_BUFFER, None);
        }

        // The UI owns its glyph atlas and uploads it on demand. Start with a
        // 1x1 white placeholder so the sampler binding is always valid.
        let font_size = (1, 1);
        let font = unsafe { texture_2d_r8(gl, 1, 1, &[255]) }?;

        let sun_direction =
            (scene.primary_light.position - scene.primary_light.target).normalize_or_zero();
        let environment = generate_environment_cubemap_with_sun(ENVIRONMENT_SIZE, sun_direction);
        let irradiance_pixels =
            generate_irradiance_cubemap(&environment, ENVIRONMENT_SIZE, IRRADIANCE_SIZE);
        let prefiltered_pixels = generate_prefiltered_cubemap(
            &environment,
            ENVIRONMENT_SIZE,
            PREFILTERED_SIZE,
            PREFILTERED_MIP_LEVELS,
        );
        let irradiance = unsafe { cubemap_environment(gl, IRRADIANCE_SIZE, &[irradiance_pixels]) }?;
        let prefiltered =
            unsafe { cubemap_environment(gl, PREFILTERED_SIZE, &prefiltered_pixels) }?;

        let mut world_program = ShaderProgram::new(
            gl,
            "world",
            include_str!("../shaders/world.vert"),
            include_str!("../shaders/world.frag"),
        )?;
        let shadow_program = ShaderProgram::new(
            gl,
            "shadow",
            include_str!("../shaders/shadow.vert"),
            include_str!("../shaders/shadow.frag"),
        )?;
        let mut bright_program = ShaderProgram::new(
            gl,
            "bloom_bright",
            include_str!("../shaders/fullscreen.vert"),
            include_str!("../shaders/bloom_bright.frag"),
        )?;
        let mut blur_program = ShaderProgram::new(
            gl,
            "blur",
            include_str!("../shaders/fullscreen.vert"),
            include_str!("../shaders/blur.frag"),
        )?;
        let mut ssao_program = ShaderProgram::new(
            gl,
            "ssao",
            include_str!("../shaders/fullscreen.vert"),
            include_str!("../shaders/ssao.frag"),
        )?;
        let mut ssao_blur_program = ShaderProgram::new(
            gl,
            "ssao_blur",
            include_str!("../shaders/fullscreen.vert"),
            include_str!("../shaders/ssao_blur.frag"),
        )?;
        let mut composite_program = ShaderProgram::new(
            gl,
            "post_process",
            include_str!("../shaders/fullscreen.vert"),
            include_str!("../shaders/post_process.frag"),
        )?;
        let mut ui_program = ShaderProgram::new(
            gl,
            "ui",
            include_str!("../shaders/ui.vert"),
            include_str!("../shaders/ui.frag"),
        )?;

        unsafe {
            world_program.bind(gl);
            bind_sampler(gl, &mut world_program, "world_sampler", 0);
            bind_sampler(gl, &mut world_program, "shadow_sampler", 1);
            bind_sampler(gl, &mut world_program, "irradiance_map", 2);
            bind_sampler(gl, &mut world_program, "prefiltered_map", 3);
            if let Some(index) = gl.get_uniform_block_index(world_program.raw(), "LightBuffer") {
                gl.uniform_block_binding(world_program.raw(), index, 0);
            }

            bright_program.bind(gl);
            bind_sampler(gl, &mut bright_program, "hdr_input", 0);
            blur_program.bind(gl);
            bind_sampler(gl, &mut blur_program, "blur_input", 0);
            ssao_program.bind(gl);
            bind_sampler(gl, &mut ssao_program, "depth_input", 0);
            ssao_blur_program.bind(gl);
            bind_sampler(gl, &mut ssao_blur_program, "ssao_input", 0);
            composite_program.bind(gl);
            bind_sampler(gl, &mut composite_program, "hdr_input", 0);
            bind_sampler(gl, &mut composite_program, "bloom_input", 1);
            bind_sampler(gl, &mut composite_program, "ssao_input", 2);
            ui_program.bind(gl);
            bind_sampler(gl, &mut ui_program, "font_sampler", 0);

            gl.use_program(None);
            gl.bind_vertex_array(None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        }

        let stats = RenderStats {
            vertices: scene.total_vertices,
            triangles: scene.total_vertices / 3,
            draw_calls: 0,
        };

        println!(
            "[engine-opengl] ready: {} vertices in {} chunks, {} lights, {}",
            scene.total_vertices,
            scene.meshes.len(),
            scene.lights.len(),
            context.version()
        );

        Ok(Self {
            context,
            scene,
            targets: Some(targets),
            shadow_fbo,
            shadow_depth,
            shadow_ready: false,
            world_program,
            shadow_program,
            bright_program,
            blur_program,
            ssao_program,
            ssao_blur_program,
            composite_program,
            ui_program,
            fullscreen_vao,
            ui_vao,
            ui_vbo,
            light_ubo,
            font,
            font_size,
            irradiance,
            prefiltered,
            stats,
            size,
            vsync,
            capture_requested: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.size = (width, height);
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.context.resize(width, height)?;
        let gl = self.context.glow();
        if let Some(targets) = self.targets.take() {
            unsafe { targets.destroy(gl) };
        }
        self.targets = Some(unsafe { Targets::create(gl, width, height, hdr_format(gl)) }?);
        Ok(())
    }

    pub fn render(&mut self, frame: &FrameData<'_>) -> Result<()> {
        if let Some(atlas) = frame.ui_atlas.as_ref() {
            self.upload_ui_atlas(atlas)?;
        }
        let gl = self.context.glow();
        let targets = self
            .targets
            .as_ref()
            .context("OpenGL targets are not initialized")?;
        let light = self.scene.primary_light;
        let light_view_projection = light.view_projection();
        let view_projection = frame.camera.view_projection();

        let lights = LightUniformBuffer::from_nearest(&self.scene.lights, frame.camera.position);
        unsafe {
            gl.bind_buffer(glow::UNIFORM_BUFFER, Some(self.light_ubo));
            gl.buffer_sub_data_u8_slice(glow::UNIFORM_BUFFER, 0, bytemuck::bytes_of(&lights));
            gl.bind_buffer(glow::UNIFORM_BUFFER, None);
        }

        let shadow_planes = extract_frustum_planes(light_view_projection);
        let camera_planes = extract_frustum_planes(view_projection);
        let shadow_visible = visible_meshes(&self.scene.meshes, &shadow_planes);
        let visible = visible_meshes(&self.scene.meshes, &camera_planes);
        self.stats.draw_calls = visible.len() as u32;

        // 1. Shadow map. The demo level is static, so the depth map is baked
        // once and reused for the whole session.
        if !self.shadow_ready {
            unsafe {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.shadow_fbo));
                gl.viewport(0, 0, SHADOW_MAP_SIZE as i32, SHADOW_MAP_SIZE as i32);
                gl.enable(glow::DEPTH_TEST);
                gl.depth_mask(true);
                gl.depth_func(glow::LESS);
                gl.disable(glow::BLEND);
                gl.disable(glow::CULL_FACE);
                gl.clear(glow::DEPTH_BUFFER_BIT);

                self.shadow_program.bind(gl);
                self.shadow_program
                    .set_mat4(gl, "u_light_vp", &light_view_projection);
                for &index in &shadow_visible {
                    let mesh = &self.scene.meshes[index];
                    gl.bind_vertex_array(Some(mesh.vao));
                    gl.draw_arrays(glow::TRIANGLES, 0, mesh.vertex_count as i32);
                }
            }
            self.shadow_ready = true;
        }

        // 2. Forward pass into HDR + depth.
        let primary_color = light.color * light.intensity;
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(targets.hdr_fbo));
            gl.viewport(0, 0, targets.width as i32, targets.height as i32);
            gl.enable(glow::DEPTH_TEST);
            gl.depth_mask(true);
            gl.depth_func(glow::LESS);
            gl.disable(glow::BLEND);
            gl.disable(glow::CULL_FACE);
            let clear = self.scene.clear_color;
            gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            self.world_program.bind(gl);
            self.world_program.set_mat4(gl, "u_mvp", &view_projection);
            self.world_program
                .set_mat4(gl, "u_light_vp", &light_view_projection);
            self.world_program.set_vec4(
                gl,
                "u_light_pos",
                light.position.extend(light.shadow.ortho_half_extent),
            );
            self.world_program
                .set_vec4(gl, "u_light_color", primary_color.extend(1.0));
            self.world_program
                .set_vec4(gl, "u_camera_pos", frame.camera.position.extend(1.0));
            self.world_program.set_vec4(
                gl,
                "u_screen_and_debug",
                Vec4::new(
                    targets.width as f32,
                    targets.height as f32,
                    frame.debug_view.as_u32() as f32,
                    0.0,
                ),
            );
            self.world_program.set_vec4(
                gl,
                "u_material",
                Vec4::new(self.scene.metallic, self.scene.roughness, 0.0, 0.0),
            );

            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.scene.albedo));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow_depth));
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(self.irradiance));
            gl.active_texture(glow::TEXTURE3);
            gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(self.prefiltered));
            gl.bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(self.light_ubo));

            for &index in &visible {
                let mesh = &self.scene.meshes[index];
                gl.bind_vertex_array(Some(mesh.vao));
                gl.draw_arrays(glow::TRIANGLES, 0, mesh.vertex_count as i32);
            }
        }

        // 3. SSAO from the depth buffer, then separable blur.
        unsafe {
            begin_fullscreen_pass(
                gl,
                targets.ssao_a_fbo,
                targets.half_width,
                targets.half_height,
                [1.0, 1.0, 1.0, 1.0],
            );
            self.ssao_program.bind(gl);
            self.ssao_program
                .set_mat4(gl, "u_proj", &frame.camera.projection);
            self.ssao_program
                .set_mat4(gl, "u_inv_proj", &frame.camera.projection.inverse());
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(targets.hdr_depth));
            draw_fullscreen(gl, self.fullscreen_vao);
        }
        unsafe {
            blur_pass(
                gl,
                targets.ssao_b_fbo,
                targets.ssao_a,
                targets.half_width,
                targets.half_height,
                Vec2::new(1.0, 0.0),
                [1.0, 1.0, 1.0, 1.0],
                &mut self.ssao_blur_program,
                self.fullscreen_vao,
            );
            blur_pass(
                gl,
                targets.ssao_a_fbo,
                targets.ssao_b,
                targets.half_width,
                targets.half_height,
                Vec2::new(0.0, 1.0),
                [1.0, 1.0, 1.0, 1.0],
                &mut self.ssao_blur_program,
                self.fullscreen_vao,
            );
        }

        // 4. Bloom bright pass and blur.
        unsafe {
            begin_fullscreen_pass(
                gl,
                targets.bloom_a_fbo,
                targets.half_width,
                targets.half_height,
                [0.0, 0.0, 0.0, 1.0],
            );
            self.bright_program.bind(gl);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(targets.hdr_color));
            draw_fullscreen(gl, self.fullscreen_vao);
        }
        unsafe {
            blur_pass(
                gl,
                targets.bloom_b_fbo,
                targets.bloom_a,
                targets.half_width,
                targets.half_height,
                Vec2::new(1.0, 0.0),
                [0.0, 0.0, 0.0, 1.0],
                &mut self.blur_program,
                self.fullscreen_vao,
            );
            blur_pass(
                gl,
                targets.bloom_a_fbo,
                targets.bloom_b,
                targets.half_width,
                targets.half_height,
                Vec2::new(0.0, 1.0),
                [0.0, 0.0, 0.0, 1.0],
                &mut self.blur_program,
                self.fullscreen_vao,
            );
        }

        // 5. Composite to the default framebuffer.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, targets.width as i32, targets.height as i32);
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::BLEND);
            gl.clear_color(0.0, 0.0, 0.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            self.composite_program.bind(gl);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(targets.hdr_color));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(targets.bloom_a));
            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, Some(targets.ssao_a));
            draw_fullscreen(gl, self.fullscreen_vao);
        }

        // 6. UI in physical pixels on top of the LDR image.
        if !frame.ui_vertices.is_empty() {
            unsafe {
                gl.enable(glow::BLEND);
                gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
                gl.disable(glow::DEPTH_TEST);
                gl.depth_mask(false);

                self.ui_program.bind(gl);
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.font));
                gl.bind_vertex_array(Some(self.ui_vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.ui_vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    bytemuck::cast_slice(frame.ui_vertices),
                    glow::DYNAMIC_DRAW,
                );
                gl.draw_arrays(glow::TRIANGLES, 0, frame.ui_vertices.len() as i32);
                gl.disable(glow::BLEND);
                gl.bind_vertex_array(None);
            }
        }

        // Capture the back buffer before it is presented. The UI pass above has
        // already drawn into the default framebuffer, so the PNG matches the
        // frame the user is looking at.
        let capture_result = if self.capture_requested {
            self.capture_requested = false;
            self.capture_framebuffer()
        } else {
            Ok(())
        };

        self.context.swap_buffers()?;
        capture_result?;
        self.stats.vertices = self.scene.total_vertices;
        self.stats.triangles = self.scene.total_vertices / 3;
        Ok(())
    }

    /// Uploads the R8 glyph atlas. The texture is recreated when its size
    /// changes; otherwise only the dirty sub-rectangles are copied.
    fn upload_ui_atlas(&mut self, atlas: &UiAtlasFrame<'_>) -> Result<()> {
        let gl = self.context.glow();
        let size = (atlas.width.max(1), atlas.height.max(1));
        if atlas.full || self.font_size != size {
            unsafe { gl.delete_texture(self.font) };
            self.font = unsafe { texture_2d_r8(gl, size.0, size.1, atlas.pixels) }?;
            self.font_size = size;
            return Ok(());
        }

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.font));
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, size.0 as i32);
            for rect in atlas.dirty {
                if rect.w == 0 || rect.h == 0 {
                    continue;
                }
                let offset = (rect.y * size.0 + rect.x) as usize;
                let Some(slice) = atlas.pixels.get(offset..) else {
                    continue;
                };
                gl.tex_sub_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    rect.x as i32,
                    rect.y as i32,
                    rect.w as i32,
                    rect.h as i32,
                    glow::RED,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(Some(slice)),
                );
            }
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
            gl.bind_texture(glow::TEXTURE_2D, None);
        }
        Ok(())
    }

    fn capture_framebuffer(&self) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let filename = format!("screenshot_{timestamp}.png");
        let gl = self.context.glow();
        let (width, height) = self.extent();
        if width == 0 || height == 0 {
            anyhow::bail!("cannot capture a zero-sized framebuffer");
        }
        let pixels = unsafe {
            let mut data = vec![0u8; (width * height * 4) as usize];
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut data)),
            );
            data
        };
        // Flip vertically because OpenGL origin is bottom-left.
        let mut flipped = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            let src_row = &pixels[(y * width * 4) as usize..((y + 1) * width * 4) as usize];
            let dst_row = &mut flipped
                [((height - 1 - y) * width * 4) as usize..((height - y) * width * 4) as usize];
            dst_row.copy_from_slice(src_row);
        }
        let mut file = std::fs::File::create(&filename)
            .with_context(|| format!("failed to create {filename}"))?;
        let mut encoder = png::Encoder::new(&mut file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .context("failed to write PNG header")?;
        writer
            .write_image_data(&flipped)
            .context("failed to write PNG image data")?;
        eprintln!("[engine-opengl] screenshot saved to {filename}");
        Ok(())
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe { self.context.glow().finish() };
        Ok(())
    }

    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    pub fn extent(&self) -> (u32, u32) {
        self.size
    }

    pub fn vsync(&self) -> VsyncMode {
        self.vsync
    }

    pub fn set_vsync(&mut self, vsync: VsyncMode) -> Result<()> {
        self.vsync = vsync;
        self.context.set_vsync(vsync)
    }
}

impl Renderer for OpenGlRenderer {
    fn backend(&self) -> BackendKind {
        BackendKind::OpenGl
    }

    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        OpenGlRenderer::resize(self, width, height)
    }

    fn render(&mut self, frame: &FrameData<'_>) -> Result<()> {
        OpenGlRenderer::render(self, frame)
    }

    fn wait_idle(&self) -> Result<()> {
        OpenGlRenderer::wait_idle(self)
    }

    fn stats(&self) -> RenderStats {
        OpenGlRenderer::stats(self)
    }

    fn extent(&self) -> (u32, u32) {
        OpenGlRenderer::extent(self)
    }

    fn set_vsync(&mut self, vsync: VsyncMode) -> Result<()> {
        OpenGlRenderer::set_vsync(self, vsync)
    }

    fn vsync(&self) -> VsyncMode {
        self.vsync
    }

    fn screenshot(&mut self) -> Result<()> {
        // The capture has to happen after the UI pass has drawn into the
        // default framebuffer. `render` performs it right before swapping.
        self.capture_requested = true;
        Ok(())
    }
}

impl Drop for OpenGlRenderer {
    fn drop(&mut self) {
        let gl = self.context.glow();
        unsafe {
            for mesh in &self.scene.meshes {
                mesh.destroy(gl);
            }
            gl.delete_texture(self.scene.albedo);
            gl.delete_framebuffer(self.shadow_fbo);
            gl.delete_texture(self.shadow_depth);
            gl.delete_texture(self.font);
            gl.delete_texture(self.irradiance);
            gl.delete_texture(self.prefiltered);
            gl.delete_vertex_array(self.fullscreen_vao);
            gl.delete_vertex_array(self.ui_vao);
            gl.delete_buffer(self.ui_vbo);
            gl.delete_buffer(self.light_ubo);
            if let Some(targets) = self.targets.take() {
                targets.destroy(gl);
            }
        }
        for program in [
            &self.world_program,
            &self.shadow_program,
            &self.bright_program,
            &self.blur_program,
            &self.ssao_program,
            &self.ssao_blur_program,
            &self.composite_program,
            &self.ui_program,
        ] {
            program.destroy(gl);
        }
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn blur_pass(
    gl: &glow::Context,
    fbo: glow::Framebuffer,
    source: glow::Texture,
    width: u32,
    height: u32,
    direction: Vec2,
    clear: [f32; 4],
    program: &mut ShaderProgram,
    fullscreen_vao: glow::VertexArray,
) {
    unsafe {
        begin_fullscreen_pass(gl, fbo, width, height, clear);
        program.bind(gl);
        program.set_vec2(gl, "u_direction", direction);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(source));
        draw_fullscreen(gl, fullscreen_vao);
    }
}

unsafe fn bind_sampler(gl: &glow::Context, program: &mut ShaderProgram, name: &str, unit: i32) {
    if let Some(location) = program.uniform_location(gl, name) {
        unsafe { gl.uniform_1_i32(Some(&location), unit) };
    }
}

unsafe fn begin_fullscreen_pass(
    gl: &glow::Context,
    fbo: glow::Framebuffer,
    width: u32,
    height: u32,
    clear: [f32; 4],
) {
    unsafe {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.viewport(0, 0, width as i32, height as i32);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.disable(glow::CULL_FACE);
        gl.disable(glow::BLEND);
        gl.clear_color(clear[0], clear[1], clear[2], clear[3]);
        gl.clear(glow::COLOR_BUFFER_BIT);
    }
}

unsafe fn draw_fullscreen(gl: &glow::Context, vao: glow::VertexArray) {
    unsafe {
        gl.bind_vertex_array(Some(vao));
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
    }
}

unsafe fn texture_2d_r8(
    gl: &glow::Context,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<glow::Texture> {
    let texture = unsafe { gl.create_texture() }
        .map_err(|error| anyhow!("failed to create the UI atlas texture: {error}"))?;
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::R8 as i32,
            width as i32,
            height as i32,
            0,
            glow::RED,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
    Ok(texture)
}

unsafe fn texture_2d_rgba8(
    gl: &glow::Context,
    width: u32,
    height: u32,
    pixels: &[u8],
    nearest_filter: bool,
) -> Result<glow::Texture> {
    let texture = unsafe { gl.create_texture() }
        .map_err(|error| anyhow!("failed to create a texture: {error}"))?;
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(pixels)),
        );
        // World UVs are tiled in world space and routinely exceed 1.0.
        // Repeat wrapping matches the Vulkan backend; clamping stretched the
        // last texel across entire walls and looked blurry.
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        if nearest_filter {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
        } else {
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR_MIPMAP_LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.generate_mipmap(glow::TEXTURE_2D);
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
    Ok(texture)
}

unsafe fn cubemap_environment(
    gl: &glow::Context,
    base_size: u32,
    levels: &[Vec<f32>],
) -> Result<glow::Texture> {
    let texture = unsafe { gl.create_texture() }
        .map_err(|error| anyhow!("failed to create a cubemap: {error}"))?;
    // RGBA32F is filterable on desktop but not guaranteed by OpenGL ES 3.0.
    // RGBA16F is core-filterable there and more than enough for the procedural
    // environment (values peak around 50).
    #[cfg(target_os = "android")]
    let internal_format = glow::RGBA16F as i32;
    #[cfg(not(target_os = "android"))]
    let internal_format = glow::RGBA32F as i32;
    unsafe {
        gl.bind_texture(glow::TEXTURE_CUBE_MAP, Some(texture));
        for (mip, pixels) in levels.iter().enumerate() {
            let size = (base_size >> mip).max(1);
            let bytes: &[u8] = bytemuck::cast_slice(pixels);
            let face_size = bytes.len() / 6;
            for face in 0..6 {
                let start = face * face_size;
                gl.tex_image_2d(
                    glow::TEXTURE_CUBE_MAP_POSITIVE_X + face as u32,
                    mip as i32,
                    internal_format,
                    size as i32,
                    size as i32,
                    0,
                    glow::RGBA,
                    glow::FLOAT,
                    glow::PixelUnpackData::Slice(Some(&bytes[start..start + face_size])),
                );
            }
        }
        let has_mips = levels.len() > 1;
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_MIN_FILTER,
            if has_mips {
                glow::LINEAR_MIPMAP_LINEAR as i32
            } else {
                glow::LINEAR as i32
            },
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_MAX_LEVEL,
            (levels.len().saturating_sub(1)) as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_CUBE_MAP,
            glow::TEXTURE_WRAP_R,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.bind_texture(glow::TEXTURE_CUBE_MAP, None);
    }
    Ok(texture)
}

unsafe fn color_texture(
    gl: &glow::Context,
    width: u32,
    height: u32,
    internal_format: i32,
    format: u32,
    data_type: u32,
) -> Result<glow::Texture> {
    let texture = unsafe { gl.create_texture() }
        .map_err(|error| anyhow!("failed to create a color target: {error}"))?;
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal_format,
            width as i32,
            height as i32,
            0,
            format,
            data_type,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
    Ok(texture)
}

unsafe fn depth_texture(gl: &glow::Context, width: u32, height: u32) -> Result<glow::Texture> {
    let texture = unsafe { gl.create_texture() }
        .map_err(|error| anyhow!("failed to create a depth target: {error}"))?;
    // CLAMP_TO_BORDER is core in desktop GL but not in OpenGL ES 3.0.
    #[cfg(target_os = "android")]
    let border_wrap = glow::CLAMP_TO_EDGE;
    #[cfg(not(target_os = "android"))]
    let border_wrap = glow::CLAMP_TO_BORDER;
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::DEPTH_COMPONENT24 as i32,
            width as i32,
            height as i32,
            0,
            glow::DEPTH_COMPONENT,
            glow::UNSIGNED_INT,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, border_wrap as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, border_wrap as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_COMPARE_MODE,
            glow::NONE as i32,
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
    Ok(texture)
}

unsafe fn framebuffer(
    gl: &glow::Context,
    color: Option<glow::Texture>,
    depth: Option<glow::Texture>,
) -> Result<glow::Framebuffer> {
    let fbo = unsafe { gl.create_framebuffer() }
        .map_err(|error| anyhow!("failed to create a framebuffer: {error}"))?;
    unsafe {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            color,
            0,
        );
        if let Some(depth) = depth {
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(depth),
                0,
            );
        }
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        if status != glow::FRAMEBUFFER_COMPLETE {
            gl.delete_framebuffer(fbo);
            bail!("OpenGL framebuffer is incomplete (status 0x{status:04X})");
        }
    }
    Ok(fbo)
}

unsafe fn depth_only_framebuffer(
    gl: &glow::Context,
    depth: glow::Texture,
) -> Result<glow::Framebuffer> {
    let fbo = unsafe { gl.create_framebuffer() }
        .map_err(|error| anyhow!("failed to create the shadow framebuffer: {error}"))?;
    unsafe {
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::DEPTH_ATTACHMENT,
            glow::TEXTURE_2D,
            Some(depth),
            0,
        );
        // `glDrawBuffer` is desktop-only; OpenGL ES 3.0 uses `glDrawBuffers`.
        #[cfg(target_os = "android")]
        gl.draw_buffers(&[glow::NONE]);
        #[cfg(not(target_os = "android"))]
        gl.draw_buffer(glow::NONE);
        gl.read_buffer(glow::NONE);
        let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        if status != glow::FRAMEBUFFER_COMPLETE {
            gl.delete_framebuffer(fbo);
            bail!("OpenGL shadow framebuffer is incomplete (status 0x{status:04X})");
        }
    }
    Ok(fbo)
}

fn visible_meshes(meshes: &[GpuMesh], planes: &[Vec4; 6]) -> Vec<usize> {
    meshes
        .iter()
        .enumerate()
        .filter(|(_, mesh)| frustum_aabb_intersect(planes, &mesh.bounds))
        .map(|(index, _)| index)
        .collect()
}

fn extract_frustum_planes(matrix: Mat4) -> [Vec4; 6] {
    let m = matrix.to_cols_array_2d();
    let mut planes = [Vec4::ZERO; 6];
    planes[0] = Vec4::new(
        m[0][3] + m[0][0],
        m[1][3] + m[1][0],
        m[2][3] + m[2][0],
        m[3][3] + m[3][0],
    );
    planes[1] = Vec4::new(
        m[0][3] - m[0][0],
        m[1][3] - m[1][0],
        m[2][3] - m[2][0],
        m[3][3] - m[3][0],
    );
    planes[2] = Vec4::new(
        m[0][3] + m[0][1],
        m[1][3] + m[1][1],
        m[2][3] + m[2][1],
        m[3][3] + m[3][1],
    );
    planes[3] = Vec4::new(
        m[0][3] - m[0][1],
        m[1][3] - m[1][1],
        m[2][3] - m[2][1],
        m[3][3] - m[3][1],
    );
    planes[4] = Vec4::new(
        m[0][3] + m[0][2],
        m[1][3] + m[1][2],
        m[2][3] + m[2][2],
        m[3][3] + m[3][2],
    );
    planes[5] = Vec4::new(
        m[0][3] - m[0][2],
        m[1][3] - m[1][2],
        m[2][3] - m[2][2],
        m[3][3] - m[3][2],
    );
    for plane in &mut planes {
        *plane = plane.normalize();
    }
    planes
}

fn frustum_aabb_intersect(planes: &[Vec4; 6], aabb: &Aabb) -> bool {
    let center = aabb.center();
    let extents = aabb.half_extents();
    for plane in planes {
        let radius =
            extents.x * plane.x.abs() + extents.y * plane.y.abs() + extents.z * plane.z.abs();
        if radius + plane.dot(center.extend(1.0)) < 0.0 {
            return false;
        }
    }
    true
}
