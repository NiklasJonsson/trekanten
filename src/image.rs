use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::VkDevice;

#[derive(Debug, Clone)]
pub enum ImageViewError {
    Creation(vk::Result),
}

impl std::error::Error for ImageViewError {}
impl std::fmt::Display for ImageViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct ImageView {
    vk_image_view: vk::ImageView,
    vk_device: Rc<VkDevice>,
}

impl std::ops::Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_image_view(self.vk_image_view, None);
        }
    }
}

impl ImageView {
    pub fn new<D: AsVkDevice>(
        device: &D,
        vk_image: &vk::Image,
        format: vk::Format,
    ) -> Result<Self, ImageViewError> {
        let comp_mapping = vk::ComponentMapping {
            r: vk::ComponentSwizzle::R,
            g: vk::ComponentSwizzle::G,
            b: vk::ComponentSwizzle::B,
            a: vk::ComponentSwizzle::A,
        };

        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let info = vk::ImageViewCreateInfo::builder()
            .image(*vk_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(comp_mapping)
            .subresource_range(subresource_range);

        let vk_image_view = unsafe {
            device
                .vk_device()
                .create_image_view(&info, None)
                .map_err(ImageViewError::Creation)?
        };

        Ok(Self {
            vk_image_view,
            vk_device: device.vk_device(),
        })
    }

    pub fn vk_image_view(&self) -> &vk::ImageView {
        &self.vk_image_view
    }
}
