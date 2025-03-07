// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! Re-implementation of Qualcomm's Socinfo driver in Rust
use core::convert::From;
use core::fmt;
use core::fmt::{Display, Formatter};
use kernel::bindings::{qcom_pmic_entry, qcom_smem_image_version, socinfo};
use kernel::c_str;
use kernel::debugfs::{Dir, Values};
use kernel::debugfs_fmt_file;
use kernel::module_platform_driver;
use kernel::platform;
use kernel::platform::Device;
use kernel::prelude::*;
use kernel::soc;
use kernel::str::CString;
use kernel::transmute::{AsBytes, FromBytes};

mod bindings;
mod data;

use bindings::qcom_smem_get;
use data::{IMAGE_NAMES, PMIC_MODELS, SOC_IDS};

module_platform_driver! {
    type: QcomSocInfo,
    name: "qcom_socinfo_driver_rust",
    author: "Matthew Maurer",
    description: "Rust re-implementation of Qualcomm's Socinfo driver",
    license: "GPL",
}

#[pin_data]
struct QcomSocInfo {
    #[pin]
    registration: soc::DeviceRegistration,
    #[pin]
    debugfs: Values<Params>,
}

impl QcomSocInfo {
    // This is a workaround for not having pin_project
    fn debugfs(self: Pin<&Self>) -> Pin<&Values<Params>> {
        // SAFETY: self.debugfs is structurally pinned, and we took a Pin<&Self>
        unsafe { Pin::new_unchecked(&Pin::into_inner_unchecked(self).debugfs) }
    }
}

fn soc_info_from_partial_bytes(soc_info_mem: &[u8]) -> socinfo {
    let mut soc_info = socinfo::default();
    let byte_view = soc_info.as_mut_bytes();
    let len = core::cmp::min(soc_info_mem.len(), byte_view.len());
    byte_view[..len].copy_from_slice(&soc_info_mem[..len]);
    soc_info
}

#[derive(Default)]
#[repr(transparent)]
struct PmicArray(&'static [qcom_pmic_entry]);

impl PmicArray {
    fn from_bytes(bytes: &'static [u8], count: usize) -> Option<Self> {
        let size = count * core::mem::size_of::<qcom_pmic_entry>();
        let bytes = &bytes[..size];
        Some(Self(FromBytes::from_bytes(bytes)?))
    }
}

#[derive(Default)]
#[repr(transparent)]
struct ImageVersions(&'static [qcom_smem_image_version]);

impl ImageVersions {
    fn from_bytes(bytes: &'static [u8]) -> Option<Self> {
        Some(Self(FromBytes::from_bytes(bytes)?))
    }
}

#[derive(Default)]
#[repr(transparent)]
struct PmicModel(u32);

impl From<u32> for PmicModel {
    fn from(x: u32) -> Self {
        Self(x)
    }
}

impl Display for PmicModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let model = SocInfo::version_split(self.0).1;
        if let Some(Some(model)) = PMIC_MODELS.get(model as usize) {
            write!(f, "{model}")
        } else {
            write!(f, "unknown ({})", model)
        }
    }
}

#[derive(Default)]
#[repr(transparent)]
struct PmicDieRev(u32);

impl From<u32> for PmicDieRev {
    fn from(x: u32) -> Self {
        Self(x)
    }
}

impl Display for PmicDieRev {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (major, minor) = SocInfo::version_split(self.0);
        write!(f, "{major}.{minor}")
    }
}

impl Display for PmicArray {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (idx, pmic_entry) in self.0.iter().enumerate() {
            let (die_rev_major, die_rev_minor) =
                SocInfo::version_split(u32::from_le(pmic_entry.die_rev));
            let model = pmic_entry.model;
            let model_idx = SocInfo::version_split(model).1 as usize;
            if let Some(Some(model)) = PMIC_MODELS.get(model_idx) {
                write!(f, "{model} {die_rev_major}.{die_rev_minor}")?
            } else {
                write!(f, "unknown ({})", model)?
            }
            if idx + 1 < self.0.len() {
                write!(f, "\n")?;
            }
        }
        Ok(())
    }
}

// Adapter to indicate a u32 should be printed as hex
struct X32(u32);

impl From<u32> for X32 {
    fn from(x: u32) -> Self {
        Self(x)
    }
}

impl Display for X32 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}

