use ash::vk;

mod command;
mod device;
mod error;
mod framebuffer;
mod image;
mod instance;
mod pipeline;
mod queue;
mod render_pass;
mod surface;
mod swapchain;
mod sync;
mod util;
pub mod window;

pub use error::RenderError;

// Notes:
// We can have N number of swapchain images, it depends on the backing presentation implementation.
// Generally, we are aiming for three images + MAILBOX (render one and use the latest of the two
// waiting)
// Per swapchain image resources:
// - Framebuffer
// - Pre-recorded command buffers (as they are bound to framebuffers, which is 1 per sc image)
//
// We use N (2, hardcoded atm) frames in flight at once. This allows us to start the next framedirectly after we render. Whenever next_frame() is called, it can be though of as binding one of the two frames to a particular swapchain image. All rendering in that frame will be bound to that.

#[derive(Debug)]
pub enum FrameSynchronizationError {
    Semaphore(sync::SemaphoreError),
    Fence(sync::FenceError),
}

impl std::error::Error for FrameSynchronizationError {}
impl std::fmt::Display for FrameSynchronizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<sync::SemaphoreError> for FrameSynchronizationError {
    fn from(e: sync::SemaphoreError) -> Self {
        Self::Semaphore(e)
    }
}

impl From<sync::FenceError> for FrameSynchronizationError {
    fn from(e: sync::FenceError) -> Self {
        Self::Fence(e)
    }
}

pub struct FrameSynchronization {
    pub image_available: sync::Semaphore,
    pub render_done: sync::Semaphore,
    pub in_flight: sync::Fence,
}

impl FrameSynchronization {
    pub fn new(device: &device::Device) -> Result<Self, FrameSynchronizationError> {
        let image_avail = sync::Semaphore::new(device)?;
        let render_done = sync::Semaphore::new(device)?;
        let in_flight = sync::Fence::new(device)?;

        Ok(Self {
            image_available: image_avail,
            render_done,
            in_flight,
        })
    }
}

pub struct Frame {
    frame_idx: u32,
    swapchain_image_idx: u32,
    recorded_command_buffers: Vec<vk::CommandBuffer>,
    gfx_command_pool: command::CommandPool,
}

impl Frame {
    pub fn new_command_buffer(&self) -> Result<command::CommandBuffer, command::CommandPoolError> {
        self.gfx_command_pool.create_command_buffer()
    }

    pub fn add_command_buffer(&mut self, cmd_buffer: command::CommandBuffer) {
        self.recorded_command_buffers
            .push(cmd_buffer.vk_command_buffer());
    }
}

// TODO: Don't hardcode
pub const WINDOW_HEIGHT: u32 = 300;
pub const WINDOW_WIDTH: u32 = 300;

const MAX_FRAMES_IN_FLIGHT: u32 = 2;
pub struct Renderer {
    // Swapchain-related
    // TODO: Could render pass be a abstracted as forward-renderer?
    render_pass: render_pass::RenderPass,
    swapchain_framebuffers: Vec<framebuffer::Framebuffer>,
    gfx_pipeline: pipeline::GraphicsPipeline,

    swapchain: swapchain::Swapchain,
    swapchain_image_idx: u32, // TODO: Bake this into the swapchain?
    image_to_frame_idx: Vec<Option<u32>>,

    // Needs to be kept-alive
    _debug_utils: util::vk_debug::DebugUtils,

    frame_synchronization: [FrameSynchronization; MAX_FRAMES_IN_FLIGHT as usize],
    frame_idx: u32,
    frames: [Option<Frame>; MAX_FRAMES_IN_FLIGHT as usize],

    device: device::Device,
    surface: surface::Surface,
    instance: instance::Instance,
}

impl std::ops::Drop for Renderer {
    fn drop(&mut self) {
        // If we fail here, there is not much we can do, just log it.
        if let Err(e) = self.device.wait_idle() {
            log::error!("{}", e);
        }
    }
}

// Result holder struct
struct SwapchainAndCo {
    swapchain: swapchain::Swapchain,
    swapchain_framebuffers: Vec<framebuffer::Framebuffer>,
    image_to_frame_idx: Vec<Option<u32>>,
    render_pass: render_pass::RenderPass,
    // TODO: Change to dynamic viewport and we don't have to recompile
    gfx_pipeline: pipeline::GraphicsPipeline,
}

fn create_swapchain_and_co(
    instance: &instance::Instance,
    device: &device::Device,
    surface: &surface::Surface,
    extent: &util::Extent2D,
    _old: Option<&swapchain::Swapchain>,
) -> Result<SwapchainAndCo, RenderError> {
    let swapchain = swapchain::Swapchain::new(&instance, &device, &surface, &extent)?;
    let render_pass = render_pass::RenderPass::new(&device, swapchain.info().format)?;

    let image_to_frame_idx: Vec<Option<u32>> = (0..swapchain.num_images()).map(|_| None).collect();
    let gfx_pipeline = pipeline::GraphicsPipeline::new(
        &device,
        swapchain.info().extent,
        &render_pass,
        "vert.spv",
        "frag.spv",
    )?;

    let swapchain_framebuffers = swapchain.create_framebuffers_for(&render_pass)?;

    Ok(SwapchainAndCo {
        swapchain,
        swapchain_framebuffers,
        image_to_frame_idx,
        render_pass,
        gfx_pipeline,
    })
}

