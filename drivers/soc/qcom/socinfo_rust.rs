//! Re-implementation of Qualcomm's Socinfo driver in Rust
use kernel::c_str;
use kernel::debugfs::Printer;
use kernel::debugfs::X32;
use kernel::debugfs::{DebugfsDir, DebugfsValues};
use kernel::error::from_err_ptr;
use kernel::module_platform_driver;
use kernel::platform;
use kernel::platform::Device;
use kernel::prelude::*;
use kernel::seq_file::SeqFile;
use kernel::seq_print;
use kernel::soc;
use kernel::str::CString;

module_platform_driver! {
    type: QcomSocInfo,
    name: "qcom_socinfo_driver_rust",
    author: "Matthew Maurer",
    description: "Rust re-implementation of Qualcomm's Socinfo driver",
    license: "GPL",
}

// TODO simplify
macro_rules! id_entry {
    ($id:ident) => {
        kernel::macros::paste! {
            SocId {
                id: kernel::bindings::[<QCOM_ID_ $id>],
                name: c_str!(stringify!($id)),
            }
        }
    };
    ($id:ident, $name:tt) => {
        SocId {
            id: kernel::macros::paste!(kernel::bindings::[<QCOM_ID_ $id>]), name: c_str!($name),
        }
    }
}

struct SocId {
    id: u32,
    name: &'static CStr,
}

