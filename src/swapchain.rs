use ash::extensions::khr::Swapchain as KHRSwapchain;
use ash::vk;
use ash::vk::SwapchainKHR as SwapchainHandle;

use crate::device::Device;
use crate::instance::InitError;
use crate::instance::Instance;
use crate::util::lifetime::LifetimeToken;

struct Image {
    vk_image: vk::Image,
}

struct SwapchainInfo {
    format: vk::Format,
    extent: vk::Extent2D,
}

// TODO: Lifetime token from device
pub struct Swapchain {
    swapchain: KHRSwapchain,
    handle: SwapchainHandle,
    images: Vec<Image>,
    info: SwapchainInfo,
    _parent_lifetime_token: LifetimeToken<Device>,
}

impl std::ops::Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.swapchain.destroy_swapchain(self.handle, None) };
    }
}

impl Swapchain {
    pub fn new(
        instance: &Instance,
        vk_device: &ash::Device,
        info: vk::SwapchainCreateInfoKHR,
        device_lifetime_token: LifetimeToken<Device>,
    ) -> Result<Self, InitError> {
        log::trace!("Creating swapchain: {:#?}", info);
        // TODO: Can we handle this without have to expose this from the instance? Should the
        // function exist on the instance?
        let swapchain =
            ash::extensions::khr::Swapchain::new(instance.inner_vk_instance(), vk_device);

        let handle = unsafe { swapchain.create_swapchain(&info, None) }?;

        let images = unsafe { swapchain.get_swapchain_images(handle) }?
            .into_iter()
            .map(|vk_image| Image { vk_image })
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
            extent: image_extent,
        };

        Ok(Self {
            swapchain,
            handle,
            images,
            info: light_info,
            _parent_lifetime_token: device_lifetime_token,
        })
    }
}
