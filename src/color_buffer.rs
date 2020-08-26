use ash::vk;

use crate::device::Device;
use crate::image::{ImageView, ImageViewError};
use crate::mem::{DeviceImage, MemoryError};
use crate::util;

#[derive(Debug)]
pub enum ColorBufferError {
    Memory(MemoryError),
    ImageView(ImageViewError),
}

impl std::error::Error for ColorBufferError {}
impl std::fmt::Display for ColorBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct ColorBuffer {
    _image: DeviceImage,
    image_view: ImageView,
    _format: util::Format,
}

impl ColorBuffer {
    pub fn new(
        device: &Device,
        format: util::Format,
        extents: &util::Extent2D,
        msaa_sample_count: vk::SampleCountFlags,
    ) -> Result<Self, ColorBufferError> {
        let usage =
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT;
        let props = vk_mem::MemoryUsage::GpuOnly;
        let mip_levels = 1; // No mip maps
        let _image = DeviceImage::empty_2d(
            device,
            *extents,
            format,
            usage,
            props,
            mip_levels,
            msaa_sample_count,
        )
        .map_err(ColorBufferError::Memory)?;
        let image_view = ImageView::new(
            device,
            _image.vk_image(),
            format,
            vk::ImageAspectFlags::COLOR,
            mip_levels,
        )
        .map_err(ColorBufferError::ImageView)?;
        Ok(Self {
            _image,
            image_view,
            _format: format,
        })
    }

    pub fn image_view(&self) -> &ImageView {
        &self.image_view
    }
}
