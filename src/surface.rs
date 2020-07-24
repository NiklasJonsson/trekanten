use ash::extensions::khr::Surface as SurfaceLoader;
use ash::vk;

use crate::instance::Instance;
use crate::util::lifetime::LifetimeToken;

#[derive(Debug, Clone)]
pub enum SurfaceError {
    Creation(vk::Result),
    SupportQuery(vk::Result),
    CapabilitesQuery(vk::Result),
    FormatsQuery(vk::Result),
    PresentModesQuery(vk::Result),
}

impl std::error::Error for SurfaceError {}
impl std::fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Surface {
    handle: vk::SurfaceKHR,
    loader: SurfaceLoader,
    _parent_lifetime_token: LifetimeToken<Instance>,
}

impl Surface {
    pub fn new<W: raw_window_handle::HasRawWindowHandle>(
        instance: &Instance,
        w: &W,
    ) -> Result<Self, SurfaceError> {
        let handle = unsafe {
            ash_window::create_surface(instance.vk_entry(), instance.vk_instance(), w, None)
                .map_err(SurfaceError::Creation)
        }?;

        Ok(Self {
            handle,
            loader: SurfaceLoader::new(instance.vk_entry(), instance.vk_instance()),
            _parent_lifetime_token: instance.lifetime_token(),
        })
    }

    pub fn is_supported_by(
        &self,
        phys_device: &vk::PhysicalDevice,
        queue_index: u32,
    ) -> Result<bool, SurfaceError> {
        unsafe {
            self.loader
                .get_physical_device_surface_support(*phys_device, queue_index, self.handle)
                .map_err(SurfaceError::SupportQuery)
        }
    }

    fn get_capabilities_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR, SurfaceError> {
        unsafe {
            self.loader
                .get_physical_device_surface_capabilities(*phys_device, self.handle)
                .map_err(SurfaceError::CapabilitesQuery)
        }
    }

    fn get_formats_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>, SurfaceError> {
        unsafe {
            self.loader
                .get_physical_device_surface_formats(*phys_device, self.handle)
                .map_err(SurfaceError::FormatsQuery)
        }
    }

    fn get_present_modes_for(
        &self,
        phys_device: &vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>, SurfaceError> {
        unsafe {
            self.loader
                .get_physical_device_surface_present_modes(*phys_device, self.handle)
                .map_err(SurfaceError::PresentModesQuery)
        }
    }

    pub fn query_swapchain_support(
        &self,
        device: &vk::PhysicalDevice,
    ) -> Result<SwapchainSupportDetails, SurfaceError> {
        let capabilites = self.get_capabilities_for(device)?;
        let formats = self.get_formats_for(device)?;
        let present_modes = self.get_present_modes_for(device)?;

        Ok(SwapchainSupportDetails {
            capabilites,
            formats,
            present_modes,
        })
    }

    pub fn vk_handle(&self) -> &vk::SurfaceKHR {
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
            self.loader.destroy_surface(self.handle, None);
        }
    }
}