#[derive(Default)]
struct Params {
    info_fmt: u32,
    build_id: CString,
    raw_version: Option<u32>,
    hardware_platform: Option<u32>,
    platform_version: Option<u32>,
    accessory_chip: Option<u32>,
    hardware_platform_subtype: Option<u32>,
    pmic_model: Option<PmicModel>,
    pmic_die_rev: Option<PmicDieRev>,
    foundry_id: Option<u32>,
    pmic_model_array: Option<PmicArray>,
    chip_family: Option<X32>,
    raw_device_family: Option<X32>,
    raw_device_number: Option<X32>,
    nproduct_id: Option<u32>,
    chip_id: Option<CString>,
    num_clusters: Option<u32>,
    ncluster_array_offset: Option<u32>,
    num_subset_parts: Option<u32>,
    nsubset_parts_array_offset: Option<u32>,
    nmodem_supported: Option<u32>,
    feature_code: Option<u32>,
    pcode: Option<u32>,
    oem_variant: Option<u32>,
    boot_core: Option<u32>,
    boot_cluster: Option<u32>,
    num_func_clusters: Option<u32>,
    versions: Option<ImageVersions>,
}

#[derive(Copy, Clone)]
struct SocInfo<'a> {
    soc_info: socinfo,
    soc_info_mem: &'a [u8],
    version_mem: &'a [u8],
}

impl<'a> SocInfo<'a> {
    fn from_mem(soc_info_mem: &'a [u8], version_mem: &'a [u8]) -> Self {
        Self {
            soc_info: soc_info_from_partial_bytes(soc_info_mem),
            soc_info_mem,
            version_mem,
        }
    }
    fn id(&self) -> u32 {
        u32::from_le(self.soc_info.id)
    }
    fn version_split(ver: u32) -> (u16, u16) {
        let major = (ver >> 16) as u16;
        let minor = (ver & 0xFFFF) as u16;
        (major, minor)
    }
    fn version_fuse(major: u16, minor: u16) -> u32 {
        ((major as u32) << 16) | minor as u32
    }
    fn version(&self) -> (u16, u16) {
        Self::version_split(self.soc_info.ver)
    }
    fn serial(&self) -> u32 {
        u32::from_le(self.soc_info.id)
    }
    fn machine(&self) -> Result<Option<CString>> {
        for soc in SOC_IDS {
            if soc.id == self.id() {
                return Ok(Some(soc.name.to_cstring()?));
            }
        }
        Ok(None)
    }
    fn device_attribute(&self) -> Result<soc::DeviceAttribute> {
        Ok(soc::DeviceAttribute {
            family: Some(c_str!("Snapdragon").to_cstring()?),
            machine: self.machine()?,
            revision: Some(CString::try_from_fmt(fmt!(
                "{}.{}",
                self.version().0,
                self.version().1
            ))?),
            serial_number: Some(CString::try_from_fmt(fmt!("{}", self.serial()))?),
            soc_id: Some(CString::try_from_fmt(fmt!("{}", self.id()))?),
        })
    }
}

macro_rules! u32_le_versioned {
    { $params:expr, $self:ident,
        [ $( { $major:expr, $minor:expr, { $( $dst:ident: $src:ident ),* } } ),*  ] } => {$(
        if $params.info_fmt >= SocInfo::version_fuse($major, $minor) {
            $( $params.$dst = Some(u32::from_le($self.soc_info.$src).into()) );*
        }
    )*}
}

impl SocInfo<'static> {
    fn build_params(&self) -> Result<Params> {
        let mut params = Params::default();
        params.build_id = CStr::from_bytes_until_nul(&self.soc_info.build_id)?.to_cstring()?;
        params.info_fmt = u32::from_le(self.soc_info.fmt);
        u32_le_versioned! { params, self, [
            {0, 2, { raw_version: raw_ver }},
            {0, 3, { hardware_platform: hw_plat }},
            {0, 4, { platform_version: plat_ver }},
            {0, 5, { accessory_chip: accessory_chip }},
            {0, 6, { hardware_platform_subtype: hw_plat_subtype }},
            {0, 7, { pmic_model: pmic_model, pmic_die_rev: pmic_die_rev }},
            {0, 9, { foundry_id: foundry_id }},
            {0, 12, {
                chip_family: chip_family,
                raw_device_family: raw_device_family,
                raw_device_number: raw_device_num
            }},
            {0, 13, { nproduct_id: nproduct_id }},
            {0, 14, {
                num_clusters: num_clusters,
                ncluster_array_offset: ncluster_array_offset,
                num_subset_parts: num_subset_parts,
                nsubset_parts_array_offset: nsubset_parts_array_offset
            }},
            {0, 15, { nmodem_supported: nmodem_supported }},
            {0, 16, { feature_code: feature_code, pcode: pcode }},
            {0, 17, { oem_variant: oem_variant }},
            {0, 19, {
                boot_core: boot_core,
                boot_cluster: boot_cluster,
                num_func_clusters: num_func_clusters
            }}
        ]};
        if params.info_fmt >= SocInfo::version_fuse(0, 11) {
            let offset = u32::from_le(self.soc_info.pmic_array_offset) as usize;
            let num_pmics = u32::from_le(self.soc_info.num_pmics) as usize;
            params.pmic_model_array =
                PmicArray::from_bytes(&self.soc_info_mem[offset..], num_pmics);
        }
        if params.info_fmt >= SocInfo::version_fuse(0, 13) {
            params.chip_id =
                Some(CStr::from_bytes_until_nul(&self.soc_info.chip_id)?.to_cstring()?);
        }
        params.versions = ImageVersions::from_bytes(self.version_mem);
        Ok(params)
    }
}

