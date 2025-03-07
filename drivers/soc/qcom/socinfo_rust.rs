//! Re-implementation of Qualcomm's Socinfo driver in Rust
use kernel::error::from_err_ptr;
use kernel::prelude::*;
use kernel::platform;
use kernel::platform::Device;
use kernel::module_platform_driver;

module_platform_driver! {
    type: QcomSocInfo,
    name: "qcom_socinfo_driver_rust",
    author: "Matthew Maurer",
    description: "Rust re-implementation of Qualcomm's Socinfo driver",
    license: "GPL",
}

// TODO module bindgen
mod ffi {
    extern "C" {
        pub(crate) fn qcom_smem_get(host: kernel::ffi::c_uint, item: kernel::ffi::c_uint, size: *mut usize) -> *mut kernel::ffi::c_void;
    }
    pub(crate) const QCOM_SMEM_HOST_ANY: kernel::ffi::c_uint = kernel::ffi::c_uint::MAX;
    pub(crate) const SMEM_HW_SW_BUILD_ID: kernel::ffi::c_uint = 137;
}

fn qcom_smem_get(host: u32, item: u32) -> Result<&'static [u8]> {
    let mut size = 0;
    // SAFETY: TODO
    let ptr = from_err_ptr(unsafe { ffi::qcom_smem_get(host as _, item as _, &mut size) })?;
    // SAFETY: TODO
    Ok(unsafe { core::slice::from_raw_parts(ptr as *const _, size) })
}

struct QcomSocInfo;

impl platform::Driver for QcomSocInfo {
    type IdInfo = ();
    const OF_ID_TABLE: Option<kernel::of::IdTable<Self::IdInfo>> = None;
    fn probe(_dev: &mut Device, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        let _mem = qcom_smem_get(ffi::QCOM_SMEM_HOST_ANY, ffi::SMEM_HW_SW_BUILD_ID)?;
        Ok(KBox::pin(Self, GFP_KERNEL)?)
    }
}
