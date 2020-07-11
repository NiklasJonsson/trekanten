use std::ffi::CString;

use crate::instance::device_selection::DeviceSuitability;

#[derive(Debug, Clone)]
pub enum InitError {
    MissingExtension(CString),
    CStrCreation(std::ffi::FromBytesWithNulError),
    VkError(ash::vk::Result),
    VkInstanceLoadError(Vec<&'static str>),
    MissingPhysicalDevice,
    UnsuitableDevice(DeviceSuitability),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::MissingExtension(c_string) => {
                write!(f, "Extension required but not available: {:?}", c_string)
            }
            // TODO: Pretty errors
            e => write!(f, "{:?}", e),
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
