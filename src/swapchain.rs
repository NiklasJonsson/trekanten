use ash::extensions::khr::Swapchain as SwapchainLoader;
use ash::vk;
use ash::vk::SwapchainKHR;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::VkDevice;
use crate::framebuffer::{Framebuffer, FramebufferError};
use crate::image::{Image, ImageView, ImageViewError};
use crate::instance::Instance;
use crate::queue::Queue;
use crate::render_pass::RenderPass;
use crate::sync::Semaphore;
use crate::util;

#[derive(Clone, Debug)]
pub enum SwapchainError {
    Creation(vk::Result),
    ImageCreation(vk::Result),
    ImageViewCreation(ImageViewError),
    FramebufferCreation(FramebufferError),
    AcquireNextImage(vk::Result),
    EnqueuePresent(vk::Result),
}

impl std::error::Error for SwapchainError {}
impl std::fmt::Display for SwapchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ImageViewError> for SwapchainError {
    fn from(e: ImageViewError) -> Self {
        Self::ImageViewCreation(e)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SwapchainStatus {
    Optimal,
    SubOptimal,
}

#[derive(Debug, Clone, Copy)]
pub struct SwapchainInfo {
    pub format: vk::Format,
    pub extent: util::Extent2D,
}

// TODO: Cleanup Type names
pub struct Swapchain {
    loader: SwapchainLoader,
    handle: vk::SwapchainKHR,
    images: Vec<Image>,
    image_views: Vec<ImageView>,
    info: SwapchainInfo,
    vk_device: Rc<VkDevice>,
}

impl std::ops::Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_swapchain(self.handle, None) };
    }
}

impl Swapchain {
    pub fn new<D: AsVkDevice>(
        instance: &Instance,
        device: &D,
        info: vk::SwapchainCreateInfoKHR,
    ) -> Result<Self, SwapchainError> {
        log::trace!("Creating swapchain: {:#?}", info);
        let vk_device = device.vk_device();
        let loader =
            ash::extensions::khr::Swapchain::new(instance.inner_vk_instance(), &*vk_device);

        let handle = unsafe {
            loader
                .create_swapchain(&info, None)
                .map_err(SwapchainError::Creation)?
        };

        let images = unsafe {
            loader
                .get_swapchain_images(handle)
                .map_err(SwapchainError::ImageCreation)?
        }
        .into_iter()
        .map(Image::new)
        .collect::<Vec<_>>();

        // Store a lightweight representation of the info
        // TODO: Store full?
        let vk::SwapchainCreateInfoKHR {
            image_format,
            image_extent,
            ..
        } = info;

        let light_info = SwapchainInfo {
            format: image_format,
            extent: image_extent.into(),
        };

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

        let image_views = images
            .iter()
            .map(|img| ImageView::new(device, img, image_format, comp_mapping, subresource_range))
            .collect::<Result<Vec<_>, ImageViewError>>()
            .map_err(SwapchainError::ImageViewCreation)?;

        Ok(Self {
            loader,
            handle,
            images,
            image_views,
            info: light_info,
            vk_device: device.vk_device(),
        })
    }

    pub fn info(&self) -> &SwapchainInfo {
        &self.info
    }

    pub fn create_framebuffers_for(
        &self,
        render_pass: &RenderPass,
    ) -> Result<Vec<Framebuffer>, FramebufferError> {
        self.image_views
            .iter()
            .map(|iv| {
                let views = [iv];
                Framebuffer::new(&self.vk_device, &views, render_pass, &self.info.extent)
            })
            .collect::<Result<Vec<_>, FramebufferError>>()
    }

    pub fn acquire_next_image(&self, sem: Option<&Semaphore>) -> Result<u32, SwapchainError> {
        let s = sem
            .map(|x| *x.vk_semaphore())
            .unwrap_or(vk::Semaphore::null());
        let f = vk::Fence::null();
        let result = unsafe {
            self.loader
                .acquire_next_image(self.handle, u64::MAX, s, f)
                .map_err(SwapchainError::AcquireNextImage)?
        };

        let (idx, optimal) = result;

        if !optimal {
            log::error!("Non-optimal swapchain!");
        }

        Ok(idx)
    }

    pub fn vk_swapchain(&self) -> &vk::SwapchainKHR {
        &self.handle
    }

    pub fn enqueue_present(
        &self,
        queue: &Queue,
        info: vk::PresentInfoKHR,
    ) -> Result<SwapchainStatus, SwapchainError> {
        let suboptimal = unsafe {
            self.loader
                .queue_present(*queue.vk_queue(), &info)
                .map_err(SwapchainError::EnqueuePresent)?
        };

        if suboptimal {
            Ok(SwapchainStatus::SubOptimal)
        } else {
            Ok(SwapchainStatus::Optimal)
        }
    }
}
