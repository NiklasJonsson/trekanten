use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::VkDevice;

pub struct Image {
    vk_image: vk::Image,
}

impl Image {
    pub fn new(vk_image: vk::Image) -> Self {
        Self { vk_image }
    }
}

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
        image: &Image,
        format: vk::Format,
        component_mapping: vk::ComponentMapping,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Result<Self, ImageViewError> {
        let info = vk::ImageViewCreateInfo::builder()
            .image(image.vk_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(component_mapping)
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

    pub fn inner_vk_image_view(&self) -> &vk::ImageView {
        &self.vk_image_view
    }
}
