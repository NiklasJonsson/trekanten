use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::descriptor::DescriptorSet;
use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDeviceHandle;
use crate::framebuffer::Framebuffer;
use crate::mesh::IndexBuffer;
use crate::mesh::VertexBuffer;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::Pipeline;
use crate::queue::QueueFamily;
use crate::render_pass::RenderPass;
use crate::util;

// TODO: Merge errors

#[derive(Debug)]
pub enum CommandPoolError {
    Creation(vk::Result),
    CommandBufferAlloc(vk::Result),
    CommandBuffer(CommandBufferError),
}

impl std::error::Error for CommandPoolError {}
impl std::fmt::Display for CommandPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<CommandBufferError> for CommandPoolError {
    fn from(e: CommandBufferError) -> Self {
        Self::CommandBuffer(e)
    }
}

pub struct CommandPool {
    queue_family: QueueFamily,
    vk_command_pool: vk::CommandPool,
    vk_device: VkDeviceHandle,
}

impl std::ops::Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_command_pool(self.vk_command_pool, None);
        }
    }
}

impl CommandPool {
    fn new(device: &Device, qfam: QueueFamily) -> Result<Self, CommandPoolError> {
        let info = vk::CommandPoolCreateInfo {
            queue_family_index: qfam.index,
            ..Default::default()
        };

        let vk_device = device.vk_device();

        let vk_command_pool = unsafe {
            vk_device
                .create_command_pool(&info, None)
                .map_err(CommandPoolError::Creation)?
        };

        Ok(Self {
            queue_family: qfam,
            vk_command_pool,
            vk_device,
        })
    }

    pub fn graphics(device: &Device) -> Result<Self, CommandPoolError> {
        Self::new(device, device.graphics_queue_family().clone())
    }

    pub fn util(device: &Device) -> Result<Self, CommandPoolError> {
        Self::new(device, device.util_queue_family().clone())
    }

    pub fn create_command_buffer(&self) -> Result<CommandBuffer, CommandPoolError> {
        let mut r = self.create_command_buffers(1)?;
        debug_assert_eq!(r.len(), 1);
        Ok(r.remove(0))
    }

    pub fn create_command_buffers(
        &self,
        amount: u32,
    ) -> Result<Vec<CommandBuffer>, CommandPoolError> {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.vk_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(amount);

        let allocated = unsafe {
            self.vk_device
                .allocate_command_buffers(&info)
                .map_err(CommandPoolError::CommandBufferAlloc)?
        };

        Ok(allocated
            .into_iter()
            .map(|vk_cmd_buf| {
                CommandBuffer::new(
                    Rc::clone(&self.vk_device),
                    vk_cmd_buf,
                    self.queue_family.props.queue_flags,
                )
            })
            .collect::<Vec<CommandBuffer>>())
    }

    pub fn begin_single_submit(&self) -> Result<CommandBuffer, CommandPoolError> {
        let buf = self.create_command_buffer()?;
        buf.begin_single_submit()
            .map_err(CommandPoolError::CommandBuffer)
    }
}

#[derive(Debug)]
pub enum CommandBufferError {
    Begin(vk::Result),
    End(vk::Result),
}

impl std::error::Error for CommandBufferError {}
impl std::fmt::Display for CommandBufferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum CommandBufferSubmission {
    Single,
    Multi,
}

// TODO: That we have to call all of these through a device means that might mean that we can't
// "easily" record command buffers on other threads?
// TODO: Builder pattern?
pub struct CommandBuffer {
    queue_flags: vk::QueueFlags,
    vk_cmd_buffer: vk::CommandBuffer,
    vk_device: VkDeviceHandle,
}

impl CommandBuffer {
    pub fn new(
        vk_device: VkDeviceHandle,
        vk_cmd_buffer: vk::CommandBuffer,
        queue_flags: vk::QueueFlags,
    ) -> Self {
        Self {
            vk_cmd_buffer,
            vk_device,
            queue_flags,
        }
    }

    pub fn vk_command_buffer(&self) -> &vk::CommandBuffer {
        &self.vk_cmd_buffer
    }

    pub fn begin(
        self,
        submission_type: CommandBufferSubmission,
    ) -> Result<Self, CommandBufferError> {
        let flags = match submission_type {
            CommandBufferSubmission::Single => vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            _ => vk::CommandBufferUsageFlags::empty(),
        };

        let info = vk::CommandBufferBeginInfo {
            flags,
            ..Default::default()
        };

        unsafe {
            self.vk_device
                .begin_command_buffer(self.vk_cmd_buffer, &info)
                .map_err(CommandBufferError::Begin)?;
        };

        Ok(self)
    }

