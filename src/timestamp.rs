// It has been 14,014,252,440 seconds since October 15, 1582, at 00:00:00 UTC (the start
// of the Gregorian calendar reform / UUIDv1 epoch), calculated up to November 17, 2026, at 23:54:00 UTC.

use crate::error::GwidError;
use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

const NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND: u64 = 10_000_000;
// Seconds between 1582-10-15 00:00:00 UTC and 1970-01-01 00:00:00 UTC
const GREGORIAN_TO_UNIX_SECS: u64 = 12_219_292_800;
// In 100-nanosecond ticks (common in UUID v1/v6 and Windows FILETIME math)
const GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS: u64 = 122_192_928_000_000_000;

const SIXTY_BITS_MASK: u64 = (1 << 60) - 1;

/// Already accounts for UNIX_EPOCH.
pub struct UnixTimestamp(pub(crate) u128);

impl UnixTimestamp {
    /// Current unix time in nanoseconds.
    ///
    /// # Panics
    ///
    /// If system time is before Unix epoch (January 1, 1970, at 00:00:00 UTC) this method will panic.
    /// The fallible version of this method is `now_checked()`.
    pub fn now() -> Self {
        let unix_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is set before UNIX_EPOCH (January 1, 1970, at 00:00:00 UTC)")
            .as_nanos();

        Self(unix_nanos)
    }

    /// Current unix time in nanoseconds.
    /// The "unchecked" version of this method is `now()`, which may panic!
    pub fn now_checked() -> Result<Self, GwidError> {
        let unix_nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(Self(unix_nanos))
    }

    /// Already accounts for UNIX_EPOCH.
    pub const fn from_nanoseconds(ns: u128) -> Self {
        Self(ns)
    }

    /// Already accounts for UNIX_EPOCH.
    pub const fn from_seconds_and_subsec_nanos(seconds: u64, subsec_nanos: u32) -> Self {
        let unix_nanos = Timestamp::seconds_and_subsec_nanos_to_nanos(seconds, subsec_nanos);
        Self(unix_nanos)
    }

    /// Fallible version of `from(SystemTime)`.
    pub fn try_from_system_time(system_time: SystemTime) -> Result<Self, GwidError> {
        let unix_nanos = system_time.duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(Self(unix_nanos))
    }

    /// Converts the timestamp back into a `std::time::SystemTime`.
    pub fn to_system_time(&self) -> SystemTime {
        let (seconds, subsec_nanos) = Timestamp::ns_to_secs_and_subsec_nanos(self.0);
        UNIX_EPOCH + Duration::new(seconds, subsec_nanos)
    }

    /// Get current unix nanoseconds
    pub const fn time(&self) -> u128 {
        self.0
    }
}

impl From<SystemTime> for UnixTimestamp {
    /// Get a `UnixTimestamp` from `SystemTime`.
    ///
    /// # Panics
    ///
    /// If system time is before Unix epoch (January 1, 1970, at 00:00:00 UTC) this method will panic.
    /// Use `try_from_system_time(SystemTime)`if you need a fallible version of this method.
    fn from(system_time: SystemTime) -> Self {
        let unix_nanos = system_time
            .duration_since(UNIX_EPOCH)
            .expect("System time is set before UNIX_EPOCH (January 1, 1970, at 00:00:00 UTC)")
            .as_nanos();

        Self(unix_nanos)
    }
}

impl From<UnixTimestamp> for SystemTime {
    fn from(unix_timestamp: UnixTimestamp) -> Self {
        unix_timestamp.to_system_time()
    }
}

pub struct Timestamp {
    unix_nanos: u128,
}

impl Timestamp {
    /// Maximum number of nanoseconds that can be represented when storing seconds
    /// as `u64` and subsecond nanoseconds as `u32`.
    const MAX_REPRESENTABLE_NANOSECONDS: u128 = 18_446_744_073_709_551_619_294_967_295;
    /// Number of nanoseconds in a second.
    const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

