//! Thin shader-program wrapper with cached uniform locations.

use std::borrow::Cow;
use std::collections::HashMap;

use anyhow::{Result, anyhow, bail};
use glam::{Mat4, Vec2, Vec4};
use glow::HasContext;

pub struct ShaderProgram {
    program: glow::Program,
    uniforms: HashMap<String, glow::UniformLocation>,
}

impl ShaderProgram {
    pub fn new(
        gl: &glow::Context,
        label: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<Self> {
        unsafe {
            let vertex = compile(gl, glow::VERTEX_SHADER, vertex_source, label)?;
            let fragment = match compile(gl, glow::FRAGMENT_SHADER, fragment_source, label) {
                Ok(shader) => shader,
                Err(error) => {
                    gl.delete_shader(vertex);
                    return Err(error);
                }
            };
            let program = gl
                .create_program()
                .map_err(|error| anyhow!("{label}: failed to allocate a program: {error}"))?;
            gl.attach_shader(program, vertex);
            gl.attach_shader(program, fragment);
            gl.link_program(program);
            gl.detach_shader(program, vertex);
            gl.detach_shader(program, fragment);
            gl.delete_shader(vertex);
            gl.delete_shader(fragment);

            if !gl.get_program_link_status(program) {
                let log = gl.get_program_info_log(program);
                gl.delete_program(program);
                bail!("{label}: program link failed:\n{log}");
            }

            Ok(Self {
                program,
                uniforms: HashMap::new(),
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe { gl.use_program(Some(self.program)) };
    }

    pub fn raw(&self) -> glow::Program {
        self.program
    }

    pub fn uniform_location(
        &mut self,
        gl: &glow::Context,
        name: &str,
    ) -> Option<glow::UniformLocation> {
        if let Some(location) = self.uniforms.get(name) {
            return Some(*location);
        }
        let location = unsafe { gl.get_uniform_location(self.program, name) }?;
        self.uniforms.insert(name.to_owned(), location);
        Some(location)
    }

    pub fn set_mat4(&mut self, gl: &glow::Context, name: &str, value: &Mat4) {
        if let Some(location) = self.uniform_location(gl, name) {
            unsafe {
                gl.uniform_matrix_4_f32_slice(Some(&location), false, &value.to_cols_array());
            }
        }
    }

    pub fn set_vec4(&mut self, gl: &glow::Context, name: &str, value: Vec4) {
        if let Some(location) = self.uniform_location(gl, name) {
            unsafe { gl.uniform_4_f32_slice(Some(&location), &value.to_array()) };
        }
    }

    pub fn set_vec2(&mut self, gl: &glow::Context, name: &str, value: Vec2) {
        if let Some(location) = self.uniform_location(gl, name) {
            unsafe { gl.uniform_2_f32_slice(Some(&location), &value.to_array()) };
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.program) };
    }
}

unsafe fn compile(
    gl: &glow::Context,
    shader_type: u32,
    source: &str,
    label: &str,
) -> Result<glow::Shader> {
    let shader = unsafe {
        gl.create_shader(shader_type)
            .map_err(|error| anyhow!("{label}: failed to allocate a shader: {error}"))?
    };
    let source = stage_source(shader_type, source);
    unsafe {
        gl.shader_source(shader, &source);
        gl.compile_shader(shader);
    }
    if unsafe { gl.get_shader_compile_status(shader) } {
        Ok(shader)
    } else {
        let log = unsafe { gl.get_shader_info_log(shader) };
        unsafe { gl.delete_shader(shader) };
        let kind = if shader_type == glow::VERTEX_SHADER {
            "vertex"
        } else {
            "fragment"
        };
        bail!("{label}: {kind} shader compilation failed:\n{log}");
    }
}

/// Returns the source handed to the driver for the current target.
///
/// Desktop builds use the shaders verbatim (`#version 330 core`). Android
/// drivers only accept GLSL ES, so the version directive is rewritten and the
/// default precision qualifiers required by GLSL ES are added. The shader body
/// is otherwise shared between desktop OpenGL and OpenGL ES.
fn stage_source(shader_type: u32, source: &str) -> Cow<'_, str> {
    #[cfg(target_os = "android")]
    {
        Cow::Owned(gles_source(source, shader_type == glow::FRAGMENT_SHADER))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = shader_type;
        Cow::Borrowed(source)
    }
}

#[cfg(target_os = "android")]
fn gles_source(source: &str, fragment: bool) -> String {
    // A `#version` directive must be the first token of a shader, so the
    // desktop version line is replaced rather than prepended to.
    let has_version = source
        .lines()
        .next()
        .is_some_and(|line| line.trim_start().starts_with("#version"));
    let body = if has_version {
        source.split_once('\n').map_or("", |(_, rest)| rest)
    } else {
        source
    };

    let mut translated = String::with_capacity(source.len() + 128);
    translated.push_str("#version 300 es\n");
    translated.push_str("precision highp float;\n");
    translated.push_str("precision highp int;\n");
    if fragment {
        translated.push_str("precision highp sampler2D;\n");
        translated.push_str("precision highp samplerCube;\n");
    }
    translated.push_str(body);
    if !translated.ends_with('\n') {
        translated.push('\n');
    }
    translated
}
