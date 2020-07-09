use ash::extensions::ext;
use ash::version::InstanceV1_0; // For destroy_instance
use ash::{version::EntryV1_0, vk, Entry};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub mod device_selection;

pub use device_selection::device_selection;

pub struct Instance {
    entry: Entry,
    vk_instance: ash::Instance,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.vk_instance.destroy_instance(None);
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitError {
    MissingExtension(CString),
    CStrCreation(std::ffi::FromBytesWithNulError),
    VkError(ash::vk::Result),
    VkInstanceLoadError(Vec<&'static str>),
    NoPhysicalDevice,
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::MissingExtension(c_string) => {
                write!(f, "Extension required but not available: {:?}", c_string)
            }
            _ => unimplemented!(),
        }
    }
}

impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            InitError::CStrCreation(e) => Some(e),
            _ => None,
        }
    }
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

fn check_extensions<T: AsRef<CStr>>(
    required: &[T],
    available: &[ash::vk::ExtensionProperties],
) -> Result<(), InitError> {
    for req in required.iter() {
        let mut found = false;
        for avail in available.iter() {
            let a = unsafe { CStr::from_ptr(avail.extension_name.as_ptr()) };
            log::trace!("Available vk instance extension: {:?}", avail);
            if a == req.as_ref() {
                found = true;
            }
        }

        if !found {
            let c_string: CString = req.as_ref().to_owned();
            return Err(InitError::MissingExtension(c_string));
        }
    }

    Ok(())
}

const DISABLE_VALIDATION_LAYERS_ENV_VAR: &str = "TREK_DISABLE_VALIDATION_LAYERS";

fn validation_layers() -> Vec<CString> {
    vec![CString::new("VK_LAYER_KHRONOS_validation").expect("Failed to create CString")]
}

fn log_cstrings(a: &[CString]) {
    for cs in a {
        log::trace!("{:?}", cs);
    }
}

fn use_vk_validation() -> bool {
    std::env::var(DISABLE_VALIDATION_LAYERS_ENV_VAR).is_err()
}

fn choose_validation_layers(entry: &Entry) -> Vec<CString> {
    if use_vk_validation() {
        let requested = validation_layers();
        log::trace!("Requested vk layers:");
        log_cstrings(&requested);

        let layers = match entry.enumerate_instance_layer_properties() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        for req in requested.iter() {
            let mut found = false;
            for layer in layers.iter() {
                let l = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                log::trace!("Found vk layer: {:?}", layer);
                if l == req.as_c_str() {
                    found = true;
                }
            }

            if !found {
                return Vec::new();
            }
        }

        log::trace!("Choosing layers:");
        log_cstrings(&requested);
        requested
    } else {
        Vec::new()
    }
}

fn choose_instance_extensions<T: AsRef<str>>(
    entry: &Entry,
    required_window_extensions: &[T],
) -> Result<Vec<CString>, InitError> {
    let available = entry.enumerate_instance_extension_properties()?;
    let required = required_window_extensions
        .iter()
        .map(|x| CString::new(x.as_ref()).expect("CString failed!"))
        .collect::<Vec<CString>>();

    check_extensions(&required, &available)?;
    let mut instance_extensions = required.to_vec();

    if use_vk_validation() {
        instance_extensions.push(ext::DebugUtils::name().to_owned());
    }

    Ok(instance_extensions)
}

/// This will leak memory if vec_ptrs_to_cstring is not called
fn vec_cstring_to_raw(v: Vec<CString>) -> Vec<*const c_char> {
    v.into_iter()
        .map(|x| x.into_raw() as *const c_char)
        .collect::<Vec<_>>()
}

/// Call this to reclaim memory of the vec of c_chars
fn vec_cstring_from_raw(v: Vec<*const c_char>) -> Vec<CString> {
    v.iter()
        .map(|x| unsafe { CString::from_raw(*x as *mut c_char) })
        .collect::<Vec<_>>()
}

impl Instance {
    pub fn new<T: AsRef<str>>(required_window_extensions: &[T]) -> Result<Self, InitError> {
        let entry = Entry::new().expect("Failed to create Entry!");

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_version(1, 2, 0),
            ..Default::default()
        };

        let extensions = choose_instance_extensions(&entry, required_window_extensions)?;
        let extensions_ptrs = vec_cstring_to_raw(extensions);

        let validation_layers = choose_validation_layers(&entry);
        let layers_ptrs = vec_cstring_to_raw(validation_layers);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions_ptrs)
            .enabled_layer_names(&layers_ptrs);

        let vk_instance = unsafe { entry.create_instance(&create_info, None)? };

        let _owned_layers = vec_cstring_from_raw(layers_ptrs);
        let _owned_extensions = vec_cstring_from_raw(extensions_ptrs);

        let instance = Instance { entry, vk_instance };

        // TODO: Setup debug callbacks to log

        Ok(instance)
    }
}
