use ash::version::DeviceV1_0;
use ash::vk;

use crate::command::CommandPool;
use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDeviceHandle;
use crate::mem;
use crate::queue::Queue;

pub trait VertexDefinition {
    fn binding_description() -> Vec<vk::VertexInputBindingDescription>;
    fn attribute_description() -> Vec<vk::VertexInputAttributeDescription>;
}

pub trait VertexSource {
    fn binding_description(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription>;
}

impl<V: VertexDefinition> VertexSource for Vec<V> {
    fn binding_description(&self) -> Vec<vk::VertexInputBindingDescription> {
        V::binding_description()
    }

    fn attribute_description(&self) -> Vec<vk::VertexInputAttributeDescription> {
        V::attribute_description()
    }
}

#[derive(Debug)]
pub enum VertexBufferError {
    Memory(mem::MemoryError),
    MemoryMapping(vk::Result),
}

impl std::error::Error for VertexBufferError {}
impl std::fmt::Display for VertexBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<mem::MemoryError> for VertexBufferError {
    fn from(e: mem::MemoryError) -> Self {
        Self::Memory(e)
    }
}

pub struct VertexBuffer {
    vk_device: VkDeviceHandle,
    vertex_buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
}

impl std::ops::Drop for VertexBuffer {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_buffer(self.vertex_buffer, None);
            self.vk_device.free_memory(self.device_memory, None);
        }
    }
}

impl VertexBuffer {
    pub fn from_slice<V>(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        slice: &[V],
    ) -> Result<Self, VertexBufferError> {
        let vk_device = device.vk_device();
        let size = std::mem::size_of::<V>() * slice.len();

        let (staging_buffer, staging_memory) = mem::create_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        unsafe {
            let mapped_ptr = vk_device
                .map_memory(staging_memory, 0, size as u64, vk::MemoryMapFlags::empty())
                .map_err(VertexBufferError::MemoryMapping)?;
            let src = slice.as_ptr() as *const u8;
            let dst = mapped_ptr as *mut u8;
            std::ptr::copy_nonoverlapping::<u8>(src, dst, size);
            vk_device.unmap_memory(staging_memory);
        }

        let (vertex_buffer, device_memory) = mem::create_buffer(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        mem::copy_buffer(
            device,
            queue,
            command_pool,
            staging_buffer,
            vertex_buffer,
            size,
        )?;

        unsafe {
            vk_device.destroy_buffer(staging_buffer, None);
            vk_device.free_memory(staging_memory, None);
        }

        Ok(Self {
            vk_device,
            vertex_buffer,
            device_memory,
        })
    }

    pub fn vk_buffer(&self) -> &vk::Buffer {
        &self.vertex_buffer
    }
}
