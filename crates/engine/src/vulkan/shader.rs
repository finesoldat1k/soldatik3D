use anyhow::Result;
use ash::vk;
use shaderc::{Compiler, ShaderKind};

pub fn compile_glsl_to_spirv(source: &str, kind: ShaderKind) -> Result<Vec<u32>> {
    let compiler = Compiler::new().ok_or_else(|| anyhow::anyhow!("Failed to create shaderc compiler"))?;
    let artifact = compiler.compile_into_spirv(source, kind, "shader", "main", None)?;
    Ok(artifact.as_binary().to_vec())
}

pub fn create_shader_module(device: &ash::Device, code: &[u32]) -> Result<vk::ShaderModule> {
    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    let module = unsafe { device.create_shader_module(&create_info, None)? };
    Ok(module)
}
