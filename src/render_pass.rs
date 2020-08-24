use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDevice;

#[derive(Clone, Debug)]
pub enum RenderPassError {
    Creation(vk::Result),
}

impl std::error::Error for RenderPassError {}
impl std::fmt::Display for RenderPassError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct RenderPass {
    vk_device: Rc<VkDevice>,
    vk_render_pass: vk::RenderPass,
    vk_clear_values: [vk::ClearValue; 2],
}

impl std::ops::Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_render_pass(self.vk_render_pass, None);
        }
    }
}

impl RenderPass {
    pub fn new(device: &Device, format: vk::Format) -> Result<Self, RenderPassError> {
        let color_attach = vk::AttachmentDescription::builder()
            .format(format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attach_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let depth_attach = vk::AttachmentDescription::builder()
            .format(device.depth_buffer_format())
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::DONT_CARE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

        let depth_attach_ref = vk::AttachmentReference {
            attachment: 1,
            layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        };

        let color_attach_refs = &[color_attach_ref];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attach_refs)
            .depth_stencil_attachment(&depth_attach_ref);

        let attachments = [*color_attach, *depth_attach];
        let subpasses = [*subpass];

        let subpass_dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let dependencies = [subpass_dependency.build()];

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        let vk_device = device.vk_device();

        let vk_render_pass = unsafe {
            vk_device
                .create_render_pass(&render_pass_info, None)
                .map_err(RenderPassError::Creation)?
        };

        let vk_clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        Ok(Self {
            vk_device,
            vk_render_pass,
            vk_clear_values,
        })
    }

    pub fn vk_clear_values(&self) -> &[vk::ClearValue] {
        &self.vk_clear_values
    }

    pub fn vk_render_pass(&self) -> &vk::RenderPass {
        &self.vk_render_pass
    }
}