    pub fn begin_single_submit(self) -> Result<Self, CommandBufferError> {
        self.begin(CommandBufferSubmission::Single)
    }

    pub fn end(self) -> Result<Self, CommandBufferError> {
        unsafe {
            self.vk_device
                .end_command_buffer(self.vk_cmd_buffer)
                .map_err(CommandBufferError::End)?;
        }
        Ok(self)
    }

    pub fn begin_render_pass(
        self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
        extent: util::Extent2D,
    ) -> Self {
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(*render_pass.vk_render_pass())
            .framebuffer(*framebuffer.vk_framebuffer())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: extent.into(),
            })
            .clear_values(render_pass.vk_clear_values());

        unsafe {
            self.vk_device.cmd_begin_render_pass(
                self.vk_cmd_buffer,
                &info,
                vk::SubpassContents::INLINE,
            );
        }

        self
    }

    pub fn end_render_pass(self) -> Self {
        unsafe {
            self.vk_device.cmd_end_render_pass(self.vk_cmd_buffer);
        }

        self
    }

    pub fn bind_graphics_pipeline(self, graphics_pipeline: &GraphicsPipeline) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        unsafe {
            self.vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer,
                GraphicsPipeline::BIND_POINT,
                *graphics_pipeline.vk_pipeline(),
            );
        }

        self
    }

    pub fn bind_vertex_buffer(self, buffer: &VertexBuffer) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        unsafe {
            self.vk_device.cmd_bind_vertex_buffers(
                self.vk_cmd_buffer,
                0,
                &[*buffer.vk_buffer()],
                &[0],
            );
        }

        self
    }

    pub fn bind_index_buffer(self, buffer: &IndexBuffer) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        unsafe {
            self.vk_device.cmd_bind_index_buffer(
                self.vk_cmd_buffer,
                *buffer.vk_buffer(),
                0,
                buffer.vk_index_type(),
            );
        }

        self
    }

    pub fn bind_descriptor_set(self, set: &DescriptorSet, pipeline: &GraphicsPipeline) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        let sets = [*set.vk_descriptor_set()];
        unsafe {
            self.vk_device.cmd_bind_descriptor_sets(
                self.vk_cmd_buffer,
                GraphicsPipeline::BIND_POINT,
                *pipeline.vk_pipeline_layout(),
                0,
                &sets,
                &[],
            );
        }

        self
    }

    /*
    // TODO: Typesafety
    pub fn draw(
        self,
        n_vertices: u32,
        n_instances: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));
        }
        unsafe {
            self.vk_device.cmd_draw(
                self.vk_cmd_buffer,
                n_vertices,
                n_instances,
                first_vertex,
                first_instance,
            );
        }

        self
    }
    */

    pub fn draw_indexed(self, n_vertices: u32) -> Self {
        assert!(self.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        unsafe {
            self.vk_device
                .cmd_draw_indexed(self.vk_cmd_buffer, n_vertices, 1, 0, 0, 0);
        }

        self
    }

    pub fn copy_buffer(self, src: &vk::Buffer, dst: &vk::Buffer, size: usize) -> Self {
        let info = vk::BufferCopy {
            src_offset: 0,
            dst_offset: 0,
            size: size as u64,
        };

        unsafe {
            self.vk_device
                .cmd_copy_buffer(self.vk_cmd_buffer, *src, *dst, &[info]);
        }

        self
    }

    pub fn copy_buffer_to_image(
        self,
        src: &vk::Buffer,
        dst: &vk::Image,
        width: u32,
        height: u32,
    ) -> Self {
        // TODO: Read this info from dst (by passing not just the vk::Image)
        let info = vk::BufferImageCopy {
            buffer_offset: 0,
            // For e.g. padded rows
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
        };

        unsafe {
            self.vk_device.cmd_copy_buffer_to_image(
                self.vk_cmd_buffer,
                *src,
                *dst,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[info],
            );
        }

        self
    }

    pub fn pipeline_barrier(
        self,
        barrier: &vk::ImageMemoryBarrier,
        src_stage: vk::PipelineStageFlags,
        dst_stage: vk::PipelineStageFlags,
    ) -> Self {
        unsafe {
            self.vk_device.cmd_pipeline_barrier(
                self.vk_cmd_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[*barrier],
            );
        }

        self
    }
}
