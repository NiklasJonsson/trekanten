use ash::extensions::khr::Swapchain as KHRSwapchain;
use ash::vk;
use ash::vk::SwapchainKHR as SwapchainHandle;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::VkDevice;
use crate::framebuffer::Framebuffer;
use crate::image::{Image, ImageView};
use crate::instance::InitError;
use crate::instance::Instance;
use crate::render_pass::RenderPass;
use crate::util;

pub struct SwapchainInfo {
    pub format: vk::Format,
    pub extent: util::Extent2D,
}

// TODO: Cleanup Type names
pub struct Swapchain {
    swapchain: KHRSwapchain,
    handle: SwapchainHandle,
    images: Vec<Image>,
    image_views: Vec<ImageView>,
    info: SwapchainInfo,
    vk_device: Rc<VkDevice>,
}

impl std::ops::Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.swapchain.destroy_swapchain(self.handle, None) };
    }
}

impl Swapchain {
    pub fn new<D: AsVkDevice>(
        instance: &Instance,
        device: &D,
        info: vk::SwapchainCreateInfoKHR,
    ) -> Result<Self, InitError> {
        log::trace!("Creating swapchain: {:#?}", info);
        let vk_device = device.vk_device();
        // TODO: Can we handle this without have to expose this from the instance? Should the
        // function exist on the instance?
        let swapchain =
            ash::extensions::khr::Swapchain::new(instance.inner_vk_instance(), &*vk_device);

        let handle = unsafe { swapchain.create_swapchain(&info, None) }?;

        let images = unsafe { swapchain.get_swapchain_images(handle) }?
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
            .collect::<Result<Vec<_>, InitError>>()?;

        Ok(Self {
            swapchain,
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
    ) -> Result<Vec<Framebuffer>, InitError> {
        self.image_views
            .iter()
            .map(|iv| {
                let views = [iv];
                Framebuffer::new(&self.vk_device, &views, render_pass, &self.info.extent)
            })
            .collect::<Result<Vec<_>, InitError>>()
    }
}
