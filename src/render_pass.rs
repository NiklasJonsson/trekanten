use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDevice;
use crate::instance::InitError;

pub struct RenderPass {
    vk_device: Rc<VkDevice>,
    vk_render_pass_handle: vk::RenderPass,
}

impl std::ops::Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_render_pass(self.vk_render_pass_handle, None);
        }
    }
}

impl RenderPass {
    pub fn new(device: &Device, format: vk::Format) -> Result<Self, InitError> {
        let color_attach_format = vk::AttachmentDescription::builder()
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

        let color_attach_refs = &[color_attach_ref];

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attach_refs);

        let attachments = [*color_attach_format];
        let subpasses = [*subpass];
        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&subpasses);

        let vk_device = device.vk_device();

        let vk_render_pass_handle =
            unsafe { vk_device.create_render_pass(&render_pass_info, None)? };

        Ok(Self {
            vk_device,
            vk_render_pass_handle,
        })
    }

    pub fn inner_vk_render_pass(&self) -> &vk::RenderPass {
        &self.vk_render_pass_handle
    }
}
