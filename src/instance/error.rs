use ash::vk;

use std::ffi::CString;

#[derive(Debug, Clone)]
pub enum InstanceCreationError {
    Creation(vk::Result),
    ExtensionEnumeration(vk::Result),
    MissingExtension(CString),
    LoadError(Vec<&'static str>),
}

impl std::error::Error for InstanceCreationError {}
impl std::fmt::Display for InstanceCreationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<ash::InstanceError> for InstanceCreationError {
    fn from(e: ash::InstanceError) -> Self {
        match e {
            ash::InstanceError::VkError(r) => InstanceCreationError::Creation(r),
            ash::InstanceError::LoadError(v) => InstanceCreationError::LoadError(v),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InstanceError {
    Creation(InstanceCreationError),
}

impl std::error::Error for InstanceError {}
impl std::fmt::Display for InstanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<InstanceCreationError> for InstanceError {
    fn from(e: InstanceCreationError) -> Self {
        Self::Creation(e)
    }
}
