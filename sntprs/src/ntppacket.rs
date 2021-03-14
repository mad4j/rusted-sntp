use log::debug;

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
    const NTP_TIMESTAMP_DELTA: u32 = 2_208_988_800u32;
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
