use ash::version::InstanceV1_0; // For destroy_instance
use ash::{version::EntryV1_0, vk, Entry};
use std::ffi::{CStr, CString};

pub struct Instance {
    _entry: Entry,
    instance: ash::Instance,
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitError {
    MissingExtension(CString),
    CStrCreation(std::ffi::FromBytesWithNulError),
    VkError(ash::vk::Result),
    VkInstanceLoadError(Vec<&'static str>),
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

fn check_extensions(
    required: &[CString],
    available: &[ash::vk::ExtensionProperties],
) -> Result<(), InitError> {
    for req in required.iter() {
        let mut found = false;
        for avail in available.iter() {
            let a = unsafe { CStr::from_ptr(avail.extension_name.as_ptr()) };
            if a == req.as_c_str() {
                found = true;
            }
        }

        if !found {
            return Err(InitError::MissingExtension(req.clone()));
        }
    }

    Ok(())
}

const DISABLE_VALIDATION_LAYERS_ENV_VAR: &str = "TREK_DISABLE_VALIDATION_LAYERS";

fn validation_layers() -> Vec<CString> {
    vec![CString::new("VK_LAYER_KHRONOS_validation").expect("Failed to create CString")]
}

fn choose_validation_layers(entry: &Entry) -> Vec<CString> {
    if std::env::var(DISABLE_VALIDATION_LAYERS_ENV_VAR).is_err() {
        let requested = validation_layers();

        let layers = match entry.enumerate_instance_layer_properties() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        for req in requested.iter() {
            let mut found = false;
            for layer in layers.iter() {
                let l = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                if l == req.as_c_str() {
                    found = true;
                }
            }

            if !found {
                return Vec::new();
            }
        }

        requested
    } else {
        Vec::new()
    }
}

impl Instance {
    pub fn new(required_window_extensions: &[CString]) -> Result<Self, InitError> {
        let entry = Entry::new().expect("Failed to create Entry!");

        let available = entry.enumerate_instance_extension_properties()?;

        check_extensions(required_window_extensions, &available)?;

        let exts = required_window_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let app_info = vk::ApplicationInfo {
            api_version: vk::make_version(1, 2, 0),
            ..Default::default()
        };

        let validation_layers = choose_validation_layers(&entry);

        let layers_ptrs = validation_layers
            .into_iter()
            .map(|x| x.into_raw() as *const i8)
            .collect::<Vec<_>>();

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(exts.as_slice())
            .enabled_layer_names(layers_ptrs.as_slice());

        let _layers_owned = layers_ptrs
            .iter()
            .map(|x| unsafe { CString::from_raw(*x as *mut i8) })
            .collect::<Vec<_>>();

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        Ok(Instance {
            _entry: entry,
            instance,
        })
    }
}
