use ash::version::DeviceV1_0;
use ash::vk;

use crate::device::VkDeviceHandle;

use crate::device::HasVkDevice;

#[derive(Debug, Copy, Clone)]
pub enum SemaphoreError {
    Creation(vk::Result),
}

impl std::error::Error for SemaphoreError {}
impl std::fmt::Display for SemaphoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Semaphore {
    vk_semaphore: vk::Semaphore,
    vk_device: VkDeviceHandle,
}

impl std::ops::Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_semaphore(self.vk_semaphore, None);
        }
    }
}

impl Semaphore {
    pub fn new<D: HasVkDevice>(device: &D) -> Result<Self, SemaphoreError> {
        let vk_device = device.vk_device();
        let info = vk::SemaphoreCreateInfo::default();

        let vk_semaphore = unsafe {
            vk_device
                .create_semaphore(&info, None)
                .map_err(SemaphoreError::Creation)?
        };

        Ok(Self {
            vk_device,
            vk_semaphore,
        })
    }

    pub fn vk_semaphore(&self) -> &vk::Semaphore {
        &self.vk_semaphore
    }
}

#[derive(Debug, Copy, Clone)]
pub enum FenceError {
    Creation(vk::Result),
    Await(vk::Result),
    Reset(vk::Result),
}

impl std::error::Error for FenceError {}
impl std::fmt::Display for FenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone)]
pub struct Fence {
    vk_fence: vk::Fence,
    vk_device: VkDeviceHandle,
}

impl std::ops::Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_fence(self.vk_fence, None);
        }
    }
}

impl Fence {
    fn new<D: HasVkDevice>(device: &D, flags: vk::FenceCreateFlags) -> Result<Self, FenceError> {
        let vk_device = device.vk_device();
        let info = vk::FenceCreateInfo {
            flags,
            ..Default::default()
        };

        let vk_fence = unsafe {
            vk_device
                .create_fence(&info, None)
                .map_err(FenceError::Creation)?
        };

        Ok(Self {
            vk_device,
            vk_fence,
        })
    }

    pub fn signaled<D: HasVkDevice>(device: &D) -> Result<Self, FenceError> {
        Self::new(device, vk::FenceCreateFlags::SIGNALED)
    }

    pub fn unsignaled<D: HasVkDevice>(device: &D) -> Result<Self, FenceError> {
        Self::new(device, vk::FenceCreateFlags::empty())
    }

    pub fn vk_fence(&self) -> &vk::Fence {
        &self.vk_fence
    }

    pub fn blocking_wait(&self) -> Result<(), FenceError> {
        let fences = [self.vk_fence];
        unsafe {
            self.vk_device
                .wait_for_fences(&fences, true, u64::MAX)
                .map_err(FenceError::Await)?;
        }

        Ok(())
    }

    pub fn reset(&self) -> Result<(), FenceError> {
        let fences = [self.vk_fence];
        unsafe {
            self.vk_device
                .reset_fences(&fences)
                .map_err(FenceError::Reset)?;
        }

        Ok(())
    }
}
