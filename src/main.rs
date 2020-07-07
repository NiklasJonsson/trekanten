use glfw::{Action, Context, Key};

use std::ffi::CString;

mod instance;

use instance::{InitError, Instance};

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        _ => {}
    }
}

const WINDOW_HEIGHT: u32 = 300;
const WINDOW_WIDTH: u32 = 300;
const WINDOW_TITLE: &str = "Vulkan";

type WindowEvents = std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>;

struct Window {
    glfw: glfw::Glfw,
    window: glfw::Window,
    events: WindowEvents,
}

impl Window {
    pub fn new() -> Self {
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("Failed to init glfw");

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
    let mut window = Window::new();

    let extensions = window
        .glfw
        .get_required_instance_extensions()
        .expect("Could not get required instance extensions");

    let raw_exts = extensions
        .iter()
        .map(|x| CString::new(x.as_str()).unwrap())
        .collect::<Vec<_>>();

    let instance = Instance::new(&raw_exts).expect("Instance creation failed!");

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }
    }
}
