#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Extent2D {
    pub width: u32,
    pub height: u32,
}

impl From<ash::vk::Extent2D> for Extent2D {
    fn from(e: ash::vk::Extent2D) -> Self {
        Self {
            width: e.width,
            height: e.height,
        }
    }
}

impl From<Extent2D> for ash::vk::Extent2D {
    fn from(e: Extent2D) -> Self {
        Self {
            width: e.width,
            height: e.height,
        }
    }
}
