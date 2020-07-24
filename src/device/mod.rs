use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::instance::Instance;
use crate::queue::Queue;
use crate::queue::QueueFamilies;
use crate::queue::QueueFamily;
use crate::surface::Surface;
use crate::swapchain::Swapchain;
use crate::util;
use crate::util::lifetime::LifetimeToken;

mod device_selection;
mod error;

use error::DeviceError;

pub type VkDevice = ash::Device;
pub type VkDeviceHandle = Rc<ash::Device>;

pub trait AsVkDevice {
    fn vk_device(&self) -> VkDeviceHandle;
}

impl AsVkDevice for VkDeviceHandle {
    fn vk_device(&self) -> VkDeviceHandle {
        Rc::clone(&self)
    }
}

pub struct Device {
    vk_device: Rc<VkDevice>,
    vk_phys_device: vk::PhysicalDevice,
    queue_families: QueueFamilies,
    graphics_queue: Queue,
    present_queue: Queue,
    _parent_lifetime_token: LifetimeToken<Instance>,
}

impl AsVkDevice for Device {
    fn vk_device(&self) -> VkDeviceHandle {
        Rc::clone(&self.vk_device)
    }
}

impl std::ops::Drop for Device {
    fn drop(&mut self) {
        if !Rc::strong_count(&self.vk_device) == 1 {
            log::error!(
                "References to inner vk device still existing but Device is being destroyed!"
            );
        }

        unsafe { self.vk_device.destroy_device(None) };
    }
}

fn choose_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    for f in formats.iter() {
        if f.format == vk::Format::B8G8R8A8_SRGB
            && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        {
            return *f;
        }
    }

    formats[0]
}

fn choose_swapchain_surface_present_mode(pmodes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    for pm in pmodes.iter() {
        if *pm == vk::PresentModeKHR::MAILBOX {
            return *pm;
        }
    }

    // Always available according to spec
    vk::PresentModeKHR::FIFO
}

fn choose_swapchain_extent(capabilites: &vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
    if capabilites.current_extent.width != u32::MAX {
        capabilites.current_extent
    } else {
        vk::Extent2D {
            width: util::clamp(
                super::WINDOW_WIDTH,
                capabilites.min_image_extent.width,
                capabilites.max_image_extent.width,
            ),
            height: util::clamp(
                super::WINDOW_HEIGHT,
                capabilites.min_image_extent.height,
                capabilites.max_image_extent.height,
            ),
        }
    }
}

impl Device {
    pub fn new(instance: &Instance, surface: &Surface) -> Result<Self, DeviceError> {
        let (vk_device, vk_phys_device, queue_families) =
            device_selection::device_selection(instance, surface)?;

        let (gfx, present) = unsafe {
            (
                vk_device.get_device_queue(queue_families.graphics.index, 0),
                vk_device.get_device_queue(queue_families.present.index, 0),
            )
        };

        let vk_device = Rc::new(vk_device);

        let graphics_queue = Queue::new(Rc::clone(&vk_device), gfx);
        let present_queue = Queue::new(Rc::clone(&vk_device), present);

        Ok(Self {
            vk_device,
            vk_phys_device,
            queue_families,
            graphics_queue,
            present_queue,
            _parent_lifetime_token: instance.lifetime_token(),
        })
    }

    // TODO: Move this to swapchain?
    pub fn create_swapchain(
        &self,
        instance: &Instance,
        surface: &Surface,
    ) -> Result<Swapchain, DeviceError> {
        let query = surface.query_swapchain_support(&self.vk_phys_device)?;
        log::trace!("Creating swapchain");
        log::trace!("Available: {:#?}", query);
        let format = choose_swapchain_surface_format(&query.formats);
        let present_mode = choose_swapchain_surface_present_mode(&query.present_modes);
        let extent = choose_swapchain_extent(&query.capabilites);

        let mut image_count = query.capabilites.min_image_count + 1;
        // Zero means no max
        if query.capabilites.max_image_count > 0 && image_count > query.capabilites.max_image_count
        {
            image_count = query.capabilites.max_image_count;
        }

        let mut builder = vk::SwapchainCreateInfoKHR::builder()
            .surface(*surface.vk_handle())
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        let qfams = &self.queue_families;
        let indices = [qfams.graphics.index, qfams.present.index];
        if qfams.graphics.index != qfams.present.index {
            // TODO: CONCURRENT is suboptimal but easier
            builder = builder
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&indices);
        } else {
            builder = builder
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .queue_family_indices(&[]); // optional
        }

        let info = builder
            .pre_transform(query.capabilites.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null())
            .build();

        Ok(Swapchain::new(instance, self, info)?)
    }

    pub fn graphics_queue_family(&self) -> &QueueFamily {
        &self.queue_families.graphics
    }

    pub fn graphics_queue(&self) -> &Queue {
        &self.graphics_queue
    }

    pub fn present_queue(&self) -> &Queue {
        &self.present_queue
    }

    pub fn wait_idle(&self) -> Result<(), DeviceError> {
        unsafe {
            self.vk_device
                .device_wait_idle()
                .map_err(DeviceError::WaitIdle)?;
        }

        Ok(())
    }
}
