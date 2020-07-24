use ash::vk;

use super::device_selection::DeviceSuitability;
use crate::surface::SurfaceError;
use crate::swapchain::SwapchainError;

#[derive(Debug, Clone)]
pub enum DeviceCreationError {
    Creation(vk::Result),
    UnsuitableDevice(DeviceSuitability),
    MissingPhysicalDevice,
    ExtensionEnumeration(vk::Result),
    PhysicalDeviceEnumeration(vk::Result),
    Surface(SurfaceError),
}

impl std::error::Error for DeviceCreationError {}
impl std::fmt::Display for DeviceCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<SurfaceError> for DeviceCreationError {
    fn from(e: SurfaceError) -> Self {
        Self::Surface(e)
    }
}

#[derive(Debug, Clone)]
pub enum DeviceError {
    Creation(DeviceCreationError),
    Swapchain(SwapchainError),
    WaitIdle(vk::Result),
}

impl std::error::Error for DeviceError {}
impl std::fmt::Display for DeviceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<SwapchainError> for DeviceError {
    fn from(e: SwapchainError) -> Self {
        Self::Swapchain(e)
    }
}

impl From<DeviceCreationError> for DeviceError {
    fn from(e: DeviceCreationError) -> Self {
        Self::Creation(e)
    }
}
