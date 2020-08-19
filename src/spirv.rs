use ash::vk;

use spirv_reflect::types::descriptor::ReflectDescriptorType;
use spirv_reflect::types::variable::ReflectShaderStageFlags;
use spirv_reflect::ShaderModule;

#[derive(Debug)]
pub enum SpirvError {
    Loading(&'static str),
    Parsing(&'static str),
}

impl std::error::Error for SpirvError {}
impl std::fmt::Display for SpirvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<&'static str> for SpirvError {
    fn from(s: &'static str) -> Self {
        Self::Parsing(s)
    }
}

pub struct DescriptorSetLayoutData {
    pub set_idx: usize,
    pub bindings: Vec<vk::DescriptorSetLayoutBinding>,
}

fn map_shader_stage_flags(refl_stage: &ReflectShaderStageFlags) -> vk::ShaderStageFlags {
    match *refl_stage {
        ReflectShaderStageFlags::VERTEX => vk::ShaderStageFlags::VERTEX,
        ReflectShaderStageFlags::FRAGMENT => vk::ShaderStageFlags::FRAGMENT,
        _ => unimplemented!("Unsupported shader stage!"),
    }
}

fn map_descriptor_type(refl_desc_ty: &ReflectDescriptorType) -> vk::DescriptorType {
    match *refl_desc_ty {
        ReflectDescriptorType::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
        _ => unimplemented!("Unsupported descriptor type!"),
    }
}

pub fn parse_descriptor_sets(spv_data: &[u32]) -> Result<Vec<DescriptorSetLayoutData>, SpirvError> {
    let module = ShaderModule::load_u32_data(spv_data).map_err(SpirvError::Loading)?;
    let desc_sets = module.enumerate_descriptor_sets(None)?;
    let shader_stage = map_shader_stage_flags(&module.get_shader_stage());
    let mut ret = Vec::with_capacity(desc_sets.len());
    for refl_desc_set in desc_sets.iter() {
        let set_idx = refl_desc_set.set;
        let bindings: Vec<vk::DescriptorSetLayoutBinding> = refl_desc_set
            .bindings
            .iter()
            .map(|refl_binding| vk::DescriptorSetLayoutBinding {
                binding: refl_binding.binding,
                descriptor_type: map_descriptor_type(&refl_binding.descriptor_type),
                descriptor_count: 1,
                stage_flags: shader_stage,
                ..Default::default()
            })
            .collect();

        ret.push(DescriptorSetLayoutData {
            set_idx: set_idx as usize,
            bindings,
        })
    }

    Ok(ret)
}

#[cfg(test)]
mod tests {
    static UBO_SPV: &[u32] = inline_spirv::inline_spirv!(
        r"
        #version 450 core
        layout(binding = 0) uniform UniformBufferObject {
            mat4 model;
            mat4 view;
            mat4 proj;
        } ubo;

        void main() {}
    ",
        vert
    );

    use super::*;

    #[test]
    fn parse_descriptor_set_layout() {
        let res = parse_descriptor_sets(UBO_SPV).expect("Failed to parse!");
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].bindings.len(), 1);
        assert_eq!(res[0].set_idx, 0);

        let binding: vk::DescriptorSetLayoutBinding = res[0].bindings[0];

        assert_eq!(binding.descriptor_type, vk::DescriptorType::UNIFORM_BUFFER);
        assert_eq!(binding.binding, 0);
        assert_eq!(binding.descriptor_count, 1);
        assert_eq!(binding.stage_flags, vk::ShaderStageFlags::VERTEX);
    }
}
