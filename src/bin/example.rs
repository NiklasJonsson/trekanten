use glfw::{Action, Key};

use trekanten::*;

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

pub struct Window {
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

fn main() -> Result<(), RenderError> {
    env_logger::init();
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to init glfw");

    let extensions = glfw
        .get_required_instance_extensions()
        .expect("Could not get required instance extensions");

    let mut window = Window::new(glfw);
    let mut renderer = trekanten::Renderer::new(&extensions, &window.window)?;

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }

        let mut frame = renderer.next_frame()?;

        let render_pass = renderer.render_pass();
        let gfx_pipeline = renderer.gfx_pipeline();
        let extent = renderer.swapchain_extent();
        let framebuffer = renderer.framebuffer(&frame);

        let cmd_buf = frame
            .new_command_buffer()?
            .begin()?
            .begin_render_pass(render_pass, framebuffer, extent)
            .bind_gfx_pipeline(&gfx_pipeline)
            .draw(3, 1, 0, 0)
            .end_render_pass()
            .end()?;

        frame.add_command_buffer(cmd_buf);

        renderer.submit(frame)?;
    }

    Ok(())
}
