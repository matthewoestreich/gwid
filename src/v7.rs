// UUID VERSION 7
//
// https://www.rfc-editor.org/info/rfc9562/#section-5.7
//
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           unix_ts_ms                          |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          unix_ts_ms           |  ver  |       rand_a          |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |var|                        rand_b                             |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                            rand_b                             |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
// BIT LAYOUT:
// [000000000000000000000000000000000000000000000000] [0111] [000000000000] [10] [00000000000000000000000000000000000000000000000000000000000000]
// |<-------------------- unix_ts_ms -------------->| | ver | | random_a | |var| |<------------------------ random_b -------------------------->|
//
// unix_ts_ms:
//     48-bit big-endian unsigned number of the Unix Epoch timestamp in milliseconds as per Section 6.1. Occupies bits 0 through 47 (octets 0-5).
// ver:
//     The 4-bit version field as defined by Section 4.2, set to 0b0111 (7). Occupies bits 48 through 51 of octet 6.
// rand_a:
//     12 bits of pseudorandom data to provide uniqueness as per Section 6.9 and/or optional constructs to guarantee additional monotonicity as per Section 6.2. Occupies bits 52 through 63 (octets 6-7).
// var:
//     The 2-bit variant field as defined by Section 4.1, set to 0b10. Occupies bits 64 and 65 of octet 8.
// rand_b:
//     The final 62 bits of pseudorandom data to provide uniqueness as per Section 6.9 and/or an optional counter to guarantee additional monotonicity as per Section 6.2. Occupies bits 66 through 127 (octets 8-15).

use crate::CLEAR_VARIANT_MASK;
use crate::CLEAR_VERSION_MASK;
use crate::VARIANT_SHIFT;
use crate::VERSION_SHIFT;
use rand::RngExt;
use std::error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH};

static CONTEXT: LazyLock<Mutex<UUIDv7Context>> =
    LazyLock::new(|| Mutex::new(UUIDv7Context { ts: 0, a: 0, b: 0 }));

const RANDOM_A_BITS: u8 = 12;
const RANDOM_B_BITS: u8 = 62;
const TIMESTAMP_BITS: u8 = 48;
const RANDOM_A_TOTAL_VALUES: u16 = 1 << RANDOM_A_BITS;
/// 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00001111 11111111
const RANDOM_A_MAX: u16 = RANDOM_A_TOTAL_VALUES - 1;
/// 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111
const RANDOM_B_MAX: u64 = (1 << RANDOM_B_BITS) - 1;
/// 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000 11111111 11111111 11111111 11111111 11111111 11111111
const TIMESTAMP_MAX: u64 = (1 << TIMESTAMP_BITS) - 1;
const TIMESTAMP_SHIFT: u8 = 80;
const RANDOM_A_SHIFT: u128 = 64;
const VERSION: u8 = 7; // 0b111
const VARIANT: u8 = 2; // 0b10

/// To hold global state for monotonic id generation
#[derive(Debug)]
struct UUIDv7Context {
    /// previous timestamp (uses ms)
    pub(crate) ts: u128,
    /// previous random_a
    pub(crate) a: u16,
    /// previous random_b
    pub(crate) b: u64,
}

/// Generates a UUIDv7 using the current Unix timestamp and a cryptographically
/// secure random number generator.
///
/// **This method is not strictly monotonic!** If you need strict monotonicity,
/// use the `generate_strict()` method!
///
/// If multiple IDs are generated in the same sub-ms (nanosecond) precision,
/// they are not guaranteed to be generated in order!
///
/// The 48-bit timestamp is combined with sub-millisecond timestamp precision
/// to populate the 12-bit `rand_a` field, following Method 3 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
/// The remaining 62 bits are populated using cryptographically secure randomness.
///
/// # Errors
///
/// Returns [UUIDv7Error::SystemTimeError] if the system time is earlier than [UNIX_EPOCH].
pub fn generate() -> Result<String, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = rand::rng().random::<u64>();
    Ok(generate_with(timestamp, random_b))
}

/// Generates a UUIDv7 using the current Unix timestamp and a cryptographically
/// secure random number generator.
///
/// The 48-bit timestamp is combined with sub-millisecond timestamp precision
/// to populate the 12-bit `rand_a` field, following Method 3 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
/// The remaining 62 bits are populated using cryptographically secure randomness.
///
/// In order to be strictly monotonic, we incorporate Method 2 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
///
/// A process-wide monotonic state ensures that generated UUIDs are strictly
/// increasing, even when multiple threads generate UUIDs concurrently or the
/// system clock moves backwards.
///
/// # Errors
///
/// Returns [UUIDv7Error::SystemTimeError] if the system time is earlier than
/// [UNIX_EPOCH], or [UUIDv7Error::Exhausted] if both random fields
/// and the timestamp cannot be advanced while preserving monotonicity; eg. everything
/// overflowed.
pub fn generate_strict() -> Result<String, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = rand::rng().random::<u64>();
    generate_strict_with(timestamp, random_b)
}