macro_rules! value_attrs {
    ($builder:ident, $params:ident, { $($s:ident),* }) => {
        $(
            if let Some(v) = $params.$s.as_ref() {
                $builder.display_file(c_str!(stringify!($s)), v)?;
            }
        )*
    }
}

fn fmt_char_array<const N: usize>(arr: &[u8; N], f: &mut Formatter<'_>) -> fmt::Result {
    if let Ok(cs) = CStr::from_bytes_until_nul(arr) {
        if cs.len() == 0 {
            Ok(())
        } else {
            write!(f, "{}\n", cs)
        }
    } else {
        write!(f, "Unterminated String\n")
    }
}

impl QcomSocInfo {
    fn build_debugfs(self: Pin<&Self>) -> Result<()> {
        self.debugfs().build(|params, dir| {
            debugfs_fmt_file!(
                dir,
                c_str!("info_fmt"),
                &params.info_fmt,
                info_fmt,
                "{info_fmt:#010x}\n"
            )?;
            dir.display_file(c_str!("build_id"), &params.build_id)?;
            value_attrs!(dir, params, {
                raw_version,
                hardware_platform,
                platform_version,
                accessory_chip,
                hardware_platform_subtype,
                raw_device_number,
                raw_device_family,
                chip_family,
                nproduct_id,
                nsubset_parts_array_offset,
                num_subset_parts,
                ncluster_array_offset,
                num_clusters,
                nmodem_supported,
                pcode,
                feature_code,
                oem_variant,
                boot_core,
                boot_cluster,
                num_func_clusters,
                foundry_id,
                chip_id,
                pmic_model,
                pmic_die_rev,
                pmic_model_array
            });
            if let Some(versions) = params.versions.as_ref() {
                for (image_name, idx) in IMAGE_NAMES {
                    if let Some(version) = versions.0.get(*idx) {
                        let subdir = dir.dir(image_name)?;
                        subdir.fmt_file(c_str!("name"), &version.name, &fmt_char_array)?;
                        subdir.fmt_file(c_str!("variant"), &version.variant, &fmt_char_array)?;
                        subdir.fmt_file(c_str!("oem"), &version.oem, &fmt_char_array)?;
                    }
                }
            }
            Ok(())
        })
    }
}

impl platform::Driver for QcomSocInfo {
    type IdInfo = ();
    const OF_ID_TABLE: Option<kernel::of::IdTable<Self::IdInfo>> = None;
    fn probe(_dev: &mut Device, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        let soc_info_mem = qcom_smem_get(
            kernel::bindings::QCOM_SMEM_HOST_ANY,
            kernel::bindings::SMEM_HW_SW_BUILD_ID,
        )?;
        let version_mem = qcom_smem_get(
            kernel::bindings::QCOM_SMEM_HOST_ANY,
            kernel::bindings::SMEM_IMAGE_VERSION_TABLE,
        )?;
        let info = SocInfo::from_mem(soc_info_mem, version_mem);
        let backing = info.build_params()?;
        let debugfs = Dir::new(c_str!("qcom_socinfo_rs"))?;
        let soc_info = KBox::pin_init(
            try_pin_init!(
                    Self {
                        registration <- soc::DeviceRegistration::register(info.device_attribute()?),
                        debugfs <- Values::attach(backing, debugfs),
                    }
            ),
            GFP_KERNEL,
        )?;

        soc_info.as_ref().build_debugfs()?;

        kernel::rand::add_device_randomness(soc_info_mem);

        Ok(soc_info)
    }
}
