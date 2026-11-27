/// Maximum number of nanoseconds that can be represented when storing seconds as `u64` and subsecond nanoseconds as `u32`.
pub const MAX_REPRESENTABLE_NANOSECONDS: u128 = 18_446_744_073_709_551_619_294_967_295;
/// Max milliseconds represented in nanoseconds.
pub const MAX_MILLIS_EXPRESSED_AS_NANOS: u128 = 18_446_744_073_709_551_615_999_999;
/// Number of nanoseconds in a second.
pub const NANOSECONDS_PER_SECOND: u128 = 1_000_000_000;
/// Number of nanoseconds in a millisecond.
pub const NANOSECONDS_PER_MILLISECOND: u128 = 1_000_000;
/// How many 100 nanosecond ticks exist in a single second.
pub const NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND: u64 = 10_000_000;
/// Seconds between 1582-10-15 00:00:00 UTC and 1970-01-01 00:00:00 UTC
pub const GREGORIAN_TO_UNIX_SECS: u64 = 12_219_292_800;
/// In 100-nanosecond ticks (common in UUID v1/v6 and Windows FILETIME math)
pub const GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS: u64 = 122_192_928_000_000_000;

pub fn nanos_clamped_to_u64_millis(nanos: u128) -> u128 {
    nanos.min(MAX_MILLIS_EXPRESSED_AS_NANOS)
}

/// Convert nanos to millis saturated.
pub fn nanos_as_millis_saturating(nanos: u128) -> u64 {
    (nanos_clamped_to_u64_millis(nanos) / 1_000_000) as u64
}

/// Nanoseconds to seconds and subbsecond nanoseconds.
pub const fn nanos_to_parts(nanos: u128) -> (u64, u32) {
    if nanos > MAX_REPRESENTABLE_NANOSECONDS {
        return (u64::MAX, u32::MAX);
    }

    let secs = nanos_to_secs(nanos);

    if secs > u64::MAX as u128 {
        const MAX_NANOSECONDS_CARRY: u128 = u64::MAX as u128 * NANOSECONDS_PER_SECOND;

        (u64::MAX, (nanos - MAX_NANOSECONDS_CARRY) as u32)
    } else {
        (secs as u64, (nanos % 1_000_000_000) as u32)
    }
}

/// Nanoseconds to seconds.
pub const fn nanos_to_secs(nanos: u128) -> u128 {
    nanos / 1_000_000_000
}

pub const fn parts_to_nanos(seconds: u64, subsec_nanos: u32) -> u128 {
    (seconds as u128 * 1_000_000_000) + subsec_nanos as u128
}
