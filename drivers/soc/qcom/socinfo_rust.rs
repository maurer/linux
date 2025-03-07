//! Re-implementation of Qualcomm's Socinfo driver in Rust
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

struct QcomSocInfo;

impl platform::Driver for QcomSocInfo {
    type IdInfo = ();
    const OF_ID_TABLE: Option<kernel::of::IdTable<Self::IdInfo>> = None;
    fn probe(_dev: &mut Device, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        Ok(KBox::pin(Self, GFP_KERNEL)?)
    }
}


