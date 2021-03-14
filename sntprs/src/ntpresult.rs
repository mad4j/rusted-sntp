
use std::fmt::Debug;
use std::fmt::Formatter;
use crate::NSEC_IN_SEC;

/// SNTP request result representation
pub struct NtpResult {
    /// NTP server seconds value
    pub sec: u32,
    /// NTP server nanoseconds value
    pub nsec: u32,
    /// Request roundtrip time
    pub roundtrip: u64,
    /// Offset of the current system time with one received from a NTP server
    pub offset: i64,
}

impl NtpResult {
    /// Create new NTP result
    /// Args:
    /// * `sec` - number of seconds
    /// * `nsec` - number of nanoseconds
    /// * `roundtrip` - calculated roundtrip in microseconds
    /// * `offset` - calculated system clock offset in microseconds
    pub fn new(sec: u32, nsec: u32, roundtrip: u64, offset: i64) -> Self {
        let residue = nsec / NSEC_IN_SEC;
        let nsec = nsec % NSEC_IN_SEC;
        let sec = sec + residue;

        NtpResult {
            sec,
            nsec,
            roundtrip,
            offset,
        }
    }
    /// Returns number of seconds reported by an NTP server
    pub fn sec(&self) -> u32 {
        self.sec
    }

    /// Returns number of nanoseconds reported by an NTP server
    pub fn nsec(&self) -> u32 {
        self.nsec
    }

    /// Returns request's roundtrip time (client -> server -> client) in microseconds
    pub fn roundtrip(&self) -> u64 {
        self.roundtrip
    }

    /// Returns system clock offset value in microseconds
    pub fn offset(&self) -> i64 {
        self.offset
    }
}

impl Debug for NtpResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NtpResult")
            .field("sec", &self.sec)
            .field("nsec", &self.nsec)
            .field("roundtrip", &self.roundtrip)
            .field("offset", &self.offset)
            .finish()
    }
}
