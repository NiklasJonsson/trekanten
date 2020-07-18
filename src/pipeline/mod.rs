use ash::version::DeviceV1_0;
use ash::vk;

use std::convert::Into;
use std::ffi::CString;
use std::fs::File;
use std::io;
use std::path::Path;
use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDevice;
use crate::render_pass::RenderPass;
use crate::util;

#[derive(Debug)]
pub enum ShaderModuleError {
    Creation(ash::vk::Result),
}

impl std::error::Error for ShaderModuleError {}
impl std::fmt::Display for ShaderModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<vk::Result> for ShaderModuleError {
    fn from(r: vk::Result) -> Self {
        Self::Creation(r)
    }
}

#[derive(Debug)]
pub enum PipelineError {
    IO(std::io::Error),
    ShaderModule(ShaderModuleError),
    PipelineLayoutCreation(vk::Result),
    PipelineCreation(vk::Result),
}

impl std::error::Error for PipelineError {}
impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for PipelineError {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

impl From<ShaderModuleError> for PipelineError {
    fn from(sme: ShaderModuleError) -> Self {
        Self::ShaderModule(sme)
    }
}

struct RawShader {
    pub data: Vec<u32>,
}

fn read_shader_abs<P: AsRef<Path>>(path: P) -> io::Result<RawShader> {
    let path_ref = path.as_ref();
    log::trace!("Reading shader from {}", path_ref.display());
    let mut file = File::open(path_ref)?;
    let words = ash::util::read_spv(&mut file)?;
    Ok(RawShader { data: words })
}

fn read_shader_rel<N: AsRef<Path>>(name: N) -> io::Result<RawShader> {
    let cd = std::env::current_dir()?;
    let path = cd.join("src").join("pipeline").join("shaders").join(name);

    read_shader_abs(path)
}

fn create_shader_module(
    device: &Device,
    raw: &RawShader,
) -> Result<vk::ShaderModule, ShaderModuleError> {
    let info = vk::ShaderModuleCreateInfo::builder().code(&raw.data);

    let vk_shader_module = unsafe { device.vk_device().create_shader_module(&info, None) }?;

    Ok(vk_shader_module)
}

pub trait Pipeline {
    const BIND_POINT: vk::PipelineBindPoint;

    fn vk_pipeline(&self) -> &vk::Pipeline;
}

pub struct GraphicsPipeline {
    vk_device: Rc<VkDevice>,
    vk_pipeline_layout: vk::PipelineLayout,
    vk_pipeline: vk::Pipeline,
}

impl Pipeline for GraphicsPipeline {
    const BIND_POINT: vk::PipelineBindPoint = vk::PipelineBindPoint::GRAPHICS;

    fn vk_pipeline(&self) -> &vk::Pipeline {
        &self.vk_pipeline
    }
}

impl std::ops::Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_pipeline_layout(self.vk_pipeline_layout, None);
            self.vk_device.destroy_pipeline(self.vk_pipeline, None);
        }
    }
}

impl GraphicsPipeline {
    pub fn new<P0: AsRef<Path>, P1: AsRef<Path>>(
        device: &Device,
        viewport_extent: util::Extent2D,
        render_pass: &RenderPass,
        vert: P0,
        frag: P1,
    ) -> Result<Self, PipelineError> {
        let vert_raw = read_shader_rel(vert)?;
        let frag_raw = read_shader_rel(frag)?;

        let vert_mod = create_shader_module(device, &vert_raw)?;
        let frag_mod = create_shader_module(device, &frag_raw)?;

        let entry_name = CString::new("main").expect("CString failed!");

        let vert_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_mod)
            .name(&entry_name);

        let frag_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_mod)
            .name(&entry_name);

        let stages = [*vert_create_info, *frag_create_info];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[])
            .vertex_attribute_descriptions(&[]);

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(viewport_extent.width as f32)
            .height(viewport_extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor_extent: vk::Extent2D = viewport_extent.into();

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: scissor_extent,
        };

        let viewports = [*viewport];
        let scissors = [scissor];
        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        let raster_state_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        let msaa_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attach_info = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false);

        let attachments = [*color_blend_attach_info];
        let color_blend_state_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .attachments(&attachments);

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = unsafe {
            device
                .vk_device()
                .create_pipeline_layout(&pipeline_layout_info, None)
                .map_err(PipelineError::PipelineLayoutCreation)?
        };

        let g_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&raster_state_info)
            .multisample_state(&msaa_info)
            .color_blend_state(&color_blend_state_info)
            .layout(pipeline_layout)
            .render_pass(*render_pass.inner_vk_render_pass())
            .subpass(0);

        let create_infos = [*g_pipeline_info];

        let vk_device = device.vk_device();

        let vk_pipelines_result = unsafe {
            vk_device.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
        };
        // According to: https://renderdoc.org/vkspec_chunked/chap10.html#pipelines-multiple
        // Implementations will attempt to create as many pipelines as possible, but if any fail, we really want to exit anyway.

        let pipelines =
            vk_pipelines_result.map_err(|(_vec, e)| PipelineError::PipelineCreation(e))?;

        assert_eq!(pipelines.len(), 1, "Expected single pipeline");

        let vk_pipeline = pipelines[0];

        unsafe {
            vk_device.destroy_shader_module(vert_mod, None);
            vk_device.destroy_shader_module(frag_mod, None);
        }

        Ok(Self {
            vk_device,
            vk_pipeline,
            vk_pipeline_layout: pipeline_layout,
        })
    }
}
