pub mod extent;
pub mod ffi;
pub mod lifetime;
pub mod vk_debug;

pub use extent::*;

pub fn clamp<T: Ord>(v: T, min: T, max: T) -> T {
    std::cmp::max(min, std::cmp::min(v, max))
}
