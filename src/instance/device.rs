use super::InitError;
use super::Instance;
use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use ash::vk;

use crate::util::LifetimeToken;

pub struct Device {
    vk_device: ash::Device,
    _parent_lifetime_token: LifetimeToken<super::Instance>,
}

impl std::ops::Drop for Device {
    fn drop(&mut self) {
        unsafe { self.vk_device.destroy_device(None) };
    }
}

fn log_physical_devices(instance: &Instance, devices: &[ash::vk::PhysicalDevice]) {
    for device in devices.iter() {
        log::trace!("Found device: {:?}", device);
        let props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };
        log::trace!("Properties: {:#?}", props);
    }
}

fn log_choice(instance: &Instance, device: &ash::vk::PhysicalDevice) {
    use std::ffi::CStr;

    log::info!("Chose vk device: {:?}", device);

    let props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };
    log::info!("Properties:");
    log::info!("\tvendor_id: {:?}", props.vendor_id);
    log::info!("\tdevice_id: {:?}", props.device_id);
    log::info!("\tdevice_type: {:?}", props.device_type);
    log::info!("\tdevice_name: {:?}", unsafe {
        CStr::from_ptr(props.device_name.as_ptr())
    });
}

#[derive(Copy, Clone, Debug)]
struct QueueFamily {
    index: u32,
    props: vk::QueueFamilyProperties,
}

#[derive(Copy, Clone, Debug)]
struct QueueFamilies {
    graphics: Option<QueueFamily>,
}

fn find_queue_families(instance: &Instance, device: &vk::PhysicalDevice) -> QueueFamilies {
    let queue_fam_props = unsafe {
        instance
            .vk_instance
            .get_physical_device_queue_family_properties(*device)
    };

    log::trace!("Found {} queues", queue_fam_props.len());

    let filtered_fams = queue_fam_props
        .iter()
        .enumerate()
        .map(|(i, fam)| {
            if fam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                assert!(i < u32::MAX as usize);
                Some(QueueFamily {
                    props: *fam,
                    index: i as u32,
                })
            } else {
                None
            }
        })
        .filter_map(|x| x)
        .collect::<Vec<_>>();

    QueueFamilies {
        graphics: filtered_fams.first().copied(),
    }
}

fn score_device(instance: &Instance, device: &vk::PhysicalDevice) -> u32 {
    let device_props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };

    let mut score = 0;

    if device_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
        score += 100;
    }

    if find_queue_families(instance, device).graphics.is_some() {
        score += 1000;
    }

    score
}

fn log_queue_fam(qfam: &vk::QueueFamilyProperties) {
    log::info!("Choosing queue:");
    log::info!("\tflags: {:?}", qfam.queue_flags);
    log::info!("\tqueue_count: {}", qfam.queue_count);
    log::trace!("{:?}", qfam);
}

pub fn device_selection(instance: &Instance) -> Result<Device, InitError> {
    let mut physical_devices = unsafe { instance.vk_instance.enumerate_physical_devices()? };

    log_physical_devices(instance, &physical_devices);

    if physical_devices.is_empty() {
        return Err(InitError::NoPhysicalDevice);
    }

    // Note that switched args. Higher score should be earlier
    physical_devices.sort_by(|a, b| score_device(instance, b).cmp(&score_device(instance, a)));

    let vk_phys_device = physical_devices[0];
    log_choice(instance, &vk_phys_device);

    let queue_families = find_queue_families(instance, &vk_phys_device);

    let graphics_fam = queue_families
        .graphics
        .ok_or(InitError::MissingGraphicsQueue)?;
    log_queue_fam(&graphics_fam.props);

    let queue_info = vk::DeviceQueueCreateInfo::builder()
        .queue_family_index(graphics_fam.index)
        .queue_priorities(&[1.0])
        .build();

    let queue_infos = [queue_info];

    // TODO: Cleanup handling layers together with instance
    let validation_layers = super::choose_validation_layers(&instance.entry);
    let layers_ptrs = super::vec_cstring_to_raw(validation_layers);

    let device_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_layer_names(&layers_ptrs);

    let vk_device = unsafe {
        instance
            .vk_instance
            .create_device(vk_phys_device, &device_info, None)
    }?;

    let _owned_layers = super::vec_cstring_from_raw(layers_ptrs);
    let device = Device {
        vk_device,
        _parent_lifetime_token: instance.lifetime_token(),
    };

    Ok(device)
}
