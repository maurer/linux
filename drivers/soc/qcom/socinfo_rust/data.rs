// SPDX-License-Identifier: GPL-2.0

// Copyright (C) 2025 Google LLC.

//! Data tables for QCom SocInfo driver
use kernel::c_str;
use kernel::prelude::*;

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

pub(crate) struct SocId {
    pub id: u32,
    pub name: &'static CStr,
}

pub(crate) static SOC_IDS: &[SocId] = &[
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

pub(crate) const PMIC_MODELS: [Option<&str>; 84] = {
    let mut models = [None; 84];
    models[0] = Some("Unknown PMIC model");
    models[1] = Some("PM8941");
    models[2] = Some("PM8841");
    models[3] = Some("PM8019");
    models[4] = Some("PM8226");
    models[5] = Some("PM8110");
    models[6] = Some("PMA8084");
    models[7] = Some("PMI8962");
    models[8] = Some("PMD9635");
    models[9] = Some("PM8994");
    models[10] = Some("PMI8994");
    models[11] = Some("PM8916");
    models[12] = Some("PM8004");
    models[13] = Some("PM8909/PM8058");
    models[14] = Some("PM8028");
    models[15] = Some("PM8901");
    models[16] = Some("PM8950/PM8027");
    models[17] = Some("PMI8950/ISL9519");
    models[18] = Some("PMK8001/PM8921");
    models[19] = Some("PMI8996/PM8018");
    models[20] = Some("PM8998/PM8015");
    models[21] = Some("PMI8998/PM8014");
    models[22] = Some("PM8821");
    models[23] = Some("PM8038");
    models[24] = Some("PM8005/PM8922");
    models[25] = Some("PM8917/PM8937");
    models[26] = Some("PM660L");
    models[27] = Some("PM660");
    models[30] = Some("PM8150");
    models[31] = Some("PM8150L");
    models[32] = Some("PM8150B");
    models[33] = Some("PMK8002");
    models[36] = Some("PM8009");
    models[37] = Some("PMI632");
    models[38] = Some("PM8150C");
    models[40] = Some("PM6150");
    models[41] = Some("SMB2351");
    models[44] = Some("PM8008");
    models[45] = Some("PM6125");
    models[46] = Some("PM7250B");
    models[47] = Some("PMK8350");
    models[48] = Some("PM8350");
    models[49] = Some("PM8350C");
    models[50] = Some("PM8350B");
    models[51] = Some("PMR735A");
    models[52] = Some("PMR735B");
    models[54] = Some("PM6350");
    models[55] = Some("PM4125");
    models[58] = Some("PM8450");
    models[65] = Some("PM8010");
    models[69] = Some("PM8550VS");
    models[70] = Some("PM8550VE");
    models[71] = Some("PM8550B");
    models[72] = Some("PMR735D");
    models[73] = Some("PM8550");
    models[74] = Some("PMK8550");
    models[82] = Some("PMC8380");
    models[83] = Some("SMB2360");
    models
};

pub(crate) const IMAGE_NAMES: &[(&CStr, usize)] = &[
    (c_str!("adsp"), 12),
    (c_str!("apps"), 10),
    (c_str!("boot"), 0),
    (c_str!("cnss"), 13),
    (c_str!("mpss"), 11),
    (c_str!("rpm"), 3),
    (c_str!("tz"), 1),
    (c_str!("video"), 14),
    (c_str!("dsps"), 15),
    (c_str!("cdsp"), 16),
    (c_str!("cdsp1"), 19),
    (c_str!("gpdsp"), 20),
    (c_str!("gpdsp1"), 21),
];
