use ash::version::DeviceV1_0;
use ash::vk;

use crate::device::VkDeviceHandle;

use crate::device::HasVkDevice;
use crate::image::ImageView;
use crate::render_pass::RenderPass;
use crate::util;

#[derive(Debug, Clone)]
pub enum FramebufferError {
    Creation(vk::Result),
}

impl std::error::Error for FramebufferError {}
impl std::fmt::Display for FramebufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Framebuffer {
    vk_device: VkDeviceHandle,
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
    pub fn new<D: HasVkDevice>(
        device: &D,
        attachments: &[&ImageView],
        render_pass: &RenderPass,
        extent: &util::Extent2D,
    ) -> Result<Self, FramebufferError> {
        let vk_device = device.vk_device();

        let vk_attachments = attachments
            .iter()
            .map(|iv| *iv.vk_image_view())
            .collect::<Vec<_>>();

        let info = vk::FramebufferCreateInfo::builder()
            .render_pass(*render_pass.vk_render_pass())
            .attachments(&vk_attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        let vk_framebuffer = unsafe {
            vk_device
                .create_framebuffer(&info, None)
                .map_err(FramebufferError::Creation)?
        };

        Ok(Self {
            vk_device,
            vk_framebuffer,
        })
    }

    pub fn vk_framebuffer(&self) -> &vk::Framebuffer {
        &self.vk_framebuffer
    }
}
