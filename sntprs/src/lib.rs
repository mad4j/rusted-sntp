//! Rust SNTP client
//!
//! This crate provides a method for sending requests to NTP servers
//! and process responses, extracting received timestamp
//!
//! # Example
//!
//! ```rust
//! use sntpc;
//!
//! let result = sntpc::request("pool.ntp.org", 123);
//!
//! if let Ok(sntpc::NtpResult {
//!     sec, nsec, roundtrip, offset
//! }) = result {
//!     println!("NTP server time: {}.{}", sec, nsec);
//!     println!("Roundtrip time: {}, offset: {}", roundtrip, offset);
//! }
//! ```

mod ntppacket;
mod ntpresult;

pub mod utils;

use crate::ntpresult::NtpResult;
use log::debug;
use std::io;
use std::mem;
use std::net;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str;
use std::time;

use ntppacket::NtpPacket;

const MODE_MASK: u8 = 0b0000_0111;
const MODE_SHIFT: u8 = 0;
const VERSION_MASK: u8 = 0b0011_1000;
const VERSION_SHIFT: u8 = 3;
const LI_MASK: u8 = 0b1100_0000;
const LI_SHIFT: u8 = 6;
const NSEC_IN_SEC: u32 = 1_000_000_000;



trait NtpNum {
    type Type;

    fn ntohl(&self) -> Self::Type;
}

impl NtpNum for u32 {
    type Type = u32;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}
impl NtpNum for u64 {
    type Type = u64;

    fn ntohl(&self) -> Self::Type {
        self.to_be()
    }
}

struct RawNtpPacket([u8; mem::size_of::<NtpPacket>()]);

impl Default for RawNtpPacket {
    fn default() -> Self {
        RawNtpPacket([0u8; mem::size_of::<NtpPacket>()])
    }
}

impl From<RawNtpPacket> for NtpPacket {
    fn from(val: RawNtpPacket) -> Self {
        // left it here for a while, maybe in future Rust releases there
        // will be a way to use such a generic function with compile-time
        // size determination
        // const fn to_array<T: Sized>(x: &[u8]) -> [u8; mem::size_of::<T>()] {
        //     let mut temp_buf = [0u8; mem::size_of::<T>()];
        //
        //     temp_buf.copy_from_slice(x);
        //     temp_buf
        // }
        let to_array_u32 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u32>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };
        let to_array_u64 = |x: &[u8]| {
            let mut temp_buf = [0u8; mem::size_of::<u64>()];
            temp_buf.copy_from_slice(x);
            temp_buf
        };

        NtpPacket {
            li_vn_mode: val.0[0],
            stratum: val.0[1],
            poll: val.0[2] as i8,
            precision: val.0[3] as i8,
            root_delay: u32::from_le_bytes(to_array_u32(&val.0[4..8])),
            root_dispersion: u32::from_le_bytes(to_array_u32(&val.0[8..12])),
            ref_id: u32::from_le_bytes(to_array_u32(&val.0[12..16])),
            ref_timestamp: u64::from_le_bytes(to_array_u64(&val.0[16..24])),
            origin_timestamp: u64::from_le_bytes(to_array_u64(&val.0[24..32])),
            recv_timestamp: u64::from_le_bytes(to_array_u64(&val.0[32..40])),
            tx_timestamp: u64::from_le_bytes(to_array_u64(&val.0[40..48])),
        }
    }
}

impl From<&NtpPacket> for RawNtpPacket {
    fn from(val: &NtpPacket) -> Self {
        let mut tmp_buf = [0u8; mem::size_of::<NtpPacket>()];

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

        RawNtpPacket(tmp_buf)
    }
}

