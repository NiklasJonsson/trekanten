use crate::*;

#[derive(Debug, Clone, Copy)]
pub enum ResizeReason {
    OutOfDate,
    SubOptimal,
}

#[derive(Debug)]
pub enum RenderError {
    Command(command::CommandError),
    Instance(instance::InstanceError),
    DebugUtils(util::vk_debug::DebugUtilsError),
    Surface(surface::SurfaceError),
    Device(device::DeviceError),
    Swapchain(swapchain::SwapchainError),
    RenderPass(render_pass::RenderPassError),
    Pipeline(pipeline::PipelineError),
    Frame(crate::FrameSynchronizationError),
    Fence(sync::FenceError),
    Queue(queue::QueueError),
    VertexBuffer(mem::MemoryError),
    IndexBuffer(mem::MemoryError),
    NeedsResize(ResizeReason),
    MissingUniformBuffersForDescriptor,
    DescriptorSet(descriptor::DescriptorSetError),
}

impl std::error::Error for RenderError {}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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

impl From<command::CommandError> for RenderError {
    fn from(e: command::CommandError) -> Self {
        Self::Command(e)
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
