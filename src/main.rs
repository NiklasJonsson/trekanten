use ash::{vk, Entry, version::EntryV1_0};
use glfw::{Action, Context, Key};

use std::ffi::{CString, CStr};

fn handle_window_event(window: &mut glfw::Window, event: glfw::WindowEvent) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
            window.set_should_close(true)
        }
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

        let (mut window, events) = glfw.create_window(WINDOW_WIDTH, WINDOW_HEIGHT, WINDOW_TITLE, glfw::WindowMode::Windowed)
                .expect("Failed to create GLFW window.");

        window.set_key_polling(true);

        Self {
            glfw,
            window,
            events,
        }
    }
}

struct Instance {
    entry: Entry,
    instance: ash::Instance,
}

use ash::extensions::khr::XlibSurface;
use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};


enum InitError {
    MissingExtension(CString),
    CStrCreation(std::ffi::FromBytesWithNulError),
    VkError(ash::vk::Result),
    VkInstanceLoadError(Vec<&'static str>),
}

impl From<std::ffi::FromBytesWithNulError> for InitError {
    fn from(e: std::ffi::FromBytesWithNulError) -> Self {
        Self::CStrCreation(e)
    }
}

impl From<ash::InstanceError> for InitError {
    fn from(e: ash::InstanceError) -> Self {
        match e {
            ash::InstanceError::VkError(r) => InitError::VkError(r),
            ash::InstanceError::LoadError(v) => InitError::VkInstanceLoadError(v),
        }
    }
}


impl From<ash::vk::Result> for InitError {
    fn from(e: ash::vk::Result) -> Self {
        if e == ash::vk::Result::SUCCESS {
            unreachable!("Did not expect success for error!");
        } else {
            Self::VkError(e)
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

fn check_extensions(required: &[CString], available: &[ash::vk::ExtensionProperties]) -> Result<(), InitError> {
    for req in required.iter() {
        let mut found = false;
        for avail in available.iter() {
            let a = unsafe { CStr::from_ptr(avail.extension_name.as_ptr())};
            if a == req.as_c_str() {
                found = true;
            }
        }

        if !found {
            return Err(InitError::MissingExtension(req.clone()));
        }
    }

    return Ok(())
}



impl Instance {
    pub fn new(required_window_extensions: &[CString]) -> Result<Self, InitError> {
        let entry = Entry::new().expect("Failed to create Entry!");

        let available = entry
            .enumerate_instance_extension_properties()?;

        check_extensions(required_window_extensions, &available)?;


        let exts = required_window_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_version(1, 2, 0),
            ..Default::default()
        };


        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(exts.as_slice());

        let instance = unsafe {
            entry.create_instance(&create_info, None)?
        };

        Ok(
            Instance {
                entry,
                instance,
            }
        )
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

    let instance = Instance::new(&raw_exts);

    while !window.window.should_close() {
        window.glfw.poll_events();
        for (_, event) in glfw::flush_messages(&window.events) {
            handle_window_event(&mut window.window, event);
        }
    }
}
