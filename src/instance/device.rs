use super::InitError;
use super::Instance;
use ash::version::DeviceV1_0;
use ash::version::InstanceV1_0;
use ash::vk;

use super::Surface;
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

fn log_device(instance: &Instance, device: &vk::PhysicalDevice) {
    use std::ffi::CStr;

    log::trace!("Vk device: {:?}", device);

    let props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };
    log::trace!("Properties:");
    log::trace!("\tvendor_id: {:?}", props.vendor_id);
    log::trace!("\tdevice_id: {:?}", props.device_id);
    log::trace!("\tdevice_type: {:?}", props.device_type);
    log::trace!("\tdevice_name: {:?}", unsafe {
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
    present: Option<QueueFamily>,
}

impl QueueFamilies {
    pub fn is_complete(&self) -> bool {
        self.graphics.is_some() && self.present.is_some()
    }
}

fn find_queue_families(
    instance: &Instance,
    device: &vk::PhysicalDevice,
    surface: &Surface,
) -> Result<QueueFamilies, InitError> {
    log::trace!("Checking queues for:");
    log_device(instance, device);

    let queue_fam_props = unsafe {
        instance
            .vk_instance
            .get_physical_device_queue_family_properties(*device)
    };

    log::trace!("Found {} queues", queue_fam_props.len());
    for queue in queue_fam_props.iter() {
        log::trace!("{:#?}", queue);
    }

    let mut families = QueueFamilies {
        graphics: None,
        present: None,
    };

    for (i, fam) in queue_fam_props.iter().enumerate() {
        assert!(i <= u32::MAX as usize);
        if fam.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            families.graphics = Some(QueueFamily {
                props: *fam,
                index: i as u32,
            });
        }

        let same_as_gfx = families
            .graphics
            .map(|f| f.index as usize == i)
            .unwrap_or(false);
        // According to vulkan tutorial, "drawing and presentation" is more performant on the same
        // queue
        if surface.is_supported_by(device, i as u32)? && (same_as_gfx || families.present.is_none())
        {
            families.present = Some(QueueFamily {
                props: *fam,
                index: i as u32,
            });
        }
    }

    Ok(families)
}

fn score_device(
    instance: &Instance,
    device: &vk::PhysicalDevice,
    surface: &Surface,
) -> Result<u32, InitError> {
    let device_props = unsafe { instance.vk_instance.get_physical_device_properties(*device) };

    let mut score = 0;

    if device_props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
        score += 100;
    }

    if find_queue_families(instance, device, surface)?.is_complete() {
        score += 1000;
    }

    Ok(score)
}

fn log_queue_family(fam: &QueueFamily) {
    log::trace!("\tindex: {}", fam.index);
    log::trace!("\tflags: {:?}", fam.props.queue_flags);
    log::trace!("\tqueue_count: {}", fam.props.queue_count);
}

fn log_queue_families(qfams: &QueueFamilies) {
    log::trace!("Graphics:");
    qfams
        .graphics
        .map_or_else(|| log::trace!("\tMissing!"), |f| log_queue_family(&f));
    log::trace!("Present:");
    qfams
        .present
        .map_or_else(|| log::trace!("\tMissing!"), |f| log_queue_family(&f));
}

fn create_infos_for_families(
    queue_families: &QueueFamilies,
    prio: &[f32],
) -> Result<Vec<vk::DeviceQueueCreateInfo>, InitError> {
    let (gfx, present) = match (queue_families.graphics, queue_families.present) {
        (None, _) => return Err(InitError::MissingGraphicsQueue),
        (_, None) => return Err(InitError::NoSurfaceSupport),
        (Some(g), Some(p)) => (g, p),
    };

    let infos = if gfx.index == present.index {
        vec![vk::DeviceQueueCreateInfo {
            queue_family_index: gfx.index,
            p_queue_priorities: prio.as_ptr(),
            ..Default::default()
        }]
    } else {
        vec![
            vk::DeviceQueueCreateInfo {
                queue_family_index: gfx.index,
                p_queue_priorities: prio.as_ptr(),
                ..Default::default()
            },
            vk::DeviceQueueCreateInfo {
                queue_family_index: present.index,
                p_queue_priorities: prio.as_ptr(),
                ..Default::default()
            },
        ]
    };

    Ok(infos)
}

pub fn device_selection(instance: &Instance, surface: &Surface) -> Result<Device, InitError> {
    let physical_devices = unsafe { instance.vk_instance.enumerate_physical_devices()? };

    log_physical_devices(instance, &physical_devices);

    if physical_devices.is_empty() {
        return Err(InitError::NoPhysicalDevice);
    }

    // The collect() create a Result<Vec<_>>, using the first Err it finds in the vector. Then ?
    // does an early return if it is Err.
    let mut scored: Vec<(u32, vk::PhysicalDevice)> = physical_devices
        .iter()
        .map(|d| score_device(instance, d, surface).map(|s| (s, *d)))
        .collect::<Result<Vec<_>, InitError>>()?;

    // Note that switched args. Higher score should be earlier
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let vk_phys_device = scored[0].1;
    log::trace!("Choosing device:");
    log_device(instance, &vk_phys_device);

    let queue_families = find_queue_families(instance, &vk_phys_device, surface)?;

    log::trace!("Choosing queue families:");
    log_queue_families(&queue_families);
    let prio = [1.0];

    let queue_infos = create_infos_for_families(&queue_families, &prio)?;

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