static SOC_IDS: &[SocId] = &[
    { id_entry!(MSM8260) },
    { id_entry!(MSM8660) },
    { id_entry!(APQ8060) },
    { id_entry!(MSM8960) },
    { id_entry!(APQ8064) },
    { id_entry!(MSM8930) },
    { id_entry!(MSM8630) },
    { id_entry!(MSM8230) },
    { id_entry!(APQ8030) },
    { id_entry!(MSM8627) },
    { id_entry!(MSM8227) },
    { id_entry!(MSM8660A) },
    { id_entry!(MSM8260A) },
    { id_entry!(APQ8060A) },
    { id_entry!(MSM8974) },
    { id_entry!(MSM8225) },
    { id_entry!(MSM8625) },
    { id_entry!(MPQ8064) },
    { id_entry!(MSM8960AB) },
    { id_entry!(APQ8060AB) },
    { id_entry!(MSM8260AB) },
    { id_entry!(MSM8660AB) },
    { id_entry!(MSM8930AA) },
    { id_entry!(MSM8630AA) },
    { id_entry!(MSM8230AA) },
    { id_entry!(MSM8626) },
    { id_entry!(MSM8610) },
    { id_entry!(APQ8064AB) },
    { id_entry!(MSM8930AB) },
    { id_entry!(MSM8630AB) },
    { id_entry!(MSM8230AB) },
    { id_entry!(APQ8030AB) },
    { id_entry!(MSM8226) },
    { id_entry!(MSM8526) },
    { id_entry!(APQ8030AA) },
    { id_entry!(MSM8110) },
    { id_entry!(MSM8210) },
    { id_entry!(MSM8810) },
    { id_entry!(MSM8212) },
    { id_entry!(MSM8612) },
    { id_entry!(MSM8112) },
    { id_entry!(MSM8125) },
    { id_entry!(MSM8225Q) },
    { id_entry!(MSM8625Q) },
    { id_entry!(MSM8125Q) },
    { id_entry!(APQ8064AA) },
    { id_entry!(APQ8084) },
    { id_entry!(MSM8130) },
    { id_entry!(MSM8130AA) },
    { id_entry!(MSM8130AB) },
    { id_entry!(MSM8627AA) },
    { id_entry!(MSM8227AA) },
    { id_entry!(APQ8074) },
    { id_entry!(MSM8274) },
    { id_entry!(MSM8674) },
    { id_entry!(MDM9635) },
    { id_entry!(MSM8974PRO_AC, "MSM8974PRO-AC") },
    { id_entry!(MSM8126) },
    { id_entry!(APQ8026) },
    { id_entry!(MSM8926) },
    { id_entry!(IPQ8062) },
    { id_entry!(IPQ8064) },
    { id_entry!(IPQ8066) },
    { id_entry!(IPQ8068) },
    { id_entry!(MSM8326) },
    { id_entry!(MSM8916) },
    { id_entry!(MSM8994) },
    { id_entry!(APQ8074PRO_AA, "APQ8074PRO-AA") },
    { id_entry!(APQ8074PRO_AB, "APQ8074PRO-AB") },
    { id_entry!(APQ8074PRO_AC, "APQ8074PRO-AC") },
    { id_entry!(MSM8274PRO_AA, "MSM8274PRO-AA") },
    { id_entry!(MSM8274PRO_AB, "MSM8274PRO-AB") },
    { id_entry!(MSM8274PRO_AC, "MSM8274PRO-AC") },
    { id_entry!(MSM8674PRO_AA, "MSM8674PRO-AA") },
    { id_entry!(MSM8674PRO_AB, "MSM8674PRO-AB") },
    { id_entry!(MSM8674PRO_AC, "MSM8674PRO-AC") },
    { id_entry!(MSM8974PRO_AA, "MSM8974PRO-AA") },
    { id_entry!(MSM8974PRO_AB, "MSM8974PRO-AB") },
    { id_entry!(APQ8028) },
    { id_entry!(MSM8128) },
    { id_entry!(MSM8228) },
    { id_entry!(MSM8528) },
    { id_entry!(MSM8628) },
    { id_entry!(MSM8928) },
    { id_entry!(MSM8510) },
    { id_entry!(MSM8512) },
    { id_entry!(MSM8936) },
    { id_entry!(MDM9640) },
    { id_entry!(MSM8939) },
    { id_entry!(APQ8036) },
    { id_entry!(APQ8039) },
    { id_entry!(MSM8236) },
    { id_entry!(MSM8636) },
    { id_entry!(MSM8909) },
    { id_entry!(MSM8996) },
    { id_entry!(APQ8016) },
    { id_entry!(MSM8216) },
    { id_entry!(MSM8116) },
    { id_entry!(MSM8616) },
    { id_entry!(MSM8992) },
    { id_entry!(APQ8092) },
    { id_entry!(APQ8094) },
    { id_entry!(MSM8209) },
    { id_entry!(MSM8208) },
    { id_entry!(MDM9209) },
    { id_entry!(MDM9309) },
    { id_entry!(MDM9609) },
    { id_entry!(MSM8239) },
    { id_entry!(MSM8952) },
    { id_entry!(APQ8009) },
    { id_entry!(MSM8956) },
    { id_entry!(MSM8929) },
    { id_entry!(MSM8629) },
    { id_entry!(MSM8229) },
    { id_entry!(APQ8029) },
    { id_entry!(APQ8056) },
    { id_entry!(MSM8609) },
    { id_entry!(APQ8076) },
    { id_entry!(MSM8976) },
    { id_entry!(IPQ8065) },
    { id_entry!(IPQ8069) },
    { id_entry!(MDM9650) },
    { id_entry!(MDM9655) },
    { id_entry!(MDM9250) },
    { id_entry!(MDM9255) },
    { id_entry!(MDM9350) },
    { id_entry!(APQ8052) },
    { id_entry!(MDM9607) },
    { id_entry!(APQ8096) },
    { id_entry!(MSM8998) },
    { id_entry!(MSM8953) },
    { id_entry!(MSM8937) },
    { id_entry!(APQ8037) },
    { id_entry!(MDM8207) },
    { id_entry!(MDM9207) },
    { id_entry!(MDM9307) },
    { id_entry!(MDM9628) },
    { id_entry!(MSM8909W) },
    { id_entry!(APQ8009W) },
    { id_entry!(MSM8996L) },
    { id_entry!(MSM8917) },
    { id_entry!(APQ8053) },
    { id_entry!(MSM8996SG) },
    { id_entry!(APQ8017) },
    { id_entry!(MSM8217) },
    { id_entry!(MSM8617) },
    { id_entry!(MSM8996AU) },
    { id_entry!(APQ8096AU) },
    { id_entry!(APQ8096SG) },
    { id_entry!(MSM8940) },
    { id_entry!(SDX201) },
    { id_entry!(SDM660) },
    { id_entry!(SDM630) },
    { id_entry!(APQ8098) },
    { id_entry!(MSM8920) },
    { id_entry!(SDM845) },
    { id_entry!(MDM9206) },
    { id_entry!(IPQ8074) },
    { id_entry!(SDA660) },
    { id_entry!(SDM658) },
    { id_entry!(SDA658) },
    { id_entry!(SDA630) },
    { id_entry!(MSM8905) },
    { id_entry!(SDX202) },
    { id_entry!(SDM670) },
    { id_entry!(SDM450) },
    { id_entry!(SM8150) },
    { id_entry!(SDA845) },
    { id_entry!(IPQ8072) },
    { id_entry!(IPQ8076) },
    { id_entry!(IPQ8078) },
    { id_entry!(SDM636) },
    { id_entry!(SDA636) },
    { id_entry!(SDM632) },
    { id_entry!(SDA632) },
    { id_entry!(SDA450) },
    { id_entry!(SDM439) },
    { id_entry!(SDM429) },
    { id_entry!(SM8250) },
    { id_entry!(SA8155) },
    { id_entry!(SDA439) },
    { id_entry!(SDA429) },
    { id_entry!(SM7150) },
    { id_entry!(SM7150P) },
    { id_entry!(IPQ8070) },
    { id_entry!(IPQ8071) },
    { id_entry!(QM215) },
    { id_entry!(IPQ8072A) },
    { id_entry!(IPQ8074A) },
    { id_entry!(IPQ8076A) },
    { id_entry!(IPQ8078A) },
    { id_entry!(SM6125) },
    { id_entry!(IPQ8070A) },
    { id_entry!(IPQ8071A) },
    { id_entry!(IPQ8172) },
    { id_entry!(IPQ8173) },
    { id_entry!(IPQ8174) },
    { id_entry!(IPQ6018) },
    { id_entry!(IPQ6028) },
    { id_entry!(SDM429W) },
    { id_entry!(SM4250) },
    { id_entry!(IPQ6000) },
    { id_entry!(IPQ6010) },
    { id_entry!(SC7180) },
    { id_entry!(SM6350) },
    { id_entry!(QCM2150) },
    { id_entry!(SDA429W) },
    { id_entry!(SM8350) },
    { id_entry!(QCM2290) },
    { id_entry!(SM7125) },
    { id_entry!(SM6115) },
    { id_entry!(IPQ5010) },
    { id_entry!(IPQ5018) },
    { id_entry!(IPQ5028) },
    { id_entry!(SC8280XP) },
    { id_entry!(IPQ6005) },
    { id_entry!(QRB5165) },
    { id_entry!(SM8450) },
    { id_entry!(SM7225) },
    { id_entry!(SA8295P) },
    { id_entry!(SA8540P) },
    { id_entry!(QCM4290) },
    { id_entry!(QCS4290) },
    { id_entry!(SM7325) },
    { id_entry!(SM8450_2, "SM8450") },
    { id_entry!(SM8450_3, "SM8450") },
    { id_entry!(SC7280) },
    { id_entry!(SC7180P) },
    { id_entry!(QCM6490) },
    { id_entry!(SM7325P) },
    { id_entry!(IPQ5000) },
    { id_entry!(IPQ0509) },
    { id_entry!(IPQ0518) },
    { id_entry!(SM6375) },
    { id_entry!(IPQ9514) },
    { id_entry!(IPQ9550) },
    { id_entry!(IPQ9554) },
    { id_entry!(IPQ9570) },
    { id_entry!(IPQ9574) },
    { id_entry!(SM8550) },
    { id_entry!(IPQ5016) },
    { id_entry!(IPQ9510) },
    { id_entry!(QRB4210) },
    { id_entry!(QRB2210) },
    { id_entry!(SAR2130P) },
    { id_entry!(SM8475) },
    { id_entry!(SM8475P) },
    { id_entry!(SA8255P) },
    { id_entry!(SA8775P) },
    { id_entry!(QRU1000) },
    { id_entry!(SM8475_2) },
    { id_entry!(QDU1000) },
    { id_entry!(X1E80100) },
    { id_entry!(SM8650) },
    { id_entry!(SM4450) },
    { id_entry!(SAR1130P) },
    { id_entry!(QDU1010) },
    { id_entry!(QRU1032) },
    { id_entry!(QRU1052) },
    { id_entry!(QRU1062) },
    { id_entry!(IPQ5332) },
    { id_entry!(IPQ5322) },
    { id_entry!(IPQ5312) },
    { id_entry!(IPQ5302) },
    { id_entry!(QCS8550) },
    { id_entry!(QCM8550) },
    { id_entry!(IPQ5300) },
    { id_entry!(IPQ5321) },
    { id_entry!(IPQ5424) },
    { id_entry!(IPQ5404) },
    { id_entry!(QCS9100) },
    { id_entry!(QCS8300) },
    { id_entry!(QCS8275) },
    { id_entry!(QCS9075) },
    { id_entry!(QCS615) },
];

