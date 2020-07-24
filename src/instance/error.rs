use crate::surface::SurfaceError;

#[derive(Debug, Clone)]
pub enum InitError {
    CStrCreation(std::ffi::FromBytesWithNulError),
    VkError(ash::vk::Result),
    VkInstanceLoadError(Vec<&'static str>),
    Surface(SurfaceError),
}

impl std::error::Error for InitError {}
impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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

impl From<SurfaceError> for InitError {
    fn from(e: SurfaceError) -> Self {
        Self::Surface(e)
    }
}
