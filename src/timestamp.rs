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
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct UnixTimestamp(pub(crate) u128);

impl UnixTimestamp {
    /// Maximum number of nanoseconds that can be represented when storing seconds
    /// as `u64` and subsecond nanoseconds as `u32`.
    const MAX_REPRESENTABLE_NANOSECONDS: u128 = 18_446_744_073_709_551_619_294_967_295;
    /// Max milliseconds represented in nanoseconds.
    const MAX_MILLIS_EXPRESSED_AS_NANOS: u128 = 18_446_744_073_709_551_615_999_999;
    /// Number of nanoseconds in a second.
    const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;

    const MAX: Self = Self::from_nanos(Self::MAX_REPRESENTABLE_NANOSECONDS);

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
    pub const fn from_nanos(ns: u128) -> Self {
        Self(ns)
    }

    /// Already accounts for UNIX_EPOCH.
    pub const fn from_parts(seconds: u64, subsec_nanos: u32) -> Self {
        let unix_nanos = parts_to_nanos(seconds, subsec_nanos);
        Self(unix_nanos)
    }

    /// Fallible version of `<UnixTimestamp>.from(<SystemTime>)`.
    pub fn try_from_system_time(system_time: SystemTime) -> Result<Self, GwidError> {
        let unix_nanos = system_time.duration_since(UNIX_EPOCH)?.as_nanos();
        Ok(Self(unix_nanos))
    }

    pub fn as_millis_saturating(&self) -> u64 {
        (self.as_nanos_clamped_to_u64_millis() / 1_000_000) as u64
    }

    /// Timestamp to seconds and subsec_nanos.
    pub fn to_parts(&self) -> (u64, u32) {
        nanos_to_parts(self.0)
    }

    /// Timestamp is clamped to max number of millis (as u64), but in nanosecond precision (u128).
    pub fn as_nanos_clamped_to_u64_millis(&self) -> u128 {
        nanos_clamped_to_u64_millis(self.0)
    }

    pub fn submillisecond_precision(&self) -> u16 {
        let submillis = self.0 % 1_000_000;
        ((submillis << 12) / 1_000_000) as u16
    }

    pub const fn as_nanos(&self) -> u128 {
        self.0
    }
}

impl From<UnixTimestamp> for SystemTime {
    fn from(timestamp: UnixTimestamp) -> Self {
        let (secs, nanos) = timestamp.to_parts();
        let dur_secs = Duration::from_secs(secs);
        let dur_nanos = Duration::from_nanos(nanos as u64);

        dur_secs
            .checked_add(dur_nanos)
            .and_then(|d| UNIX_EPOCH.checked_add(d))
            .unwrap_or(UNIX_EPOCH)
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

pub fn nanos_clamped_to_u64_millis(nanos: u128) -> u128 {
    nanos.min(UnixTimestamp::MAX_MILLIS_EXPRESSED_AS_NANOS)
}

pub fn nanos_as_millis_saturating(nanos: u128) -> u64 {
    (nanos_clamped_to_u64_millis(nanos) / 1_000_000) as u64
}

/// Nanoseconds to seconds and subbsecond nanoseconds.
pub const fn nanos_to_parts(nanos: u128) -> (u64, u32) {
    if nanos > UnixTimestamp::MAX_REPRESENTABLE_NANOSECONDS {
        return (u64::MAX, u32::MAX);
    }

    let secs = nanos_to_secs(nanos);

    if secs > u64::MAX as u128 {
        const MAX_NANOSECONDS_CARRY: u128 =
            u64::MAX as u128 * UnixTimestamp::NANOSECONDS_PER_SECOND;

        (u64::MAX, (nanos - MAX_NANOSECONDS_CARRY) as u32)
    } else {
        (secs as u64, (nanos % 1_000_000_000) as u32)
    }
}

/// Nanoseconds to seconds.
const fn nanos_to_secs(nanos: u128) -> u128 {
    nanos / 1_000_000_000
}

pub const fn parts_to_nanos(seconds: u64, subsec_nanos: u32) -> u128 {
    (seconds as u128 * 1_000_000_000) + subsec_nanos as u128
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
    fn max_const_is_actually_max() {
        let ts = UnixTimestamp::MAX;
        let (secs, subsec_nanos) = ts.to_parts();
        assert_eq!(
            u64::MAX,
            secs,
            "expected secs ({secs}) to be u64::MAX ({})",
            u64::MAX
        );
        assert_eq!(
            u32::MAX,
            subsec_nanos,
            "expected subsec_nanos ({subsec_nanos}) to be u32::MAX ({})",
            u32::MAX
        );
    }

    #[test]
    fn ts_to_millis() {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let unix_ts = UnixTimestamp::from_parts(ts.as_secs(), ts.subsec_nanos());
        let ts_millis = ts.as_millis() as u64;
        let unix_millis = unix_ts.as_millis_saturating();
        assert_eq!(
            ts_millis, unix_millis,
            "expected ts_millis ({ts_millis}) to equal unix_millis ({unix_millis})"
        );
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
