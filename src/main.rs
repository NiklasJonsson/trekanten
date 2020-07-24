use glfw::{Action, Key};

use ash::vk;

mod command;
mod device;
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

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}

pub const WINDOW_HEIGHT: u32 = 300;
pub const WINDOW_WIDTH: u32 = 300;
const WINDOW_TITLE: &str = "Vulkan";

type WindowEvents = std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>;

struct Window {
    glfw: glfw::Glfw,
    window: glfw::Window,
    events: WindowEvents,
}

impl Window {
    pub fn new(mut glfw: glfw::Glfw) -> Self {
        assert!(glfw.vulkan_supported(), "No vulkan!");

        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));

        let (mut window, events) = glfw
            .create_window(
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                WINDOW_TITLE,
                glfw::WindowMode::Windowed,
            )
            .expect("Failed to create GLFW window.");

        window.set_key_polling(true);

        Self {
            glfw,
            window,
            events,
        }
    }
}

#[derive(Debug)]
enum RenderError {
    CommandBuffer(command::CommandBufferError),
}

impl From<command::CommandBufferError> for RenderError {
    fn from(cbe: command::CommandBufferError) -> Self {
        Self::CommandBuffer(cbe)
    }
}

impl std::error::Error for RenderError {}
impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;

fn main() -> Result<(), RenderError> {
    env_logger::init();
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to init glfw");

    let extensions = glfw
        .get_required_instance_extensions()
        .expect("Could not get required instance extensions");

    let instance = instance::Instance::new(&extensions).expect("Instance creation failed!");

    let debug_utils =
        util::vk_debug::DebugUtils::new(&instance).expect("Failed to create DebugUtils");

    let mut window = Window::new(glfw);
    let surface =
        surface::Surface::new(&instance, &window.window).expect("Failed to create surface");

    let device = device::Device::new(&instance, &surface).expect("Failed to create device");

    let swapchain = swapchain::Swapchain::new(&instance, &device, &surface)
        .expect("Failed to create swapchain");

    let render_pass = render_pass::RenderPass::new(&device, swapchain.info().format)
        .expect("Failed to create render pass");

    let g_pipeline = pipeline::GraphicsPipeline::new(
        &device,
        swapchain.info().extent,
        &render_pass,
        "vert.spv",
        "frag.spv",
    )
    .expect("Failed to create graphics pipeline");

    let fbs = swapchain
        .create_framebuffers_for(&render_pass)
        .expect("Failed to create framebuffers");

    let gfx_cmd_pool =
        command::CommandPool::graphics(&device).expect("Failed to create graphics command pool");

    let cmd_buffers = gfx_cmd_pool
        .create_command_buffers(fbs.len() as u32)
        .expect("Failed to create cmd buffers");

    let recorded_cmd_buffers = cmd_buffers
        .into_iter()
        .zip(fbs.iter())
        .map(
            |(cmd_buffer, fb)| -> Result<command::CommandBuffer, command::CommandBufferError> {
                Ok(cmd_buffer
                    .begin()?
                    .begin_render_pass(&render_pass, &fb, swapchain.info().extent)
                    .bind_gfx_pipeline(&g_pipeline)
                    .draw(3, 1, 0, 0)
                    .end_render_pass()
                    .end()?)
            },
        )
        .collect::<Result<Vec<_>, command::CommandBufferError>>()?;

    let image_avail_sem = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| sync::Semaphore::new(&device).expect("Failed to create semaphore"))
        .collect::<Vec<_>>();
    let render_done_sem = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| sync::Semaphore::new(&device).expect("Failed to create semaphore"))
        .collect::<Vec<_>>();

    let in_flight_fences = (0..MAX_FRAMES_IN_FLIGHT)
        .map(|_| sync::Fence::new(&device).expect("Failed to create fence"))
        .collect::<Vec<_>>();
    let mut images_in_flight_fences: Vec<Option<usize>> = (0..fbs.len()).map(|_| None).collect();

    let mut current_frame = 0;

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }

        in_flight_fences[current_frame]
            .blocking_wait()
            .expect("Failed to wait for fence");

        let img_idx = swapchain
            .acquire_next_image(Some(&image_avail_sem[current_frame]))
            .expect("Failed to get swapchain image index");

        if let Some(fence_idx) = images_in_flight_fences[img_idx as usize] {
            in_flight_fences[fence_idx]
                .blocking_wait()
                .expect("Failed to wait");
        }

        images_in_flight_fences[img_idx as usize] = Some(current_frame);

        let gfx_queue = device.graphics_queue();

        // TODO: Get rid of the allocations below
        let vk_wait_sems = [*image_avail_sem[current_frame].vk_semaphore()];

        let wait_dst_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let vk_sig_sems = [*render_done_sem[current_frame].vk_semaphore()];

        let r_cmd_buffers = recorded_cmd_buffers
            .iter()
            .map(|cb| [*cb.vk_command_buffer()])
            .collect::<Vec<_>>();

        // TODO: Move this out of here
        let info = vk::SubmitInfo::builder()
            .wait_semaphores(&vk_wait_sems)
            .wait_dst_stage_mask(&wait_dst_mask)
            .signal_semaphores(&vk_sig_sems)
            .command_buffers(&r_cmd_buffers[img_idx as usize]);

        in_flight_fences[current_frame]
            .reset()
            .expect("Failed to reset fence");
        gfx_queue
            .submit(&info, &in_flight_fences[current_frame])
            .expect("Failed to submit to queue");

        let swapchains = [*swapchain.vk_swapchain()];
        let indices = [img_idx];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&vk_sig_sems)
            .swapchains(&swapchains)
            .image_indices(&indices);

        swapchain
            .enqueue_present(device.present_queue(), present_info.build())
            .expect("Failed to submit present");

        current_frame = (current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    device.wait_idle().expect("Failed to wait");

    Ok(())
}
