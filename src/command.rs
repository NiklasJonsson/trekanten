use ash::version::DeviceV1_0;
use ash::vk;

use std::rc::Rc;

use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDevice;
use crate::framebuffer::Framebuffer;
use crate::instance::InitError;
use crate::pipeline::GraphicsPipeline;
use crate::pipeline::Pipeline;
use crate::queue::QueueFamily;
use crate::render_pass::RenderPass;
use crate::util;

#[derive(Debug)]
pub enum CommandPoolError {
    Init(InitError),
    CommandBufferAlloc(vk::Result),
}

impl std::error::Error for CommandPoolError {}
impl std::fmt::Display for CommandPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<InitError> for CommandPoolError {
    fn from(e: InitError) -> Self {
        Self::Init(e)
    }
}

pub struct CommandPool {
    queue_family: QueueFamily,
    vk_command_pool: vk::CommandPool,
    vk_device: Rc<VkDevice>,
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
    pub fn graphics(device: &Device) -> Result<Self, CommandPoolError> {
        let qfam = device.graphics_queue_family().clone();

        let info = vk::CommandPoolCreateInfo {
            queue_family_index: qfam.index,
            ..Default::default()
        };

        let vk_device = device.vk_device();

        let vk_command_pool = unsafe {
            vk_device
                .create_command_pool(&info, None)
                .map_err(InitError::from)?
        };

        Ok(Self {
            queue_family: qfam,
            vk_command_pool,
            vk_device,
        })
    }

    pub fn create_command_buffers(
        &self,
        amount: u32,
    ) -> Result<Vec<CommandBuffer>, CommandPoolError> {
        let info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.vk_command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(amount);

        Ok(unsafe {
            self.vk_device
                .allocate_command_buffers(&info)
                .map(|vec| {
                    vec.into_iter()
                        .map(|vk_cmd_buf| {
                            CommandBuffer::new(
                                Rc::clone(&self.vk_device),
                                vk_cmd_buf,
                                self.queue_family.props.queue_flags,
                            )
                        })
                        .collect::<Vec<CommandBuffer>>()
                })
                .map_err(CommandPoolError::CommandBufferAlloc)?
        })
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

// TODO: That we have to call all of these through a device means that might mean that we can't
// "easily" record command buffers on other threads?
pub struct CommandBuffer {
    queue_flags: vk::QueueFlags,
    vk_cmd_buffer: vk::CommandBuffer,
    vk_device: Rc<VkDevice>,
}

impl CommandBuffer {
    pub fn new(
        vk_device: Rc<VkDevice>,
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

    pub fn begin(self) -> Result<Self, CommandBufferError> {
        let info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::empty(),
            ..Default::default()
        };

        unsafe {
            self.vk_device
                .begin_command_buffer(self.vk_cmd_buffer, &info)
                .map_err(CommandBufferError::Begin)?;
        };

        Ok(self)
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
        let clear_values = [vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }];

        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(*render_pass.inner_vk_render_pass())
            .framebuffer(*framebuffer.inner_vk_framebuffer())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: extent.into(),
            })
            .clear_values(&clear_values);

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

    pub fn bind_gfx_pipeline(self, pipeline: &GraphicsPipeline) -> Self {
        // TODO: Verify queue family here

        unsafe {
            self.vk_device.cmd_bind_pipeline(
                self.vk_cmd_buffer,
                GraphicsPipeline::BIND_POINT,
                *pipeline.vk_pipeline(),
            );
        }

        self
    }

    // TODO: Typesafety
    pub fn draw(
        self,
        n_vertices: u32,
        n_instances: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
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
}
