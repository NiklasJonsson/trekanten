use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::Device;
use crate::instance::InitError;

pub struct Image {
    vk_image: vk::Image,
}

impl Image {
    pub fn new(vk_image: vk::Image) -> Self {
        Self { vk_image }
    }
}

pub struct ImageView {
    vk_image_view: vk::ImageView,
    vk_device: Rc<ash::Device>,
}

impl std::ops::Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_image_view(self.vk_image_view, None);
        }
    }
}

impl ImageView {
    pub fn new(
        device: &Device,
        image: &Image,
        format: vk::Format,
        component_mapping: vk::ComponentMapping,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Result<Self, InitError> {
        let info = vk::ImageViewCreateInfo::builder()
            .image(image.vk_image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .components(component_mapping)
            .subresource_range(subresource_range);

        let vk_image_view = device.create_image_view(&info)?;

        Ok(Self {
            vk_image_view,
            vk_device: device.inner_vk_device(),
        })
    }
}
