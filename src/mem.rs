use ash::version::DeviceV1_0;
use ash::vk;

use crate::command::CommandBufferError;
use crate::command::CommandPool;
use crate::command::CommandPoolError;
use crate::device::AsVkDevice;
use crate::device::Device;
use crate::queue::Queue;
use crate::queue::QueueError;
use crate::sync::Fence;
use crate::sync::FenceError;

#[derive(Debug)]
pub enum MemoryError {
    BufferCreation(vk::Result),
    Allocation(vk::Result),
    BufferBinding(vk::Result),
    CopyCommandPool(CommandPoolError),
    CopyCommandBuffer(CommandBufferError),
    CopySync(FenceError),
    CopySubmit(QueueError),
}

impl std::error::Error for MemoryError {}
impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn find_memory_type(
    device: &Device,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Option<u32> {
    let mem_props = device.memory_properties();

    for i in 0..mem_props.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && (mem_props.memory_types[i as usize].property_flags & properties == properties)
        {
            return Some(i);
        }
    }

    return None;
}

pub fn create_buffer(
    device: &Device,
    size: usize,
    usage_flags: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory), MemoryError> {
    let buffer_info = vk::BufferCreateInfo {
        size: size as u64,
        usage: usage_flags,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };

    let vk_device = device.vk_device();
    let buffer = unsafe {
        vk_device
            .create_buffer(&buffer_info, None)
            .map_err(MemoryError::BufferCreation)?
    };

    let mem_reqs = unsafe { vk_device.get_buffer_memory_requirements(buffer) };

    let memory_type_index = find_memory_type(device, mem_reqs.memory_type_bits, properties)
        .expect("Failed to find appropriate memory type");

    let alloc_info = vk::MemoryAllocateInfo {
        allocation_size: mem_reqs.size,
        memory_type_index,
        ..Default::default()
    };

    let device_memory = unsafe {
        vk_device
            .allocate_memory(&alloc_info, None)
            .map_err(MemoryError::Allocation)?
    };

    unsafe {
        vk_device
            .bind_buffer_memory(buffer, device_memory, 0)
            .map_err(MemoryError::BufferBinding)?;
    };

    Ok((buffer, device_memory))
}

pub fn copy_buffer(
    device: &Device,
    queue: &Queue,
    command_pool: &CommandPool,
    src: vk::Buffer,
    dst: vk::Buffer,
    size: usize,
) -> Result<(), MemoryError> {
    let cmd_buf = command_pool
        .create_command_buffer()
        .map_err(MemoryError::CopyCommandPool)?;
    let vk_cmd_buf = cmd_buf
        .begin_single_submit()
        .map_err(MemoryError::CopyCommandBuffer)?
        .copy_buffer(src, dst, size)
        .end()
        .map_err(MemoryError::CopyCommandBuffer)?
        .vk_command_buffer();

    let bufs = [vk_cmd_buf];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&bufs);

    let copied = Fence::unsignaled(device).map_err(MemoryError::CopySync)?;
    queue
        .submit(&submit_info, &copied)
        .map_err(MemoryError::CopySubmit)?;

    // TODO: Async?
    copied.blocking_wait().map_err(MemoryError::CopySync)?;
    Ok(())
}
