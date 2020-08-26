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
    _image: DeviceImage,
    image_view: ImageView,
    _format: util::Format,
}

impl DepthBuffer {
    pub fn new(
        device: &Device,
        extents: &util::Extent2D,
        msaa_sample_count: vk::SampleCountFlags,
    ) -> Result<Self, DepthBufferError> {
        let format = device.depth_buffer_format().into();
        let usage = vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
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
        .map_err(DepthBufferError::Memory)?;
        let image_view = ImageView::new(
            device,
            _image.vk_image(),
            format,
            vk::ImageAspectFlags::DEPTH,
            mip_levels,
        )
        .map_err(DepthBufferError::ImageView)?;
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
