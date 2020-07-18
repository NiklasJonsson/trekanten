use glfw::{Action, Key};

use instance::{InitError, Instance};

mod device;
mod framebuffer;
mod image;
mod instance;
mod pipeline;
mod queue;
mod render_pass;
mod surface;
mod swapchain;
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

enum RenderError {
    InitError(InitError),
}

impl From<InitError> for RenderError {
    fn from(e: InitError) -> Self {
        Self::InitError(e)
    }
}

fn main() {
    env_logger::init();
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to init glfw");

    let extensions = glfw
        .get_required_instance_extensions()
        .expect("Could not get required instance extensions");

    let instance = Instance::new(&extensions).expect("Instance creation failed!");

    let debug_utils =
        util::vk_debug::DebugUtils::new(&instance).expect("Failed to create DebugUtils");

    let mut window = Window::new(glfw);
    let surface = instance
        .create_surface(&window.window)
        .expect("Unable to create surface");

    let device = instance
        .create_device(&surface)
        .expect("Unable to create device");

    // TODO: Move this function to instance?
    // It is techically a child of the device...
    // Maybe the device should have a reference to instance?
    // Should the device "consume" the instance?
    let swapchain = device
        .create_swapchain(&instance, &surface)
        .expect("Unable to create swapchain");

    let render_pass = render_pass::RenderPass::new(&device, swapchain.info().format)
        .expect("Unable to create render pass");

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

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }
    }
}
