//! Soc bindings

use crate::bindings;
use crate::error;
use crate::prelude::*;
use crate::str::CString;
use core::marker::PhantomPinned;
use core::ptr::addr_of;

/// Attributes for a soc device
pub struct DeviceAttribute {
    /// Machine
    pub machine: Option<CString>,
    /// Family
    pub family: Option<CString>,
    /// Revision
    pub revision: Option<CString>,
    /// Serial Number
    pub serial_number: Option<CString>,
    /// SOC ID
    pub soc_id: Option<CString>,
}

#[pin_data]
struct BuiltDeviceAttribute {
    backing: DeviceAttribute,
    #[pin]
    inner: bindings::soc_device_attribute,
}

fn cstring_to_c<'a>(mcs: &Option<CString>) -> *const kernel::ffi::c_char {
    mcs.as_ref()
        .map(|cs| cs.as_char_ptr())
        .unwrap_or(core::ptr::null())
}

impl BuiltDeviceAttribute {
    fn as_mut_ptr(&self) -> *mut bindings::soc_device_attribute {
        &self.inner as *const _ as *mut _
    }
}

impl DeviceAttribute {
    // TODO custom_attr_group?
    fn build(self) -> impl init::PinInit<BuiltDeviceAttribute> {
        pin_init!(BuiltDeviceAttribute {
            inner: bindings::soc_device_attribute {
                machine: cstring_to_c(&self.machine),
                family: cstring_to_c(&self.family),
                revision: cstring_to_c(&self.revision),
                serial_number: cstring_to_c(&self.serial_number),
                soc_id: cstring_to_c(&self.soc_id),
                data: core::ptr::null(),
                custom_attr_group: core::ptr::null(),
            },
            backing: self,
        })
    }
}

// TODO safety
unsafe impl Sync for Device {}

// TODO safety
unsafe impl Send for Device {}

/// A registered soc device
#[repr(transparent)]
pub struct Device {
    inner: *mut bindings::soc_device,
}

impl Device {
    // Intentionally private
    // TODO safety
    unsafe fn register(attr: *const BuiltDeviceAttribute) -> Result<Device> {
        let raw_soc =
            error::from_err_ptr(unsafe { bindings::soc_device_register((*attr).as_mut_ptr()) })?;
        Ok(Device { inner: raw_soc })
    }
}

#[pin_data(PinnedDrop)]
/// Registration handle for your soc_dev. If you let it go out of scope, your soc_dev will be
/// unregistered.
pub struct DeviceRegistration {
    #[pin]
    attr: BuiltDeviceAttribute,
    soc_dev: Device,
    #[pin]
    _pin: PhantomPinned,
}

#[pinned_drop]
impl PinnedDrop for DeviceRegistration {
    fn drop(self: Pin<&mut Self>) {
        // TODO
        unsafe { bindings::soc_device_unregister(self.soc_dev.inner) }
    }
}

impl DeviceRegistration {
    /// Register a new socdevice
    pub fn register(attr: DeviceAttribute) -> impl init::PinInit<Self, Error> {
        // TODO safety
        try_pin_init!(&this in Self {
                    attr <- attr.build(),
                    // TODO Safety
                    soc_dev: unsafe { Device::register(addr_of!((*this.as_ptr()).attr))? },
                    _pin: PhantomPinned,
            }? Error)
    }
}