/// **This method is not strictly monotonic!** If you need strict monotonicity,
/// use the `generate_strict()` method!
///
/// If multiple IDs are generated in the same sub-ms (nanosecond) precision,
/// they are not guaranteed to be generated in order!
///
/// The 48-bit timestamp is combined with sub-millisecond timestamp precision
/// to populate the 12-bit `rand_a` field, following Method 3 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
/// The remaining 62 bits are populated using cryptographically secure randomness.
fn generate_with(timestamp: Duration, random_b: u64) -> String {
    let unix_ts_ms = timestamp.as_millis();
    let random_a = derive_random_a(timestamp);
    uuid_from_parts(unix_ts_ms, random_a, random_b)
}

/// Enorce strict monotonicity across process.
///
/// The 48-bit timestamp is combined with sub-millisecond timestamp precision
/// to populate the 12-bit `rand_a` field, following Method 3 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
/// The remaining 62 bits are populated using cryptographically secure randomness.
///
/// In order to be strictly monotonic, we incorporate Method 2 described in
/// [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2).
fn generate_strict_with(timestamp: Duration, mut random_b: u64) -> Result<String, UUIDv7Error> {
    let mut unix_ts_ms = timestamp.as_millis();
    let mut random_a = derive_random_a(timestamp);

    let mut ctx = CONTEXT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    ensure_monotonicity(&mut unix_ts_ms, &mut random_a, &mut random_b, &mut ctx)?;

    Ok(uuid_from_parts(unix_ts_ms, random_a, random_b))
}

/// Generate `random_a` bits using additional clock  precision as to satisfy monotonic
/// constraints following **Method 3** described in [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2)
fn derive_random_a(from_timestamp: Duration) -> u16 {
    let sub_ms_nanos = from_timestamp.subsec_nanos() % 1_000_000;
    ((RANDOM_A_TOTAL_VALUES as u32 * sub_ms_nanos) / 1_000_000) as u16
}

fn uuid_from_parts(unix_ts_ms: u128, random_a: u16, random_b: u64) -> String {
    let uuid = (unix_ts_ms << TIMESTAMP_SHIFT) & CLEAR_VERSION_MASK
        | ((VERSION as u128) << VERSION_SHIFT)
        | ((random_a as u128) << RANDOM_A_SHIFT) & CLEAR_VARIANT_MASK
        | ((VARIANT as u128) << VARIANT_SHIFT)
        | (random_b as u128);

    crate::add_hyphens(uuid)
}

/// - Enforces nondecreasing time.
/// - Only need monotonicity if timestamps were in same sub-ms precision,
///   in which case, only `random_b` is incremented.
/// - Handles overflow.
/// - Updates global context.
fn ensure_monotonicity(
    unix_ts_ms: &mut u128,
    random_a: &mut u16,
    random_b: &mut u64,
    ctx: &mut UUIDv7Context,
) -> Result<(), UUIDv7Error> {
    // Enforce nondecreasing time.
    *unix_ts_ms = ctx.ts.max(*unix_ts_ms);

    // Only need monotonicity if timestamps were in same sub-ms precision.
    if *unix_ts_ms == ctx.ts && *random_a == ctx.a {
        // Prevent sub-ms precision from decreasing
        *random_a = ctx.a.max(*random_a);

        // Only need to advance rand_b when rand_a has not advanced
        // and rand_b is not already greater than its previous value.
        if *random_a == ctx.a && *random_b <= ctx.b {
            if ctx.b < RANDOM_B_MAX {
                *random_b = (*random_b).max(ctx.b + 1);
            } else if ctx.a < RANDOM_A_MAX {
                // rand_b is exhausted. Advance rand_a.
                *random_a = ctx.a + 1;
                *random_b = 0;
            } else if ctx.ts < TIMESTAMP_MAX as u128 {
                // Both random fields are exhausted. Advance the timestamp.
                *unix_ts_ms = ctx.ts + 1;
                *random_a = 0;
                *random_b = 0;
            } else {
                return Err(UUIDv7Error::Exhausted);
            }
        }
    }

    ctx.ts = *unix_ts_ms;
    ctx.a = *random_a;
    ctx.b = *random_b;

    Ok(())
}

#[derive(Debug)]
pub enum UUIDv7Error {
    Exhausted,
    SystemTimeError(SystemTimeError),
}

