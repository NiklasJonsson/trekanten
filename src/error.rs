use crate::*;

#[derive(Debug)]
pub enum RenderError {
    CommandBuffer(command::CommandBufferError),
    Instance(instance::InstanceError),
    DebugUtils(util::vk_debug::DebugUtilsError),
    Surface(surface::SurfaceError),
    Device(device::DeviceError),
    Swapchain(swapchain::SwapchainError),
    RenderPass(render_pass::RenderPassError),
    Pipeline(pipeline::PipelineError),
    Material(material::MaterialError),
    CommandPool(command::CommandPoolError),
    Frame(crate::FrameSynchronizationError),
    Fence(sync::FenceError),
    Queue(queue::QueueError),
    VertexBuffer(mem::DeviceBufferError),
    IndexBuffer(mem::DeviceBufferError),
    NeedsResize,
}

impl std::error::Error for RenderError {}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<command::CommandBufferError> for RenderError {
    fn from(cbe: command::CommandBufferError) -> Self {
        Self::CommandBuffer(cbe)
    }
}

impl From<instance::InstanceError> for RenderError {
    fn from(ie: instance::InstanceError) -> Self {
        Self::Instance(ie)
    }
}

impl From<util::vk_debug::DebugUtilsError> for RenderError {
    fn from(e: util::vk_debug::DebugUtilsError) -> Self {
        Self::DebugUtils(e)
    }
}

impl From<surface::SurfaceError> for RenderError {
    fn from(e: surface::SurfaceError) -> Self {
        Self::Surface(e)
    }
}

impl From<device::DeviceError> for RenderError {
    fn from(e: device::DeviceError) -> Self {
        Self::Device(e)
    }
}

impl From<swapchain::SwapchainError> for RenderError {
    fn from(e: swapchain::SwapchainError) -> Self {
        Self::Swapchain(e)
    }
}

impl From<render_pass::RenderPassError> for RenderError {
    fn from(e: render_pass::RenderPassError) -> Self {
        Self::RenderPass(e)
    }
}

impl From<pipeline::PipelineError> for RenderError {
    fn from(e: pipeline::PipelineError) -> Self {
        Self::Pipeline(e)
    }
}

impl From<command::CommandPoolError> for RenderError {
    fn from(e: command::CommandPoolError) -> Self {
        Self::CommandPool(e)
    }
}

impl From<crate::FrameSynchronizationError> for RenderError {
    fn from(e: crate::FrameSynchronizationError) -> Self {
        Self::Frame(e)
    }
}

impl From<sync::FenceError> for RenderError {
    fn from(e: sync::FenceError) -> Self {
        Self::Fence(e)
    }
}

impl From<queue::QueueError> for RenderError {
    fn from(e: queue::QueueError) -> Self {
        Self::Queue(e)
    }
}

impl From<material::MaterialError> for RenderError {
    fn from(e: material::MaterialError) -> Self {
        Self::Material(e)
    }
}
