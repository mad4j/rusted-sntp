use chrono::{Local, TimeZone, Timelike, Utc};
use log::debug;

#[cfg(unix)]
use unix::sync_time;
#[cfg(windows)]
use windows::sync_time;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

/// Set up system time based on the given parameters
/// Args:
/// * sec - Seconds since UNIX epoch start
/// * nsec - Fraction of seconds from an NTP response
pub fn update_system_time(sec: u32, nsec: u32) {
    let time = Utc.timestamp(sec as i64, nsec);
    let local_time = time.with_timezone(&Local);
    debug!(
        "UTC time: {:02}:{:02}:{:02}",
        time.hour(),
        time.minute(),
        time.second()
    );
    debug!(
        "{} time: {:02}:{:02}:{:02}",
        local_time.offset(),
        local_time.hour(),
        local_time.minute(),
        local_time.second()
    );

    sync_time(local_time);
}
