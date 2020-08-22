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

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Extent3D {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

impl Extent3D {
    pub fn from_2d(extent: Extent2D, depth: u32) -> Self {
        Self {
            width: extent.width,
            height: extent.height,
            depth,
        }
    }
}

impl From<ash::vk::Extent3D> for Extent3D {
    fn from(e: ash::vk::Extent3D) -> Self {
        Self {
            width: e.width,
            height: e.height,
            depth: e.depth,
        }
    }
}

impl From<Extent3D> for ash::vk::Extent3D {
    fn from(e: Extent3D) -> Self {
        Self {
            width: e.width,
            height: e.height,
            depth: e.depth,
        }
    }
}