impl Renderer {
    pub fn new<W>(window: &W) -> Result<Self, RenderError>
    where
        W: raw_window_handle::HasRawWindowHandle + window::Window,
    {
        let extensions = window.required_instance_extensions();

        let instance = instance::Instance::new(&extensions)?;
        let _debug_utils = util::vk_debug::DebugUtils::new(&instance)?;
        let surface = surface::Surface::new(&instance, window)?;
        let device = device::Device::new(&instance, &surface)?;

        let extent = util::Extent2D {
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
        };

        let SwapchainAndCo {
            swapchain,
            swapchain_framebuffers,
            image_to_frame_idx,
            render_pass,
            gfx_pipeline,
        } = create_swapchain_and_co(&instance, &device, &surface, &extent, None)?;

        let frames = [None, None];
        let frame_synchronization = [
            FrameSynchronization::new(&device)?,
            FrameSynchronization::new(&device)?,
        ];

        Ok(Self {
            instance,
            surface,
            device,
            swapchain,
            image_to_frame_idx,
            render_pass,
            swapchain_framebuffers,
            gfx_pipeline,
            frame_synchronization,
            frame_idx: 0,
            frames,
            swapchain_image_idx: 0,
            _debug_utils,
        })
    }

    pub fn next_frame(&mut self) -> Result<Frame, RenderError> {
        let frame_sync = &self.frame_synchronization[self.frame_idx as usize];
        frame_sync.in_flight.blocking_wait()?;

        self.swapchain_image_idx = self
            .swapchain
            .acquire_next_image(Some(&frame_sync.image_available))
            .map_err(|e| {
                if let swapchain::SwapchainError::OutOfDate = e {
                    RenderError::NeedsResize
                } else {
                    RenderError::Swapchain(e)
                }
            })?;

        // This means that we received an image that might be in the process of rendering
        if let Some(frame_idx) = self.image_to_frame_idx[self.swapchain_image_idx as usize] {
            self.frame_synchronization[frame_idx as usize]
                .in_flight
                .blocking_wait()?;
        }

        // This will drop the frame that resided here previously
        std::mem::replace(&mut self.frames[self.frame_idx as usize], None);

        let gfx_command_pool = command::CommandPool::graphics(&self.device)?;

        self.image_to_frame_idx[self.swapchain_image_idx as usize] = Some(self.frame_idx);

        Ok(Frame {
            frame_idx: self.frame_idx,
            swapchain_image_idx: self.swapchain_image_idx,
            recorded_command_buffers: Vec::new(),
            gfx_command_pool,
        })
    }

    pub fn submit(&mut self, frame: Frame) -> Result<(), RenderError> {
        assert_eq!(frame.frame_idx, self.frame_idx, "Mismatching frame indexes");

        // Make sure that this is captured before any early returns. If this function returns
        // without having extended the lifetime of frame, it might be dropped while it's command
        // buffers are still in use.
        self.frames[self.frame_idx as usize] = Some(frame);
        let frame = self.frames[self.frame_idx as usize].as_ref().unwrap();

        let frame_sync = &self.frame_synchronization[self.frame_idx as usize];
        let vk_wait_sems = [*frame_sync.image_available.vk_semaphore()];
        let wait_dst_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let vk_sig_sems = [*frame_sync.render_done.vk_semaphore()];

        let info = vk::SubmitInfo::builder()
            .wait_semaphores(&vk_wait_sems)
            .wait_dst_stage_mask(&wait_dst_mask)
            .signal_semaphores(&vk_sig_sems)
            .command_buffers(&frame.recorded_command_buffers);

        let gfx_queue = self.device.graphics_queue();
        frame_sync.in_flight.reset()?;

        gfx_queue.submit(&info, &frame_sync.in_flight)?;

        let swapchains = [*self.swapchain.vk_swapchain()];
        let indices = [self.swapchain_image_idx];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&vk_sig_sems)
            .swapchains(&swapchains)
            .image_indices(&indices);

        let status = self
            .swapchain
            .enqueue_present(self.device.present_queue(), present_info.build())
            .map_err(|e| {
                if let swapchain::SwapchainError::OutOfDate = e {
                    RenderError::NeedsResize
                } else {
                    RenderError::Swapchain(e)
                }
            })?;

        if let swapchain::SwapchainStatus::SubOptimal = status {
            return Err(RenderError::NeedsResize);
        }

        self.frame_idx = (self.frame_idx + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn render_pass(&self) -> &render_pass::RenderPass {
        &self.render_pass
    }

    pub fn gfx_pipeline(&self) -> &pipeline::GraphicsPipeline {
        &self.gfx_pipeline
    }

    pub fn swapchain_extent(&self) -> util::Extent2D {
        self.swapchain.info().extent
    }

    pub fn framebuffer(&self, frame: &Frame) -> &framebuffer::Framebuffer {
        &self.swapchain_framebuffers[frame.swapchain_image_idx as usize]
    }

    pub fn resize(&mut self, new_extent: util::Extent2D) -> Result<(), RenderError> {
        self.device.wait_idle()?;

        let SwapchainAndCo {
            swapchain,
            swapchain_framebuffers,
            image_to_frame_idx,
            render_pass,
            gfx_pipeline,
        } = create_swapchain_and_co(
            &self.instance,
            &self.device,
            &self.surface,
            &new_extent,
            Some(&self.swapchain),
        )?;

        self.swapchain = swapchain;
        self.swapchain_framebuffers = swapchain_framebuffers;
        self.image_to_frame_idx = image_to_frame_idx;
        self.render_pass = render_pass;
        self.gfx_pipeline = gfx_pipeline;

        Ok(())
    }
}
