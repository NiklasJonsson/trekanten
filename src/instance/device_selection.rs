use super::InitError;
use super::Instance;
use ash::version::InstanceV1_0;
use ash::vk;

pub struct Device {}

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

fn find_graphics_queue_family(
    instance: &Instance,
    device: &ash::vk::PhysicalDevice,
) -> Option<vk::QueueFamilyProperties> {
    let queue_fam_props = unsafe {
        instance
            .vk_instance
            .get_physical_device_queue_family_properties(*device)
    };

    queue_fam_props
        .iter()
        .find(|fam| fam.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .copied()
}

fn score_device(instance: &Instance, device: &vk::PhysicalDevice) -> u32 {
    let device_props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };

    let mut score = 0;

    if device_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
        score += 100;
    }

    if find_graphics_queue_family(instance, device).is_some() {
        score += 1000;
    }

    score
}

pub fn device_selection(instance: &Instance) -> Result<Device, InitError> {
    let mut physical_devices = unsafe { instance.vk_instance.enumerate_physical_devices()? };

    log_physical_devices(instance, &physical_devices);

    if physical_devices.is_empty() {
        return Err(InitError::NoPhysicalDevice);
    }

    // Note that switched args. Higher score should be earlier
    physical_devices.sort_by(|a, b| score_device(instance, b).cmp(&score_device(instance, a)));

    // TODO: Sort the devices and try to choose the best
    let vk_phys_device = physical_devices[0];
    log_choice(instance, &vk_phys_device);

    let queue_fam_props = find_graphics_queue_family(instance, &vk_phys_device)
        .ok_or(InitError::MissingGraphicsQueue)?;

    let device = Device {};

    Ok(device)
}