fn qcom_smem_get(host: i32, item: u32) -> Result<&'static [u8]> {
    let mut size = 0;
    // SAFETY: TODO
    let ptr =
        from_err_ptr(unsafe { kernel::bindings::qcom_smem_get(host as _, item as _, &mut size) })?;
    // SAFETY: TODO
    Ok(unsafe { core::slice::from_raw_parts(ptr as *const _, size) })
}

#[pin_data]
struct QcomSocInfo {
    #[pin]
    registration: soc::DeviceRegistration,
    #[pin]
    debugfs: DebugfsValues<Params>,
}

// TODO better name
// TODO right constant
struct B32([u8; 32]);

impl Printer for B32 {
    fn print(&self, seq_file: &SeqFile) -> i32 {
        let Some(nul_idx) = self.0.iter().position(|x| x == &0) else {
            seq_print!(seq_file, "Missing NUL terminator: {:?}", self.0);
            return 0;
        };
        let Ok(cs) = CStr::from_bytes_with_nul(&self.0[0..=nul_idx]) else {
            seq_print!(
                seq_file,
                "Internal error - first NUL no longer first NUL: {:?}",
                self.0
            );
            return 0;
        };
        seq_print!(seq_file, "{cs}");
        0
    }
}

// TODO remove
#[allow(dead_code)]
struct Params {
    raw_device_family: u32,
    hw_plat_subtype: u32,
    accessory_chip: u32,
    raw_device_num: u32,
    chip_family: u32,
    foundry_id: u32,
    plat_ver: u32,
    raw_ver: u32,
    hw_plat: u32,
    fmt: u32,
    nproduct_id: u32,
    num_clusters: u32,
    ncluster_array_offset: u32,
    num_subset_parts: u32,
    nsubset_parts_array_offset: u32,
    nmodem_supported: u32,
    feature_code: u32,
    pcode: u32,
    oem_variant: u32,
    num_func_clusters: u32,
    boot_cluster: u32,
    boot_core: u32,
    build_id: B32,
}

