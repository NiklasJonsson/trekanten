use ash::vk;

use crate::command::CommandPool;
use crate::device::Device;
use crate::mem;
use crate::queue::Queue;

#[derive(Debug)]
pub enum IndexBufferError {
    Memory(mem::MemoryError),
    MemoryMapping(vk::Result),
}

impl std::error::Error for IndexBufferError {}
impl std::fmt::Display for IndexBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<mem::MemoryError> for IndexBufferError {
    fn from(e: mem::MemoryError) -> Self {
        Self::Memory(e)
    }
}

pub struct IndexBuffer(pub mem::DeviceBuffer);

impl IndexBuffer {
    pub fn from_slice<V>(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        slice: &[V],
    ) -> Result<Self, mem::DeviceBufferError> {
        mem::DeviceBuffer::from_slice_staging(
            device,
            queue,
            command_pool,
            vk::BufferUsageFlags::INDEX_BUFFER,
            slice,
        )
        .map(Self)
    }

    pub fn vk_buffer(&self) -> &vk::Buffer {
        &self.0.vk_buffer()
    }
}

pub struct VertexBuffer(pub mem::DeviceBuffer);

impl VertexBuffer {
    pub fn from_slice<V>(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        slice: &[V],
    ) -> Result<Self, mem::DeviceBufferError> {
        mem::DeviceBuffer::from_slice_staging(
            device,
            queue,
            command_pool,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            slice,
        )
        .map(Self)
    }

    pub fn vk_buffer(&self) -> &vk::Buffer {
        &self.0.vk_buffer()
    }
}
