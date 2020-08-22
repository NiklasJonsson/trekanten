use ash::version::DeviceV1_0;
use ash::vk;

use crate::command::CommandBufferError;
use crate::command::CommandPool;
use crate::command::CommandPoolError;
use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDeviceHandle;
use crate::queue::Queue;
use crate::queue::QueueError;
use crate::util;

#[derive(Debug)]
pub enum MemoryError {
    BufferCreation(vk::Result),
    ImageCreation(vk::Result),
    Allocation(vk::Result),
    BufferBinding(vk::Result),
    ImageBinding(vk::Result),
    CopyCommandPool(CommandPoolError),
    CopyCommandBuffer(CommandBufferError),
    CopySubmit(QueueError),
    MemoryMapping(vk::Result),
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

    None
}

fn alloc_memory(
    device: &Device,
    mem_reqs: vk::MemoryRequirements,
    mem_props: vk::MemoryPropertyFlags,
) -> Result<vk::DeviceMemory, MemoryError> {
    let vk_device = device.vk_device();
    let memory_type_index = find_memory_type(device, mem_reqs.memory_type_bits, mem_props)
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

    Ok(device_memory)
}

pub struct DeviceBuffer {
    vk_device: VkDeviceHandle,
    buffer: vk::Buffer,
    device_memory: vk::DeviceMemory,
    is_host_avail: bool,
    size: usize,
}

impl DeviceBuffer {
    pub fn empty(
        device: &Device,
        size: usize,
        usage_flags: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
    ) -> Result<Self, MemoryError> {
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
        let device_memory = alloc_memory(device, mem_reqs, properties)?;

        unsafe {
            vk_device
                .bind_buffer_memory(buffer, device_memory, 0)
                .map_err(MemoryError::BufferBinding)?;
        };

        let is_host_avail = properties.contains(
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        );

        Ok(Self {
            vk_device,
            buffer,
            device_memory,
            is_host_avail,
            size,
        })
    }

    pub fn staging_empty(device: &Device, size: usize) -> Result<Self, MemoryError> {
        DeviceBuffer::empty(
            device,
            size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )
    }

    pub fn staging_with_data(device: &Device, data: &[u8]) -> Result<Self, MemoryError> {
        let vk_device = device.vk_device();
        let size = data.len();

        let staging = DeviceBuffer::staging_empty(device, size)?;

        unsafe {
            let mapped_ptr = vk_device
                .map_memory(
                    staging.device_memory,
                    0,
                    size as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(MemoryError::MemoryMapping)?;
            let src = data.as_ptr() as *const u8;
            let dst = mapped_ptr as *mut u8;
            std::ptr::copy_nonoverlapping::<u8>(src, dst, size);
            vk_device.unmap_memory(staging.device_memory);
        }

        Ok(staging)
    }

    pub fn device_local_by_staging(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        usage: vk::BufferUsageFlags,
        data: &[u8],
    ) -> Result<Self, MemoryError> {
        let staging = Self::staging_with_data(device, data)?;

        let dst_buffer = Self::empty(
            device,
            staging.size,
            vk::BufferUsageFlags::TRANSFER_DST | usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        copy_buffer(
            queue,
            command_pool,
            &staging.buffer,
            &dst_buffer.buffer,
            staging.size,
        )?;

        Ok(dst_buffer)
    }

    pub fn vk_buffer(&self) -> &vk::Buffer {
        &self.buffer
    }

    pub fn update_data_at(&mut self, data: &[u8], offset: usize) -> Result<(), MemoryError> {
        assert!(self.is_host_avail);
        let size = data.len();
        unsafe {
            let mapped_ptr = self
                .vk_device
                .map_memory(
                    self.device_memory,
                    offset as vk::DeviceSize,
                    size as vk::DeviceSize,
                    vk::MemoryMapFlags::empty(),
                )
                .map_err(MemoryError::MemoryMapping)?;
            let src = data.as_ptr() as *const u8;
            let dst = mapped_ptr as *mut u8;
            std::ptr::copy_nonoverlapping::<u8>(src, dst, size);
            self.vk_device.unmap_memory(self.device_memory);
        }

        Ok(())
    }
}

impl std::ops::Drop for DeviceBuffer {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_buffer(self.buffer, None);
            self.vk_device.free_memory(self.device_memory, None);
        }
    }
}