    /// Nanoseconds to seconds and subbsecond nanoseconds.
    pub const fn ns_to_secs_and_subsec_nanos(nanos: u128) -> (u64, u32) {
        if nanos > Timestamp::MAX_REPRESENTABLE_NANOSECONDS {
            return (u64::MAX, u32::MAX);
        }

        let secs = Self::ns_to_secs(nanos);

        if secs > u64::MAX as u128 {
            const MAX_NANOSECONDS_CARRY: u128 =
                u64::MAX as u128 * Timestamp::NANOSECONDS_PER_SECOND;

            (u64::MAX, (nanos - MAX_NANOSECONDS_CARRY) as u32)
        } else {
            (secs as u64, (nanos % 1_000_000_000) as u32)
        }
    }

    /// Nanoseconds to seconds.
    pub const fn ns_to_secs(nanos: u128) -> u128 {
        nanos / 1_000_000_000
    }

    pub const fn seconds_and_subsec_nanos_to_nanos(seconds: u64, subsec_nanos: u32) -> u128 {
        (seconds as u128 * 1_000_000_000) + subsec_nanos as u128
    }
}

// ------------------------------------------------------------------------------------------------------------------------------------
// OLD STUFF BELOW
// ------------------------------------------------------------------------------------------------------------------------------------

fn gregorian_epoch() -> SystemTime {
    UNIX_EPOCH - Duration::from_secs(GREGORIAN_TO_UNIX_SECS)
}

fn current_gregorian_ticks() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before Unix epoch");

    // Convert seconds and nanoseconds into 100-nanosecond ticks
    let unix_ticks = duration.as_secs() * 10_000_000 + (duration.subsec_nanos() as u64) / 100;

    unix_ticks + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS
}

pub fn gregorian_now() -> u128 {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    duration.as_nanos() / 100 + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS as u128
}

#[cfg(test)]
mod test {
    use rand::RngExt;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn timestamp_bar() {
        let uts = UnixTimestamp::now();
        let uts_sys_time: SystemTime = uts.into();
        let back_to_uts: UnixTimestamp = uts_sys_time.into();
        let mut r: [u8; 6] = [0; 6];
        loop {
            rand::rng().fill(&mut r);
            let idv1 = Uuid::now_v1(&r);
            println!("{idv1} | {}", idv1.as_u128());

            // let id = Uuid::now_v7();
            // println!("uuid = {id}");
            // let mid = crate::v7::generate_strict().unwrap();
            // println!("mine = {mid}");
            // println!();
        }
    }

    #[test]
    fn timestamp_foo() {
        let unix_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let unix_ts_secs = unix_ts.as_secs();
        let unix_ts_ms = unix_ts.as_millis();
        let unix_ts_ms_calcuated = unix_ts_secs * 1_000;
        let unix_ts_subsec_millis = unix_ts.subsec_millis();

        let unix_ts_secs_as_gregorian_ticks =
            (unix_ts_secs * NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND);
        let gregorian_timestamp_raw =
            unix_ts_secs_as_gregorian_ticks + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS;
        let gregorian_timestamp = gregorian_timestamp_raw & SIXTY_BITS_MASK;

        const GREGORIAN_OFFSET: u128 = 122_192_928_000_000_000;
        let gregorian_timestamp = unix_ts_ms * 10_000_u128 + GREGORIAN_OFFSET;

        println!("sec      = {unix_ts_secs}");
        println!("ms       = {unix_ts_ms}");
        println!("ms_calc  = {unix_ts_ms_calcuated} + {unix_ts_subsec_millis} subsec_millis");
        println!("greg     = {gregorian_timestamp}");
        println!("greg_now = {}", gregorian_now());
        println!("\n\n");

        println!();

        let mine_greg = SystemTime::now().duration_since(gregorian_epoch()).unwrap();
        let mine_greg_secs = mine_greg.as_secs();
        let mine_greg_as_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH - Duration::from_secs(mine_greg.as_secs()))
            .unwrap();
        let mine_greg_as_unix_nanos = mine_greg_as_unix.as_secs();
        println!("mine    = {mine_greg_secs} unix = {mine_greg_as_unix_nanos}");

        let theirs_greg = Duration::from_secs(
            current_gregorian_ticks() / NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND,
        );
        let theirs_greg_secs = theirs_greg.as_secs();
        let theirs_greg_as_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH - Duration::from_secs(theirs_greg.as_secs()))
            .unwrap();
        let theirs_greg_as_unix_nanos = theirs_greg_as_unix.as_secs();
        println!("theirs  = {theirs_greg_secs} unix = {theirs_greg_as_unix_nanos}");
    }
}
