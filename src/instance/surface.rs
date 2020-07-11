use ash::extensions::khr::Surface as KHRSurface;
use ash::vk;
use ash::vk::SurfaceKHR as SurfaceHandle;

use crate::instance::InitError;
use crate::instance::Instance;
use crate::util::LifetimeToken;

pub struct Surface {
    handle: SurfaceHandle,
    surface: KHRSurface,
    _parent_lifetime_token: LifetimeToken<Instance>,
}

impl Surface {
    pub fn new(
        entry: &ash::Entry,
        instance: &ash::Instance,
        handle: SurfaceHandle,
        parent_token: LifetimeToken<Instance>,
    ) -> Self {
        Self {
            handle,
            surface: KHRSurface::new(entry, instance),
            _parent_lifetime_token: parent_token,
        }
    }

    pub fn is_supported_by(
        &self,
        phys_device: &vk::PhysicalDevice,
        queue_index: u32,
    ) -> Result<bool, InitError> {
        let ret = unsafe {
            self.surface
                .get_physical_device_surface_support(*phys_device, queue_index, self.handle)
        }?;

        Ok(ret)
    }
}

impl std::ops::Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface.destroy_surface(self.handle, None);
        }
    }
}
