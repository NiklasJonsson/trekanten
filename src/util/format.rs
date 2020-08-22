use ash::vk;

// TODO: We might be mixing up format/color space/layout here
// Vulkan seems to have a format as layout + SRGB and then color space on top of that...
// ColorSpaceKHR::SRGB_NONLINEAR

/// First channel is the lowest address, last is the highest (same as the vulkan spec).
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum ComponentLayout {
    R8G8B8A8,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    Linear,
    Srgb,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
pub struct Format {
    pub component_layout: ComponentLayout,
    pub color_space: ColorSpace,
}

impl From<Format> for vk::Format {
    fn from(f: Format) -> vk::Format {
        match (f.component_layout, f.color_space) {
            (ComponentLayout::R8G8B8A8, ColorSpace::Srgb) => vk::Format::R8G8B8A8_SRGB,
            _ => unimplemented!(),
            //(ComponentLayout::R8G8B8A8, ColorSpace::Linear) => vk::Format::R8G8B8A8_UNORM,
        }
    }
}

impl From<vk::Format> for Format {
    fn from(f: vk::Format) -> Self {
        match f {
            vk::Format::R8G8B8A8_SRGB => Self {
                component_layout: ComponentLayout::R8G8B8A8,
                color_space: ColorSpace::Srgb,
            },
            /*
            vk::Format::R8G8B8A8_UNORM => Self {
                component_layout: ComponentLayout::R8G8B8A8,
                color_space: ColorSpace::Linear,
            },
            */
            _ => unimplemented!(),
        }
    }
}
