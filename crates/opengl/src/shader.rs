//! Thin shader-program wrapper with cached uniform locations.

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
    unsafe {
        gl.shader_source(shader, source);
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