/// Send request to a NTP server with the given address
/// and process the response
///
/// * `pool` - Server's name or IP address as a string
/// * `port` - Server's port as an int
///
/// # Example
///
/// ```rust
/// use sntpc;
///
/// let result = sntpc::request("time.google.com", 123);
/// // OR
/// let result = sntpc::request("83.168.200.199", 123);
///
/// // .. process the result
/// ```
pub fn request(pool: &str, port: u32) -> io::Result<NtpResult> {
    debug!("Pool: {}", pool);
    let socket = net::UdpSocket::bind("0.0.0.0:0")
        .expect("Unable to create a UDP socket");
    let dest = format!("{}:{}", pool, port).to_socket_addrs()?;

    socket
        .set_read_timeout(Some(time::Duration::new(2, 0)))
        .expect("Unable to set up socket timeout");
    let req = NtpPacket::new();
    let dest = process_request(dest, &req, &socket)?;
    let mut buf: RawNtpPacket = RawNtpPacket::default();
    let (response, src) = socket.recv_from(buf.0.as_mut())?;
    let recv_timestamp = get_ntp_timestamp();
    debug!("Response: {}", response);

    if src != dest {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "SNTP response port / address mismatch",
        ));
    }

    if response == mem::size_of::<NtpPacket>() {
        let result = process_response(&req, buf, recv_timestamp);

        return match result {
            Ok(result) => {
                debug!("{:?}", result);
                Ok(result)
            }
            Err(err_str) => Err(io::Error::new(io::ErrorKind::Other, err_str)),
        };
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Incorrect NTP packet size read",
    ))
}

fn process_request(
    dest: std::vec::IntoIter<SocketAddr>,
    req: &NtpPacket,
    socket: &UdpSocket,
) -> io::Result<SocketAddr> {
    for addr in dest {
        debug!("Address: {}", &addr);

        match send_request(&req, &socket, addr) {
            Ok(write_bytes) => {
                assert_eq!(write_bytes, mem::size_of::<NtpPacket>());
                return Ok(addr);
            }
            Err(err) => debug!("{}. Try another one", err),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AddrNotAvailable,
        "SNTP servers not responding",
    ))
}

fn send_request(
    req: &NtpPacket,
    socket: &net::UdpSocket,
    dest: net::SocketAddr,
) -> io::Result<usize> {
    let buf: RawNtpPacket = req.into();

    socket.send_to(&buf.0, dest)
}

fn process_response(
    req: &NtpPacket,
    resp: RawNtpPacket,
    recv_timestamp: u64,
) -> Result<NtpResult, &str> {
    const SNTP_UNICAST: u8 = 4;
    const SNTP_BROADCAST: u8 = 5;
    const LI_MAX_VALUE: u8 = 3;
    const MSEC_MASK: u64 = 0x0000_0000_ffff_ffff;
    let shifter = |val, mask, shift| (val & mask) >> shift;
    let mut packet = NtpPacket::from(resp);

    convert_from_network(&mut packet);
    #[cfg(debug_assertions)]
    debug_ntp_packet(&packet);

    if req.tx_timestamp != packet.origin_timestamp {
        return Err("Incorrect origin timestamp");
    }
    // Shift is 0
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);
    let resp_version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let req_version = shifter(req.li_vn_mode, VERSION_MASK, VERSION_SHIFT);

    if mode != SNTP_UNICAST && mode != SNTP_BROADCAST {
        return Err("Incorrect MODE value");
    }

    if li > LI_MAX_VALUE {
        return Err("Incorrect LI value");
    }

    if req_version != resp_version {
        return Err("Incorrect response version");
    }

    if packet.stratum == 0 {
        return Err("Incorrect STRATUM headers");
    }
    //    theta = T(B) - T(A) = 1/2 * [(T2-T1) + (T3-T4)]
    //    and the round-trip delay
    //    delta = T(ABA) = (T4-T1) - (T3-T2).
    //    where:
    //      - T1 = client's TX timestamp
    //      - T2 = server's RX timestamp
    //      - T3 = server's TX timestamp
    //      - T4 = client's RX timestamp
    let delta = (recv_timestamp - packet.origin_timestamp) as i64
        - (packet.tx_timestamp - packet.recv_timestamp) as i64;
    let theta = ((packet.recv_timestamp as i64
        - packet.origin_timestamp as i64)
        + (recv_timestamp as i64 - packet.tx_timestamp as i64))
        / 2;

    debug!("Roundtrip delay: {} us. Offset: {} us", delta.abs(), theta);

    let seconds = (packet.tx_timestamp >> 32) as u32;
    let nsec = (packet.tx_timestamp & MSEC_MASK) as u32;
    let tx_tm = seconds - NtpPacket::NTP_TIMESTAMP_DELTA;

    Ok(NtpResult::new(tx_tm, nsec, delta.abs() as u64, theta))
}

