use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::{AsVkDevice, VkDevice};
use crate::sync::Fence;

#[derive(Debug, Copy, Clone)]
pub enum QueueError {
    Submit(vk::Result),
}

impl std::error::Error for QueueError {}
impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct QueueFamily {
    pub index: u32,
    pub props: vk::QueueFamilyProperties,
}

#[derive(Clone, Debug)]
pub struct QueueFamilies {
    pub graphics: QueueFamily,
    pub present: QueueFamily,
}

#[derive(Clone)]
pub struct Queue {
    vk_device: Rc<VkDevice>,
    vk_queue: vk::Queue,
}

impl Queue {
    pub fn new<D: AsVkDevice>(device: D, vk_queue: vk::Queue) -> Self {
        Self {
            vk_device: device.vk_device(),
            vk_queue,
        }
    }

    pub fn submit(&self, info: &vk::SubmitInfo, fence: &Fence) -> Result<(), QueueError> {
        let infos = [*info];
        unsafe {
            self.vk_device
                .queue_submit(self.vk_queue, &infos, *fence.vk_fence())
                .map_err(QueueError::Submit)?;
        }

        Ok(())
    }

    pub fn vk_queue(&self) -> &vk::Queue {
        &self.vk_queue
    }
}
