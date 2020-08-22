pub mod cache;
pub mod cached_storage;
pub mod storage;

pub use cached_storage::CachedStorage;
pub use storage::Handle;
pub use storage::Storage;

pub trait ResourceManager<Descriptor, Resource, Error> {
    fn get_resource(&self, handle: &Handle<Resource>) -> Option<&Resource>;
    fn create_resource(&mut self, descriptor: Descriptor) -> Result<Handle<Resource>, Error>;
}