fn convert_from_network(packet: &mut NtpPacket) {
    fn ntohl<T: NtpNum>(val: T) -> T::Type {
        val.ntohl()
    }

    packet.root_delay = ntohl(packet.root_delay);
    packet.root_dispersion = ntohl(packet.root_dispersion);
    packet.ref_id = ntohl(packet.ref_id);
    packet.ref_timestamp = ntohl(packet.ref_timestamp);
    packet.origin_timestamp = ntohl(packet.origin_timestamp);
    packet.recv_timestamp = ntohl(packet.recv_timestamp);
    packet.tx_timestamp = ntohl(packet.tx_timestamp);
}

#[cfg(debug_assertions)]
fn debug_ntp_packet(packet: &NtpPacket) {
    let shifter = |val, mask, shift| (val & mask) >> shift;
    let mode = shifter(packet.li_vn_mode, MODE_MASK, MODE_SHIFT);
    let version = shifter(packet.li_vn_mode, VERSION_MASK, VERSION_SHIFT);
    let li = shifter(packet.li_vn_mode, LI_MASK, LI_SHIFT);

    debug!("{}", (0..52).map(|_| "=").collect::<String>());
    debug!("| Mode:\t\t{}", mode);
    debug!("| Version:\t{}", version);
    debug!("| Leap:\t\t{}", li);
    debug!("| Stratum:\t{}", packet.stratum);
    debug!("| Poll:\t\t{}", packet.poll);
    debug!("| Precision:\t\t{}", packet.precision);
    debug!("| Root delay:\t\t{}", packet.root_delay);
    debug!("| Root dispersion:\t{}", packet.root_dispersion);
    debug!(
        "| Reference ID:\t\t{}",
        str::from_utf8(&packet.ref_id.to_be_bytes()).unwrap_or("")
    );
    debug!("| Reference timestamp:\t{:>16}", packet.ref_timestamp);
    debug!("| Origin timestamp:\t\t{:>16}", packet.origin_timestamp);
    debug!("| Receive timestamp:\t\t{:>16}", packet.recv_timestamp);
    debug!("| Transmit timestamp:\t\t{:>16}", packet.tx_timestamp);
    debug!("{}", (0..52).map(|_| "=").collect::<String>());
}

fn get_ntp_timestamp() -> u64 {
    let now_since_unix = time::SystemTime::now()
        .duration_since(time::SystemTime::UNIX_EPOCH)
        .unwrap();
    let timestamp = ((now_since_unix.as_secs()
        + (u64::from(NtpPacket::NTP_TIMESTAMP_DELTA)))
        << 32)
        + u64::from(now_since_unix.subsec_micros());

    timestamp
}

#[cfg(test)]
mod sntpc_tests {
    use crate::{NtpResult, NSEC_IN_SEC};

    #[test]
    fn test_ntp_result() {
        let result1 = NtpResult::new(0, 0, 0, 0);

        assert_eq!(0, result1.sec());
        assert_eq!(0, result1.nsec());
        assert_eq!(0, result1.roundtrip());
        assert_eq!(0, result1.offset());

        let result2 = NtpResult::new(1, 2, 3, 4);

        assert_eq!(1, result2.sec());
        assert_eq!(2, result2.nsec());
        assert_eq!(3, result2.roundtrip());
        assert_eq!(4, result2.offset());

        let residue3 = u32::max_value() / NSEC_IN_SEC;
        let result3 = NtpResult::new(
            u32::max_value() - residue3,
            u32::max_value(),
            u64::max_value(),
            i64::max_value(),
        );

        assert_eq!(u32::max_value(), result3.sec());
        assert_eq!(u32::max_value() % NSEC_IN_SEC, result3.nsec());
        assert_eq!(u64::max_value(), result3.roundtrip());
        assert_eq!(i64::max_value(), result3.offset());
    }

    #[test]
    fn test_ntp_nsec_overflow_result() {
        let result = NtpResult::new(0, u32::max_value(), 0, 0);
        let max_value_sec = u32::max_value() / NSEC_IN_SEC;
        let max_value_nsec = u32::max_value() % NSEC_IN_SEC;

        assert_eq!(max_value_sec, result.sec());
        assert_eq!(max_value_nsec, result.nsec());
        assert_eq!(0, result.roundtrip());
        assert_eq!(0, result.offset());
    }
}
