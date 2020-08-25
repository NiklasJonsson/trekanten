use ash::version::DeviceV1_0;
use ash::vk;

use crate::command::CommandBuffer;
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
    mip_levels: u32,
    _vk_format: vk::Format,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> Result<(), MemoryError> {
    // Note: The barrier below does not really matter at the moment as we wait on the fence
    // directly after submitting. If the code is used elsewhere, it makes the following
    // assumptions:
    // * The image is only read in the fragment shader
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
            level_count: mip_levels,
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

// TODO: This code depends on vk_image being TRANSfER_DST_OPTIMAL. We should track this together
// with the image.
fn generate_mipmaps(
    mut cmd_buf: CommandBuffer,
    vk_image: &vk::Image,
    extent: &util::Extent2D,
    mip_levels: u32,
) -> CommandBuffer {
    assert!(cmd_buf.is_started());
    let aspect_mask = vk::ImageAspectFlags::COLOR;

    let mut barrier = vk::ImageMemoryBarrier {
        image: *vk_image,
        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut mip_width = extent.width;
    let mut mip_height = extent.height;

    for i in 1..mip_levels {
        barrier.subresource_range.base_mip_level = i - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;

        let src_offsets = [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: mip_width as i32,
                y: mip_height as i32,
                z: 1,
            },
        ];

        let dst_x = if mip_width > 1 { mip_width / 2 } else { 1 } as i32;
        let dst_y = if mip_height > 1 { mip_height / 2 } else { 1 } as i32;
        let dst_offsets = [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: dst_x,
                y: dst_y,
                z: 1,
            },
        ];

        let image_blit = vk::ImageBlit {
            src_offsets,
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask,
                mip_level: i - 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_offsets,
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask,
                mip_level: i,
                base_array_layer: 0,
                layer_count: 1,
            },
        };

        let transistion_src_barrier = vk::ImageMemoryBarrier {
            old_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_access_mask: vk::AccessFlags::TRANSFER_READ,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            ..barrier
        };

        cmd_buf = cmd_buf
            .pipeline_barrier(
                &barrier,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
            )
            .blit_image(vk_image, vk_image, &image_blit)
            .pipeline_barrier(
                &transistion_src_barrier,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            );

        mip_width = if mip_width > 1 {
            mip_width / 2
        } else {
            mip_width
        };
        mip_height = if mip_height > 1 {
            mip_height / 2
        } else {
            mip_height
        };
    }

    let last_mip_level_transition = vk::ImageMemoryBarrier {
        subresource_range: vk::ImageSubresourceRange {
            base_mip_level: mip_levels - 1,
            ..barrier.subresource_range
        },
        old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
        dst_access_mask: vk::AccessFlags::SHADER_READ,
        ..barrier
    };

    cmd_buf.pipeline_barrier(
        &last_mip_level_transition,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
    )
}

pub struct DeviceImage {
    vk_device: VkDeviceHandle,
    vk_image: vk::Image,
    device_memory: vk::DeviceMemory,
}

impl DeviceImage {
    pub fn empty_2d(
        device: &Device,
        extents: util::Extent2D,
        format: util::Format,
        usage: vk::ImageUsageFlags,
        memory_properties: vk::MemoryPropertyFlags,
        mip_levels: u32,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Self, MemoryError> {
        log::trace!("Creating empty 2D DeviceImage with:");
        log::trace!("\textents: {}", extents);
        log::trace!("\tformat: {:?}", format);
        log::trace!("\tusage: {:?}", usage);
        log::trace!("\tmemory properties: {:?}", memory_properties);
        log::trace!("\tmip level: {}", mip_levels);
        log::trace!("\tsample count: {:?}", sample_count);
        log::trace!("\timage tiling {:?}", vk::ImageTiling::OPTIMAL);

        let extents3d = util::Extent3D::from_2d(extents, 1);
        let info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extents3d.into())
            .mip_levels(mip_levels)
            .array_layers(1)
            .format(format.into())
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(sample_count);

        let vk_device = device.vk_device();
        let vk_image = unsafe {
            vk_device
                .create_image(&info, None)
                .map_err(MemoryError::ImageCreation)?
        };

        let mem_reqs = unsafe { vk_device.get_image_memory_requirements(vk_image) };

        let device_memory = alloc_memory(device, mem_reqs, memory_properties)?;

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

    /// Create a device local image, generating mipmaps in the process
    pub fn device_local_mipmapped(
        device: &Device,
        queue: &Queue,
        command_pool: &CommandPool,
        extents: util::Extent2D,
        format: util::Format,
        mip_levels: u32,
        data: &[u8],
    ) -> Result<Self, MemoryError> {
        let staging = DeviceBuffer::staging_with_data(device, data)?;
        let usage = vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::SAMPLED;
        let dst_image = Self::empty_2d(
            device,
            extents,
            format,
            usage,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            mip_levels,
            vk::SampleCountFlags::TYPE_1,
        )?;

        transition_image_layout(
            queue,
            command_pool,
            &dst_image.vk_image,
            mip_levels,
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

        // Transitioned to SHADER_READ_ONLY_OPTIMAL during mipmap generation
        // TODO: Reuse this command buffer above
        let mut cmd_buf = command_pool
            .begin_single_submit()
            .map_err(MemoryError::CopyCommandPool)?;

        cmd_buf = generate_mipmaps(cmd_buf, dst_image.vk_image(), &extents, mip_levels)
            .end()
            .map_err(MemoryError::CopyCommandBuffer)?;

        queue
            .submit_and_wait(&cmd_buf)
            .map_err(MemoryError::CopySubmit)?;

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
