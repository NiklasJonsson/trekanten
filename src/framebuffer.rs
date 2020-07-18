use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::VkDevice;
use crate::image::ImageView;
use crate::instance::InitError;
use crate::render_pass::RenderPass;
use crate::util;

pub struct Framebuffer {
    vk_device: Rc<VkDevice>,
    vk_framebuffer: vk::Framebuffer,
}

impl std::ops::Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_framebuffer(self.vk_framebuffer, None);
        }
    }
}

impl Framebuffer {
    pub fn new<D: AsVkDevice>(
        device: &D,
        attachments: &[&ImageView],
        render_pass: &RenderPass,
        extent: &util::Extent2D,
    ) -> Result<Self, InitError> {
        let vk_device = device.vk_device();

        let vk_attachments = attachments
            .iter()
            .map(|iv| *iv.inner_vk_image_view())
            .collect::<Vec<_>>();

        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(*render_pass.inner_vk_render_pass())
            .attachments(&vk_attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let vk_framebuffer = unsafe { vk_device.create_framebuffer(&info, None)? };

        Ok(Self {
            vk_device,
            vk_framebuffer,
        })
    }

    pub fn inner_vk_framebuffer(&self) -> &vk::Framebuffer {
        &self.vk_framebuffer
    }
}