fn copy_buffer(
    queue: &Queue,
    command_pool: &CommandPool,
    src: &vk::Buffer,
    dst: &vk::Buffer,
    size: usize,
) -> Result<(), MemoryError> {
    let cmd_buf = command_pool
        .begin_single_submit()
        .map_err(MemoryError::CopyCommandPool)?
        .copy_buffer(&src, &dst, size)
        .end()
        .map_err(MemoryError::CopyCommandBuffer)?;

    queue
        .submit_and_wait(&cmd_buf)
        .map_err(MemoryError::CopySubmit)
}

fn transition_image_layout(
    queue: &Queue,
    command_pool: &CommandPool,
    vk_image: &vk::Image,
    _vk_format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<(), MemoryError> {
    // Note: The barrier below does not really matter at the moment as we wait on the fence
    // directly after submitting. If the code is used elsewhere, it makes the following
    // assumptions:
    // * The image is only read in the fragment shader
    // * The image has no mip map levels
    // * The image is not an image array
    // * The image is only used in one queue

    let (src_mask, src_stage, dst_mask, dst_stage) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TRANSFER,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        _ => unimplemented!(),
    };

    let barrier = vk::ImageMemoryBarrier {
        old_layout,
        new_layout,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        image: *vk_image,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        src_access_mask: src_mask,
        dst_access_mask: dst_mask,
        ..Default::default()
    };

    let cmd_buf = command_pool
        .begin_single_submit()
        .map_err(MemoryError::CopyCommandPool)?
        .pipeline_barrier(&barrier, src_stage, dst_stage)
        .end()
        .map_err(MemoryError::CopyCommandBuffer)?;

    queue
        .submit_and_wait(&cmd_buf)
        .map_err(MemoryError::CopySubmit)
}

fn copy_buffer_to_image(
    queue: &Queue,
    command_pool: &CommandPool,
    src: &vk::Buffer,
    dst: &vk::Image,
    width: u32,
    height: u32,
) -> Result<(), MemoryError> {
    let cmd_buf = command_pool
        .begin_single_submit()
        .map_err(MemoryError::CopyCommandPool)?
        .copy_buffer_to_image(src, dst, width, height)
        .end()
        .map_err(MemoryError::CopyCommandBuffer)?;

    queue
        .submit_and_wait(&cmd_buf)
        .map_err(MemoryError::CopySubmit)
}

pub struct DeviceImage {
    vk_device: VkDeviceHandle,
    vk_image: vk::Image,
    device_memory: vk::DeviceMemory,
}

impl DeviceImage {
    // Use this for device local image sampling from e.g. shaders
    // Get's its values from a copy from a staging buffer
    fn empty_dst_2d(
        device: &Device,
        extents: util::Extent2D,
        format: util::Format,
    ) -> Result<Self, MemoryError> {
        let extents3d = util::Extent3D::from_2d(extents, 1);
        let info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extents3d.into())
            .mip_levels(1)
            .array_layers(1)
            .format(format.into())
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);

        let vk_device = device.vk_device();
        let vk_image = unsafe {
            vk_device
                .create_image(&info, None)
                .map_err(MemoryError::ImageCreation)?
        };

        let mem_reqs = unsafe { vk_device.get_image_memory_requirements(vk_image) };

        let device_memory = alloc_memory(device, mem_reqs, vk::MemoryPropertyFlags::DEVICE_LOCAL)?;

        unsafe {
            vk_device
                .bind_image_memory(vk_image, device_memory, 0)
                .map_err(MemoryError::ImageBinding)?;
        };

        Ok(Self {
            vk_device,
            vk_image,
            device_memory,
        })
    }

    pub fn device_local_by_staging(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        extents: util::Extent2D,
        format: util::Format,
        data: &[u8],
    ) -> Result<Self, MemoryError> {
        let staging = DeviceBuffer::staging_with_data(device, data)?;
        let dst_image = Self::empty_dst_2d(device, extents, format)?;
        // Bake into empty_dst_2d?
        transition_image_layout(
            queue,
            command_pool,
            &dst_image.vk_image,
            format.into(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;

        copy_buffer_to_image(
            queue,
            command_pool,
            &staging.buffer,
            &dst_image.vk_image,
            extents.width,
            extents.height,
        )?;

        transition_image_layout(
            queue,
            command_pool,
            &dst_image.vk_image,
            format.into(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        Ok(dst_image)
    }

    pub fn vk_image(&self) -> &vk::Image {
        &self.vk_image
    }
}

impl std::ops::Drop for DeviceImage {
    fn drop(&mut self) {
        unsafe {
            self.vk_device.destroy_image(self.vk_image, None);
            self.vk_device.free_memory(self.device_memory, None);
        }
    }
}
