use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::instance::Instance;
use crate::queue::Queue;
use crate::queue::QueueFamilies;
use crate::queue::QueueFamily;
use crate::surface::Surface;
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

    pub fn graphics_queue_family(&self) -> &QueueFamily {
        &self.queue_families.graphics
    }

    pub fn present_queue_family(&self) -> &QueueFamily {
        &self.queue_families.present
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

    pub fn vk_phys_device(&self) -> &vk::PhysicalDevice {
        &self.vk_phys_device
    }
}
