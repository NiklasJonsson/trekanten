use crate::util;

pub trait Window {
    fn required_instance_extensions(&self) -> Vec<String>;
    fn extents(&self) -> util::Extent2D;
    fn aspect_ratio(&self) -> f32 {
        let util::Extent2D { width, height } = self.extents();

        width as f32 / height as f32
    }
}

pub const WINDOW_HEIGHT: u32 = 300;
pub const WINDOW_WIDTH: u32 = 300;
const WINDOW_TITLE: &str = "Trekanten";

pub type GlfwWindowEvents = std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>;

pub struct GlfwWindow {
    pub glfw: glfw::Glfw,
    pub window: glfw::Window,
    pub events: GlfwWindowEvents,
}

impl GlfwWindow {
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

impl Window for GlfwWindow {
    fn required_instance_extensions(&self) -> Vec<String> {
        self.glfw
            .get_required_instance_extensions()
            .expect("Could not get required instance extensions")
    }

    fn extents(&self) -> util::Extent2D {
        let (w, h) = self.window.get_framebuffer_size();
        util::Extent2D {
            width: w as u32,
            height: h as u32,
        }
    }
}

unsafe impl raw_window_handle::HasRawWindowHandle for GlfwWindow {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        self.window.raw_window_handle()
    }
}