#[derive(Copy, Clone)]
struct SocInfo<'a>(&'a kernel::bindings::socinfo);
impl<'a> SocInfo<'a> {
    fn id(&self) -> u32 {
        u32::from_le(self.0.id)
    }
    fn version_split(ver: u32) -> (u16, u16) {
        let major = (ver >> 16) as u16;
        let minor = (ver & 0xFFFF) as u16;
        (major, minor)
    }
    fn version(&self) -> (u16, u16) {
        Self::version_split(self.0.ver)
    }
    fn serial(&self) -> u32 {
        u32::from_le(self.0.id)
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
impl SocInfo<'static> {
    fn build_params(&self) -> Result<Params> {
        unimplemented!()
    }
}
impl SocInfo<'static> {
    fn build_debugfs(&self) -> impl PinInit<DebugfsValues<Params>, Error> {
        // TODO no unwrap or propagate
        let backing = self.build_params().unwrap();
        // TODO remove _rs
        // TODO no unwrap or propagate
        let debugfs = DebugfsDir::new(c_str!("qcom_socinfo_rs")).unwrap();
        DebugfsValues::attach(backing, debugfs)
    }
}

impl QcomSocInfo {
    fn enable_debugfs(&self) -> Result<()> {
        self.debugfs.build(|params, builder| {
            builder.value_file(c_str!("info_fmt"), X32::from_u32_ref(&params.fmt));
            builder.seq_file(c_str!("build_id"), &params.build_id);
        });
        Ok(())
    }
}

impl platform::Driver for QcomSocInfo {
    type IdInfo = ();
    const OF_ID_TABLE: Option<kernel::of::IdTable<Self::IdInfo>> = None;
    fn probe(_dev: &mut Device, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        let mem = qcom_smem_get(
            kernel::bindings::QCOM_SMEM_HOST_ANY,
            kernel::bindings::SMEM_HW_SW_BUILD_ID,
        )?;
        // TODO abstract
        // SAFETY: TODO
        let info = SocInfo(unsafe {
            mem.as_ptr()
                .cast::<kernel::bindings::socinfo>()
                .as_ref()
                .unwrap()
        });
        let soc_info = KBox::pin_init(
            try_pin_init!(
                    Self {
                        registration <- soc::DeviceRegistration::register(info.device_attribute()?),
                        debugfs <- info.build_debugfs(),
                    }
            ),
            GFP_KERNEL,
        )?;

        soc_info.enable_debugfs()?;

        kernel::rand::add_device_randomness(mem);

        Ok(soc_info)
    }
}
