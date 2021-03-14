
use crate::get_ntp_timestamp;
use log::debug;

pub const NTP_PACKET_SIZE: usize = 48;

pub type RawPacket = [u8; NTP_PACKET_SIZE];


//dividere li_vn_mode in tre campi e aggiornare la conversione da per raw bytes
//dimensione è 48 bytes
pub struct NtpPacket {
    pub li_vn_mode: u8,
    pub stratum: u8,
    pub poll: i8,
    pub precision: i8,
    pub root_delay: u32,
    pub root_dispersion: u32,
    pub ref_id: u32,
    pub ref_timestamp: u64,
    pub origin_timestamp: u64,
    pub recv_timestamp: u64,
    pub tx_timestamp: u64,
}


impl NtpPacket {
    pub const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
    const SNTP_CLIENT_MODE: u8 = 3;
    const SNTP_VERSION: u8 = 4 << 3;
    #[allow(dead_code)]
    const LI_MASK: u8 = 0b0000_0011;
    #[allow(dead_code)]
    const VN_MASK: u8 = 0b0001_1100;
    #[allow(dead_code)]
    const MODE_MASK: u8 = 0b1110_0000;

    pub fn new() -> NtpPacket {
        let tx_timestamp = get_ntp_timestamp();

        debug!("{}", tx_timestamp);

        NtpPacket {
            li_vn_mode: NtpPacket::SNTP_CLIENT_MODE | NtpPacket::SNTP_VERSION,
            stratum: 0,
            poll: 0,
            precision: 0,
            root_delay: 0,
            root_dispersion: 0,
            ref_id: 0,
            ref_timestamp: 0,
            origin_timestamp: 0,
            recv_timestamp: 0,
            tx_timestamp,
        }
    }
}

impl From<RawPacket> for NtpPacket {
    fn from(val: RawPacket) -> Self {
         NtpPacket {
            li_vn_mode: val[0],
            stratum: val[1],
            poll: val[2] as i8,
            precision: val[3] as i8,
            root_delay: u32::from_le_bytes(*array_ref![val, 4, 4]),
            root_dispersion: u32::from_le_bytes(*array_ref![val, 8, 4]),
            ref_id: u32::from_le_bytes(*array_ref![val, 12, 4]),
            ref_timestamp: u64::from_le_bytes(*array_ref![val, 16, 8]),
            origin_timestamp: u64::from_le_bytes(*array_ref![val, 24, 8]),
            recv_timestamp: u64::from_le_bytes(*array_ref![val, 32, 8]),
            tx_timestamp: u64::from_le_bytes(*array_ref![val, 40, 8]),
        }
    }
}

impl From<&NtpPacket> for RawPacket {
    fn from(val: &NtpPacket) -> Self {
        let mut tmp_buf = [0u8; NTP_PACKET_SIZE];

        tmp_buf[0] = val.li_vn_mode;
        tmp_buf[1] = val.stratum;
        tmp_buf[2] = val.poll as u8;
        tmp_buf[3] = val.precision as u8;
        tmp_buf[4..8].copy_from_slice(&val.root_delay.to_be_bytes());
        tmp_buf[8..12].copy_from_slice(&val.root_dispersion.to_be_bytes());
        tmp_buf[12..16].copy_from_slice(&val.ref_id.to_be_bytes());
        tmp_buf[16..24].copy_from_slice(&val.ref_timestamp.to_be_bytes());
        tmp_buf[24..32].copy_from_slice(&val.origin_timestamp.to_be_bytes());
        tmp_buf[32..40].copy_from_slice(&val.recv_timestamp.to_be_bytes());
        tmp_buf[40..48].copy_from_slice(&val.tx_timestamp.to_be_bytes());

        tmp_buf
    }
}
