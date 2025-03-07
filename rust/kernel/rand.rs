//! Randomness bindings (TODO longer explanation)

/// TODO
pub fn add_device_randomness(data: &[u8]) {
    // TODO
    unsafe { crate::bindings::add_device_randomness(data.as_ptr() as *const _, data.len()) }
}
