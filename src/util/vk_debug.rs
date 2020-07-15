use ash::extensions::ext;
use ash::version::InstanceV1_0;
use ash::vk;

use std::ffi::CStr;
use std::fmt::Write;
use std::os::raw::c_char;

use crate::instance::InitError;
use crate::instance::Instance;
use crate::util::lifetime::LifetimeToken;

pub struct DebugUtils {
    loader: ext::DebugUtils,
    callback_handle: vk::DebugUtilsMessengerEXT,
    _parent_lifetime_token: LifetimeToken<Instance>,
}

impl Drop for DebugUtils {
    fn drop(&mut self) {
        unsafe {
            self.loader
                .destroy_debug_utils_messenger(self.callback_handle, None);
        }
    }
}

impl DebugUtils {
    pub fn new(instance: &Instance) -> Result<Self, InitError> {
        let loader = ext::DebugUtils::new(instance.entry(), instance.inner_vk_instance());

        let info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
            .message_type(vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(vk_debug_callback));

        let callback_handle = unsafe { loader.create_debug_utils_messenger(&info, None) }?;

        Ok(Self {
            loader,
            callback_handle,
            _parent_lifetime_token: instance.lifetime_token(),
        })
    }
}

const NULL_AS_STR: &str = "NULL";

unsafe fn write_maybe_null(mut s: &mut String, p: *const c_char) {
    if p.is_null() {
        write!(&mut s, "({})", "NULL").expect("vk_debug_callback failed to write");
    } else {
        write!(&mut s, "({:?})", CStr::from_ptr(p)).expect("vk_debug_callback failed to write");
    }
}

unsafe extern "system" fn vk_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use vk::DebugUtilsMessageSeverityFlagsEXT as Severity;
    use vk::DebugUtilsMessageTypeFlagsEXT as Type;

    let callback_data = *p_callback_data;
    let p_null_str = NULL_AS_STR.as_ptr() as *const c_char;

    let mut message = String::new();

    write!(&mut message, "[{:?}]", message_type).expect("vk_debug_callback failed to write");
    write!(&mut message, "[ID {}", callback_data.message_id_number)
        .expect("vk_debug_callback failed to write");

    write_maybe_null(&mut message, callback_data.p_message_id_name);
    write!(&mut message, "]\n").expect("vk_debug_callback failed to write");

    write_maybe_null(&mut message, callback_data.p_message);

    if message_severity.contains(Severity::VERBOSE) {
        log::trace!("{}", message);
    }

    if message_severity.contains(Severity::INFO) {
        log::info!("{}", message);
    }

    if message_severity.contains(Severity::WARNING) {
        log::warn!("{}", message);
    }

    if message_severity.contains(Severity::ERROR) {
        log::error!("{}", message);
    }

    // According to the lunarg tutorial for the callback, false => don't bail out
    0
}
