use ash::vk;

#[derive(Clone, Debug)]
pub struct QueueFamily {
    pub index: u32,
    pub props: vk::QueueFamilyProperties,
}
#[derive(Clone, Debug)]
pub struct QueueFamilies {
    pub graphics: QueueFamily,
    pub present: QueueFamily,
}