impl Display for UUIDv7Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        use UUIDv7Error::*;
        match self {
            Exhausted => write!(
                f,
                "Both random fields, as well as the timestamp, have been exhausted! Rethink your threading strategy!"
            ),
            SystemTimeError(err) => write!(f, "System time error {err}"),
        }
    }
}

impl From<SystemTimeError> for UUIDv7Error {
    fn from(error: SystemTimeError) -> Self {
        Self::SystemTimeError(error)
    }
}

impl error::Error for UUIDv7Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::SystemTimeError(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    fn fresh_ctx() -> UUIDv7Context {
        UUIDv7Context { ts: 0, a: 0, b: 0 }
    }

    #[test]
    fn it_works() {
        let _uuid = crate::v7::generate_strict().unwrap();
    }

    #[test]
    #[ignore]
    // This test is just for me to watch creation.
    // `clear && cargo nextest run generate_until_duplicate_or_manually_stopped --run-ignored only --no-capture`
    fn generate_until_duplicate_or_manually_stopped() {
        use std::collections::HashSet;
        let mut uuids = HashSet::new();
        let mut prev: u128 = 0;

        loop {
            // so we dont run out of RAM ...
            if uuids.len() > 100_000_000 {
                uuids.clear();
                println!("RESET UUID SET");
                std::thread::sleep(Duration::from_secs(3));
            }

            let current_id = generate_strict().unwrap();
            let current_id_u128 = crate::uuid_to_u128(&current_id);

            assert!(
                uuids.insert(current_id_u128),
                "duplicate ID encountered! {current_id}"
            );
            assert!(
                current_id_u128 > prev,
                "ids should be stricty increasing! current={current_id_u128} is less than previous={prev}"
            );

            prev = current_id_u128;
            println!("{current_id}");
        }
    }

    #[test]
    fn monotonic_generation() {
        let mut previous: u128 = 0;

        for _ in 0..100_000 {
            let uuid = generate_strict().unwrap().replace("-", "");
            let current = u128::from_str_radix(&uuid, 16).unwrap();
            assert!(
                current > previous,
                "expected monotonic generation (current should be > previous) : previous={previous} ; current={current}"
            );
            previous = current;
        }
    }

    #[test]
    fn multiple_ids_same_ms() {
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

        let a = generate_strict_with(ts, rand::rng().random::<u64>()).unwrap();
        let b = generate_strict_with(ts, rand::rng().random::<u64>()).unwrap();

        let a_u128 = crate::uuid_to_u128(&a);
        let b_u128 = crate::uuid_to_u128(&b);

        println!("a_uuid={a} a_u128={a_u128}\nb_uuid={b} b_u128={b_u128}",);
        assert!(
            a_u128 < b_u128,
            "expected id generated first to be less than id generated second : first={a_u128} ; second={b_u128}",
        );
        println!("{}", "-".repeat(100));
    }

    #[test]
    fn example_from_rfc() {
        let expected: u16 = 1870;

        // 1 Jan 2023 12:34:56.1234567
        let jan_1_2023_12_34_56_1234567 = UNIX_EPOCH + Duration::new(1_672_576_496, 123_456_700);

        let duration = jan_1_2023_12_34_56_1234567
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let _timestamp = duration.as_millis();
        let random_a = crate::v7::derive_random_a(duration);

        assert_eq!(
            expected, random_a,
            "expected = {expected} ; got = {random_a}"
        );
    }

    #[test]
    fn uuid_has_correct_format() {
        let uuid = generate_strict_with(Duration::from_millis(1_700_000_000_000), 12345).unwrap();

        assert_eq!(uuid.len(), 36);
        assert_eq!(uuid.chars().filter(|&c| c == '-').count(), 4);

        assert_eq!(&uuid[8..9], "-");
        assert_eq!(&uuid[13..14], "-");
        assert_eq!(&uuid[18..19], "-");
        assert_eq!(&uuid[23..24], "-");

        assert!(uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn uuid_has_correct_version_and_variant() {
        let uuid = generate_strict_with(Duration::from_millis(1_700_000_000_000), 12345).unwrap();

        // Version 7.
        assert_eq!(&uuid[14..15], "7");

        // RFC 9562 variant: binary 10xx.
        assert!(matches!(&uuid[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn timestamp_is_encoded_correctly() {
        let timestamp = 1_700_000_000_000u128;

        let uuid = generate_strict_with(Duration::from_millis(timestamp as u64), 12345).unwrap();

        let value = crate::uuid_to_u128(&uuid);
        let decoded_timestamp = (value >> TIMESTAMP_SHIFT) & TIMESTAMP_MAX as u128;

        assert_eq!(decoded_timestamp, timestamp);
    }

    #[test]
    fn random_a_uses_submillisecond_precision() {
        assert_eq!(derive_random_a(Duration::from_nanos(0)), 0);
        assert_eq!(derive_random_a(Duration::from_nanos(500_000)), 2048);
        assert_eq!(derive_random_a(Duration::from_nanos(999_999)), 4095);
    }

    #[test]
    fn random_a_is_nondecreasing() {
        let mut previous = 0;

        for nanos in (1_000..1_000_000).step_by(1_000) {
            let current = derive_random_a(Duration::from_nanos(nanos));
            assert!(current >= previous);
            previous = current;
        }
    }

    #[test]
    fn monotonicity_increments_random_b_when_timestamp_and_random_a_are_unchanged() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 100;
        ctx.b = 200;

        let mut timestamp = 100;
        let mut random_a = 100;
        let mut random_b = 150;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 100);
        assert_eq!(random_a, 100);
        assert_eq!(
            random_b, 201,
            "expected randomb_b to equal 201, got={random_b}"
        );
    }

    #[test]
    fn monotonicity_preserves_higher_random_a() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 100;
        ctx.b = 200;

        let mut timestamp = 100;
        let mut random_a = 101;
        let mut random_b = 50;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(random_a, 101);
        assert_eq!(random_b, 50);
    }

    #[test]
    fn monotonicity_preserves_higher_random_b_when_timestamp_and_random_a_match() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 100;
        ctx.b = 200;

        let mut timestamp = 101;
        let mut random_a = 100;
        let mut random_b = 300;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(random_b, 300);
    }

    #[test]
    fn monotonicity_does_not_increment_random_b_when_random_a_differs() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 100;
        ctx.b = 200;

        let mut timestamp = 100;
        let mut random_a = 101;
        let mut random_b = 50;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(random_b, 50);
    }

    #[test]
    fn monotonicity_clamps_backwards_timestamp() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 100;
        ctx.b = 200;

        let mut timestamp = 99;
        let mut random_a = 100;
        let mut random_b = 200;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 100);
        assert_eq!(random_a, 100);
        assert_eq!(random_b, 201);
    }

    #[test]
    fn monotonicity_allows_timestamp_to_advance() {
        let mut ctx = fresh_ctx();

        ctx.ts = 100;
        ctx.a = 4095;
        ctx.b = RANDOM_B_MAX;

        let mut timestamp = 101;
        let mut random_a = 10;
        let mut random_b = 20;

        ensure_monotonicity(&mut timestamp, &mut random_a, &mut random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 101);
        assert_eq!(random_a, 10);
        assert_eq!(random_b, 20);
    }

    #[test]
    // This launches 8 threads, each generating 10,000 UUIDs simultaneously, then verifies that all 80,000 UUIDs are unique.
    fn concurrent_generation_produces_unique_uuids() {
        use std::{
            collections::HashSet,
            sync::{Arc, Barrier},
            thread,
        };

        const THREADS: usize = 8;
        const UUIDS_PER_THREAD: usize = 10_000;

        let barrier = Arc::new(Barrier::new(THREADS));

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    // Synchronize threads so they begin generating together.
                    barrier.wait();
                    (0..UUIDS_PER_THREAD)
                        .map(|_| generate_strict().unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        let uuids: Vec<String> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();

        assert_eq!(uuids.len(), THREADS * UUIDS_PER_THREAD);
        let unique_uuids: HashSet<_> = uuids.iter().collect();
        assert_eq!(unique_uuids.len(), uuids.len());
    }

    #[test]
    // This specifically stresses monotonicity logic when multiple threads generate UUIDs using the exact same timestamp.
    fn concurrent_generation_with_same_timestamp_is_unique() {
        use std::{
            collections::HashSet,
            sync::{Arc, Barrier},
            thread,
        };

        const THREADS: usize = 8;
        const UUIDS_PER_THREAD: usize = 10_000;

        let barrier = Arc::new(Barrier::new(THREADS));
        let timestamp = Duration::from_millis(1_700_000_000_000);

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    barrier.wait();

                    (0..UUIDS_PER_THREAD)
                        .map(|_| generate_strict_with(timestamp, 0).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        let uuids: Vec<String> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();

        let uuids_len = uuids.len();
        let expect_len = THREADS * UUIDS_PER_THREAD;

        assert_eq!(
            uuids_len, expect_len,
            "expected uuids.len={uuids_len} and expected_len={expect_len} to be equal"
        );

        let unique_uuids: HashSet<_> = uuids.iter().collect();
        let unique_uuids_len = unique_uuids.len();

        assert_eq!(
            unique_uuids_len, uuids_len,
            "expected unique_uuids_len={unique_uuids_len} to equal uuids_len={uuids_len}"
        );
    }
}
