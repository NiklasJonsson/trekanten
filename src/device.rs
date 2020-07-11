use ash::version::DeviceV1_0;

use crate::instance::Instance;
use crate::util::lifetime::LifetimeToken;

pub struct Device {
    vk_device: ash::Device,
    _parent_lifetime_token: LifetimeToken<Instance>,
}
impl std::ops::Drop for Device {
    fn drop(&mut self) {
        unsafe { self.vk_device.destroy_device(None) };
    }
}

impl Device {
    pub fn new(vk_device: ash::Device, parent_lifetime_token: LifetimeToken<Instance>) -> Self {
        Self {
            vk_device,
            _parent_lifetime_token: parent_lifetime_token,
        }
    }
}
