use ash::extensions::khr::Surface as KHRSurface;
use ash::vk;
use ash::vk::SurfaceKHR as SurfaceHandle;

use crate::instance::InitError;
use crate::instance::Instance;
use crate::util::lifetime::LifetimeToken;

// TODO: Figure out better type naming:
// KHR/Handle is confusing
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

    fn get_capabilities_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, InitError> {
        let ret = unsafe {
            self.surface
                .get_physical_device_surface_capabilities(*phys_device, self.handle)
        }?;

        Ok(ret)
    }

    fn get_formats_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, InitError> {
        let ret = unsafe {
            self.surface
                .get_physical_device_surface_formats(*phys_device, self.handle)
        }?;

        Ok(ret)
    }

    fn get_present_modes_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, InitError> {
        let ret = unsafe {
            self.surface
                .get_physical_device_surface_present_modes(*phys_device, self.handle)
        }?;

        Ok(ret)
    }

    pub fn query_swapchain_support(
        &self,
        device: &vk::PhysicalDevice,
    ) -> Result<SwapchainSupportDetails, InitError> {
        let capabilites = self.get_capabilities_for(device)?;
        let formats = self.get_formats_for(device)?;
        let present_modes = self.get_present_modes_for(device)?;

        Ok(SwapchainSupportDetails {
            capabilites,
            formats,
            present_modes,
        })
    }

    pub fn vk_handle(&self) -> &SurfaceHandle {
        &self.handle
    }
}

#[derive(Clone, Debug)]
pub struct SwapchainSupportDetails {
    pub capabilites: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl std::ops::Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface.destroy_surface(self.handle, None);
        }
    }
}
