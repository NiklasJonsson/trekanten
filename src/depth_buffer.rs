use ash::vk;

use crate::device::Device;
use crate::image::{ImageView, ImageViewError};
use crate::mem::{DeviceImage, MemoryError};
use crate::util;

#[derive(Debug)]
pub enum DepthBufferError {
    Memory(MemoryError),
    ImageView(ImageViewError),
}

impl std::error::Error for DepthBufferError {}
impl std::fmt::Display for DepthBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct DepthBuffer {
    image: DeviceImage,
    image_view: ImageView,
    _format: util::Format,
}

impl DepthBuffer {
    pub fn new(device: &Device, extents: &util::Extent2D) -> Result<Self, DepthBufferError> {
        let format = device.depth_buffer_format().into();
        let usage = vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        let props = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        let mip_levels = 1; // No mip maps
        let image = DeviceImage::empty_2d(device, *extents, format, usage, props, mip_levels)
            .map_err(DepthBufferError::Memory)?;
        let image_view = ImageView::new(
            device,
            image.vk_image(),
            format,
            vk::ImageAspectFlags::DEPTH,
            mip_levels,
        )
        .map_err(DepthBufferError::ImageView)?;
        Ok(Self {
            image,
            image_view,
            _format: format,
        })
    }

    pub fn vk_image(&self) -> &vk::Image {
        &self.image.vk_image()
    }

    pub fn image_view(&self) -> &ImageView {
        &self.image_view
    }
}
