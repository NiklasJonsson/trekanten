use std::path::Path;
use std::path::PathBuf;

use ash::vk;

use crate::device::Device;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::PipelineError;
use crate::render_pass::RenderPass;
use crate::resource::{Handle, Storage};
use crate::util;
use crate::vertex::VertexDefinition;

#[derive(Debug)]
pub enum MaterialError {
    Builder(BuilderError),
    Pipeline(PipelineError),
}

impl std::error::Error for MaterialError {}
impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<BuilderError> for MaterialError {
    fn from(e: BuilderError) -> Self {
        Self::Builder(e)
    }
}

impl From<PipelineError> for MaterialError {
    fn from(e: PipelineError) -> Self {
        Self::Pipeline(e)
    }
}

#[derive(Default)]
pub struct Materials {
    desc_storage: Storage<MaterialDescriptor>,
    mat_storage: Storage<Material>,
}

impl Materials {
    pub fn new() -> Self {
        Self {
            desc_storage: Storage::<MaterialDescriptor>::new(),
            mat_storage: Storage::<Material>::new(),
        }
    }

    pub fn recreate_all(
        &mut self,
        device: &Device,
        viewport_extent: util::Extent2D,
        render_pass: &RenderPass,
    ) -> Result<(), MaterialError> {
        for (mut mat, desc) in self.mat_storage.iter_mut().zip(self.desc_storage.iter()) {
            let mut new_material = Material::create(device, desc, viewport_extent, render_pass)?;
            std::mem::replace(&mut mat, &mut new_material);
        }

        Ok(())
    }

    pub fn create(
        &mut self,
        device: &Device,
        descriptor: MaterialDescriptor,
        viewport_extent: util::Extent2D,
        render_pass: &RenderPass,
    ) -> Result<Handle<Material>, MaterialError> {
        let material = Material::create(&device, &descriptor, viewport_extent, render_pass)?;
        self.desc_storage.add(descriptor);
        Ok(self.mat_storage.add(material))
    }

    pub fn get(&self, h: &Handle<Material>) -> Option<&Material> {
        self.mat_storage.get(h)
    }
}

pub struct Material {
    pipeline: GraphicsPipeline,
}

impl Material {
    pub fn pipeline(&self) -> &GraphicsPipeline {
        &self.pipeline
    }

    fn create(
        device: &Device,
        descriptor: &MaterialDescriptor,
        viewport_extent: util::Extent2D,
        render_pass: &RenderPass,
    ) -> Result<Self, MaterialError> {
        let pipeline = GraphicsPipeline::builder(device)
            .vertex_shader(&descriptor.vert)?
            .fragment_shader(&descriptor.frag)?
            .vertex_input(
                &descriptor.vert_attribute_description,
                &descriptor.vert_binding_description,
            )
            .viewport_extent(viewport_extent)
            .render_pass(render_pass)
            .build()?;

        Ok(Material { pipeline })
    }
}

#[derive(Clone, Debug)]
pub struct MaterialDescriptor {
    vert: PathBuf,
    frag: PathBuf,
    vert_binding_description: Vec<vk::VertexInputBindingDescription>,
    vert_attribute_description: Vec<vk::VertexInputAttributeDescription>,
}

impl MaterialDescriptor {
    pub fn builder() -> MaterialDescriptorBuilder {
        MaterialDescriptorBuilder {
            vert: None,
            frag: None,
            vert_attribute_description: Vec::new(),
            vert_binding_description: Vec::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BuilderError {
    MissingVertexShader,
    MissingFragmentShader,
    MissingVertexDescription,
}

impl std::error::Error for BuilderError {}
impl std::fmt::Display for BuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct MaterialDescriptorBuilder {
    vert: Option<PathBuf>,
    frag: Option<PathBuf>,
    vert_binding_description: Vec<vk::VertexInputBindingDescription>,
    vert_attribute_description: Vec<vk::VertexInputAttributeDescription>,
}

impl MaterialDescriptorBuilder {
    pub fn vertex_shader<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.vert = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn fragment_shader<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.frag = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn vertex_type<V>(mut self) -> Self
    where
        V: VertexDefinition,
    {
        self.vert_binding_description = V::binding_description();
        self.vert_attribute_description = V::attribute_description();
        self
    }

    pub fn build(self) -> Result<MaterialDescriptor, MaterialError> {
        let vert = self.vert.ok_or(BuilderError::MissingVertexShader)?;
        let frag = self.frag.ok_or(BuilderError::MissingFragmentShader)?;
        if self.vert_binding_description.is_empty() || self.vert_attribute_description.is_empty() {
            return Err(BuilderError::MissingVertexDescription.into());
        }

        let vert_binding_description = self.vert_binding_description;
        let vert_attribute_description = self.vert_attribute_description;

        Ok(MaterialDescriptor {
            vert,
            frag,
            vert_binding_description,
            vert_attribute_description,
        })
    }
}
