use ash::vk;

use ash::version::DeviceV1_0;

use crate::device::AsVkDevice;
use crate::device::Device;
use crate::device::VkDeviceHandle;
use crate::resource::{Handle, Storage};
use crate::uniform::UniformBuffer;

use crate::common::MAX_FRAMES_IN_FLIGHT;

#[derive(Debug)]
pub enum DescriptorSetError {
    Allocation(vk::Result),
}

impl std::error::Error for DescriptorSetError {}
impl std::fmt::Display for DescriptorSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

struct DescriptorPool {
    vk_device: VkDeviceHandle,
    vk_descriptor_pool: vk::DescriptorPool,
    n_allocated: usize,
}

impl std::ops::Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.vk_device
                .destroy_descriptor_pool(self.vk_descriptor_pool, None);
        }
    }
}

impl DescriptorPool {
    fn new(device: &Device) -> Self {
        let descriptor_pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: MAX_FRAMES_IN_FLIGHT as u32,
        };

        let pool_sizes = [descriptor_pool_size];

        let pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

        let vk_descriptor_pool = unsafe {
            device
                .vk_device()
                .create_descriptor_pool(&pool_create_info, None)
                .expect("Failed!")
        };

        Self {
            vk_device: device.vk_device(),
            vk_descriptor_pool,
            n_allocated: 0,
        }
    }

    fn alloc(
        &mut self,
        layout: &vk::DescriptorSetLayout,
        count: usize,
    ) -> Result<Vec<DescriptorSet>, DescriptorSetError> {
        let layouts = vec![*layout; count];
        let info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.vk_descriptor_pool)
            .set_layouts(&layouts);

        let desc_sets: Vec<DescriptorSet> = unsafe {
            self.vk_device
                .allocate_descriptor_sets(&info)
                .map_err(DescriptorSetError::Allocation)?
                .into_iter()
                .map(DescriptorSet::new)
                .collect()
        };

        self.n_allocated += count;

        Ok(desc_sets)
    }
}

// TODO: Rename? (to avoid DescriptorSetDescriptor)
pub struct DescriptorSet {
    vk_descriptor_set: vk::DescriptorSet,
}

impl DescriptorSet {
    fn new(vk_descriptor_set: vk::DescriptorSet) -> Self {
        Self { vk_descriptor_set }
    }

    fn bind_uniform_buffer(&self, vk_device: &VkDeviceHandle, buffer: &UniformBuffer) {
        let buffer_info = vk::DescriptorBufferInfo {
            buffer: *buffer.vk_buffer(),
            offset: 0,
            range: buffer.elem_size() as u64,
        };
        let infos = [buffer_info];

        // TODO: Use the values from the layout
        let write = vk::WriteDescriptorSet::builder()
            .dst_set(self.vk_descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&infos)
            .build();

        let writes = [write];

        unsafe {
            vk_device.update_descriptor_sets(&writes, &[]);
        }
    }

    pub fn vk_descriptor_set(&self) -> &vk::DescriptorSet {
        &self.vk_descriptor_set
    }
}

pub struct DescriptorSetDescriptor<'a> {
    pub layout: vk::DescriptorSetLayout,
    pub uniform_buffers: [&'a UniformBuffer; MAX_FRAMES_IN_FLIGHT],
}

pub struct DescriptorSets {
    vk_device: VkDeviceHandle,
    descriptor_pool: DescriptorPool,
    storage: [Storage<DescriptorSet>; MAX_FRAMES_IN_FLIGHT],
}

impl DescriptorSets {
    pub fn new(device: &Device) -> Self {
        Self {
            vk_device: device.vk_device(),
            descriptor_pool: DescriptorPool::new(device),
            storage: [
                Storage::<DescriptorSet>::new(),
                Storage::<DescriptorSet>::new(),
            ],
        }
    }

    pub fn create<'a>(
        &mut self,
        descriptor: DescriptorSetDescriptor<'a>,
    ) -> Result<Handle<DescriptorSet>, DescriptorSetError> {
        let mut desc_sets = self
            .descriptor_pool
            .alloc(&descriptor.layout, self.storage.len())?;
        let set0 = desc_sets.remove(0);
        let set1 = desc_sets.remove(0);

        for (i, s) in [&set0, &set1].iter().enumerate() {
            s.bind_uniform_buffer(&self.vk_device, descriptor.uniform_buffers[i]);
        }

        let handle0 = self.storage[0].add(set0);
        let _handle1 = self.storage[1].add(set1);

        Ok(handle0)
    }

    pub fn get(&self, h: &Handle<DescriptorSet>, frame_idx: usize) -> Option<&DescriptorSet> {
        self.storage[frame_idx].get(h)
    }
}
