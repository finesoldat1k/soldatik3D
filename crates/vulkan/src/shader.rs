use anyhow::Result;
use ash::vk;
use shaderc::{Compiler, ShaderKind};

pub fn compile(source: &str, kind: ShaderKind, name: &str) -> Result<Vec<u32>> {
    let compiler =
        Compiler::new().ok_or_else(|| anyhow::anyhow!("failed to create shaderc compiler"))?;
    let artifact = compiler.compile_into_spirv(source, kind, name, "main", None)?;
    Ok(artifact.as_binary().to_vec())
}

pub fn module(device: &ash::Device, code: &[u32]) -> Result<vk::ShaderModule> {
    let info = vk::ShaderModuleCreateInfo::default().code(code);
    Ok(unsafe { device.create_shader_module(&info, None)? })
}
