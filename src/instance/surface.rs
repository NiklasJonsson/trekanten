use ash::extensions::khr::Surface as KHRSurface;
use ash::vk::SurfaceKHR as SurfaceHandle;

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
}

impl std::ops::Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface.destroy_surface(self.handle, None);
        }
    }
}
