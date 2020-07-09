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

    log::trace!("Chose: {:?}", device);

    let props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };
    log::trace!("Properties:");
    log::trace!("\tvendor_id: {:?}", props.vendor_id);
    log::trace!("\tdevice_id: {:?}", props.device_id);
    log::trace!("\tdevice_type: {:?}", props.device_type);
    log::trace!("\tdevice_name: {:?}", unsafe {
        CStr::from_ptr(props.device_name.as_ptr())
    });
}

fn score_device(instance: &Instance, device: &ash::vk::PhysicalDevice) -> u32 {
    let device_props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };

    let mut score = 0;

    if device_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
        score += 1000;
    }

    let queue_fam_props = unsafe {
        instance
            .vk_instance
            .get_physical_device_queue_family_properties(*device)
    };

    if queue_fam_props
        .iter()
        .any(|fam| fam.queue_flags.contains(vk::QueueFlags::GRAPHICS))
    {
        score += 100;
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

    let device = Device {};

    Ok(device)
}
