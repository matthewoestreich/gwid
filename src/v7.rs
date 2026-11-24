// # UUID VERSION 7
//
// https://www.rfc-editor.org/info/rfc9562/#section-5.7
//
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
//
// # FIELDS
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
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
//
// # UUID VERSION 7 FIELD DEFINITIONS
//
// ## unix_ts_ms:
//     48-bit big-endian unsigned number of the Unix Epoch timestamp in milliseconds as per Section 6.1. Occupies bits 0 through 47 (octets 0-5).
// ## ver:
//     The 4-bit version field as defined by Section 4.2, set to 0b0111 (7). Occupies bits 48 through 51 of octet 6.
// ## rand_a:
//     12 bits of pseudorandom data to provide uniqueness as per Section 6.9 and/or optional constructs to guarantee additional monotonicity as per Section 6.2. Occupies bits 52 through 63 (octets 6-7).
// ## var:
//     The 2-bit variant field as defined by Section 4.1, set to 0b10. Occupies bits 64 and 65 of octet 8.
// ## rand_b:
//     The final 62 bits of pseudorandom data to provide uniqueness as per Section 6.9 and/or an optional counter to guarantee additional monotonicity as per Section 6.2. Occupies bits 66 through 127 (octets 8-15).
//
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
//
// # UUID VERSION 7 BIT LAYOUT:
//
// Bit Count |                          48                    | |  4   |  |     12     | | 2   | |                            62                                |
//           |000000000000000000000000000000000000000000000000| | 0000 |  |000000000000| | 00  | |00000000000000000000000000000000000000000000000000000000000000|
//   Field   |<-------------------- unix_ts_ms -------------->| | ver  |  |  random_a  | | var | |<------------------------ random_b -------------------------->|
//
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
//
// # CRATE DEFINED TERMINOLOGY
//
// ## Strict Mode :
//      - The uuid rfc only requires timestamps to be monotonic at the millisecond level.
//      - In strict mode, we ensure timestamps are strictly monotonic, at every timestamp precision.
//
// ## Additional Precision Mode :
//      - If enabled, uses `random_a` field (12 bits) for sub-millisecond timestamp precision.
//      - If disabled, the `random_a` field (12 bits) are added to the MSB (30 bits) of random_b
//
// ## Crypto :
//      - If enabled, use crypto secure randomness
//      - If disabled, use MathRandom
//
// ## Global :
//      - If enabled, uses shared context (coontext wrapped in Mutex)
//      - If disabled, uses **thread-local** context.
//
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
//
// # CRATE DEFINED BEHAVIOR
//
// ## When in strict mode:
//      - Timestamps should always be, at minimum, **thread-local** monotonic, even inside same ms
//      timestamp
//      - If GLOBAL is enabled, we don't have to do anything bc our context is now in a Mutex.
//
// ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

use crate::Uuid;
use crate::VARIANT_SHIFT;
use crate::VERSION_SHIFT;
use crate::builder;
use crate::builder::Version7;
use crate::timestamp::UnixTimestamp;
use crate::timestamp::nanos_as_millis_saturating;
use rand::RngExt;
use rand::distr::Distribution;
use rand::distr::StandardUniform;
use std::cell::RefCell;
use std::cell::RefMut;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Mutex;
use std::sync::MutexGuard;

const RANDOM_A_BITS: u32 = 12;
const RANDOM_B_HIGH_BITS: u32 = 30;
const TIMESTAMP_BITS: u32 = 48;
const COUNTER_BITS: u32 = RANDOM_A_BITS + RANDOM_B_HIGH_BITS;
const COUNTER_LOW_MASK: u64 = (1u64 << RANDOM_B_HIGH_BITS) - 1;
const COUNTER_MAX: u64 = (1u64 << COUNTER_BITS) - 1;
const RESEED_MASK: u64 = (1u64 << (COUNTER_BITS - 1)) - 1;
const TIMESTAMP_MAX: u64 = (1 << TIMESTAMP_BITS) - 1;
const TIMESTAMP_SHIFT: u32 = 80;
const RANDOM_A_SHIFT: u32 = 64;
const COUNTER_SHIFT: u32 = 32;
const VERSION: u128 = 7;
const VARIANT: u128 = 2;

thread_local! {
    static CONTEXT_LOCAL: RefCell<ContextV7> = const { RefCell::new(ContextV7::new()) };
}

static CONTEXT_GLOBAL: Mutex<ContextV7> = Mutex::new(ContextV7::new());

impl From<builder::Version7> for Uuid {
    fn from(options: builder::Version7) -> Self {
        let (seconds, subsec_nanos) = options.timestamp.to_parts();

        let mut context = get_context(if options.global {
            ContextV7Scope::Global
        } else {
            ContextV7Scope::ThreadLocal
        });

        let (counter, seconds, subsec_nanos) =
            generate_timestamp_sequence(seconds, subsec_nanos, &options, &mut context);

        let next_timestamp = UnixTimestamp::from_parts(seconds, subsec_nanos);

        context.timestamp = next_timestamp;
        context.counter = counter;

        let timestamp_millis = (next_timestamp.as_millis_saturating() & TIMESTAMP_MAX) as u128;
        let random_a = (counter >> RANDOM_B_HIGH_BITS) as u128;
        let counter = (counter & COUNTER_LOW_MASK) as u128;
        let random_b = get_entropy::<u32>(options.crypto) as u128;

        let raw_uuid = (timestamp_millis << TIMESTAMP_SHIFT)
            | (VERSION << VERSION_SHIFT)
            | (random_a << RANDOM_A_SHIFT)
            | (VARIANT << VARIANT_SHIFT)
            | (counter << COUNTER_SHIFT)
            | random_b;

        Uuid(raw_uuid.to_be_bytes())
    }
}

/// Returns (counter_value, seconds, subsec_nanos)
fn generate_timestamp_sequence(
    seconds: u64,
    subsec_nanos: u32,
    options: &Version7,
    context: &mut ContextGuard,
) -> (u64, u64, u32) {
    let curr_unix_nanos = crate::timestamp::parts_to_nanos(seconds, subsec_nanos);
    let prev_timestamp = context.timestamp;
    let prev_nanos = prev_timestamp.as_nanos();

    // Enforce nondecreasing time.
    let mut next_timestamp = curr_unix_nanos.max(prev_nanos);

    let prev_millis = nanos_as_millis_saturating(prev_nanos);
    let next_millis = nanos_as_millis_saturating(next_timestamp);
    let precision_params = calculate_precision_parameters(options.additional_precision);

    // We are in a new ms, reseed counter.
    let next_counter = if next_millis > prev_millis {
        reseed_counter(next_timestamp, options.crypto, &precision_params)
    }
    // We are in the same ms and in strict mode, ensure monotonicity within sub-ms precision.
    else if options.strict && next_millis == prev_millis {
        let mut next = increment_counter(context.counter, next_timestamp, &precision_params);

        // If the counter overflowed, increment timestamp by 1ms.
        if next > COUNTER_MAX {
            increment_unix_nanos_by_millis(&mut next_timestamp, 1);
            next = reseed_counter(next_timestamp, options.crypto, &precision_params);

            // If the timestamp overflowed as well, rethink ur threading strategy!
            if next_timestamp > TIMESTAMP_MAX as u128 {
                panic!("both counter and timestamp overflow. rethink your threading strategy!");
            }
        }

        next
    }
    // We are in the same ms but not in strict mode (do not need sub-ms monotonicity), generate new random bits.
    else {
        get_entropy::<u64>(options.crypto) & RESEED_MASK
    };

    let (next_secs, next_subsec_nanos) = crate::timestamp::nanos_to_parts(next_timestamp);
    (next_counter, next_secs, next_subsec_nanos)
}

fn increment_counter(
    counter: u64,
    next_timestamp: u128,
    precision_params: &PrecisionParameters,
) -> u64 {
    let next = counter + 1;
    let next_with_precision = apply_precision(next, next_timestamp, precision_params);
    std::cmp::max(next, next_with_precision)
}

fn increment_unix_nanos_by_millis(unix_nanos: &mut u128, millis: u32) {
    let millis_as_nanos =
        (millis as u128).saturating_mul(UnixTimestamp::NANOSECONDS_PER_MILLISECOND);
    *unix_nanos = unix_nanos.saturating_add(millis_as_nanos);
}

fn reseed_counter(
    next_timestamp: u128,
    is_crypto: bool,
    precision_params: &PrecisionParameters,
) -> u64 {
    let reseeded_counter = get_entropy::<u64>(is_crypto) & RESEED_MASK;
    apply_precision(reseeded_counter, next_timestamp, precision_params)
}

fn apply_precision(counter: u64, next_timestamp: u128, params: &PrecisionParameters) -> u64 {
    if params.bits == 0 {
        return counter;
    }
    let additional = ((next_timestamp % 1_000_000) * (1u128 << params.bits)) / 1_000_000;
    (counter & params.mask) | ((additional as u64) << params.shift)
}

fn calculate_precision_parameters(is_additional_precision: bool) -> PrecisionParameters {
    let bits = if is_additional_precision {
        RANDOM_A_BITS as usize
    } else {
        0
    };

    let counter_bits_remaining = COUNTER_BITS as usize - bits;

    PrecisionParameters {
        bits,
        mask: (1u64 << counter_bits_remaining) - 1,
        shift: counter_bits_remaining as u32,
    }
}

fn get_entropy<T>(is_crypto: bool) -> T
where
    T: crate::rand::RandomValue,
    StandardUniform: Distribution<T>,
{
    if is_crypto {
        rand::rng().random()
    } else {
        crate::rand::random()
    }
}

#[derive(Debug, Clone, Copy)]
struct PrecisionParameters {
    bits: usize,
    mask: u64,
    shift: u32,
}

#[derive(Default, Debug)]
struct ContextV7 {
    pub timestamp: UnixTimestamp,
    pub counter: u64,
}

impl ContextV7 {
    pub const fn new() -> Self {
        Self {
            timestamp: UnixTimestamp(0),
            counter: 0,
        }
    }
}

enum ContextGuard<'a> {
    Global(MutexGuard<'a, ContextV7>),
    ThreadLocal(RefMut<'a, ContextV7>),
}

impl Deref for ContextGuard<'_> {
    type Target = ContextV7;

    #[inline]
    fn deref(&self) -> &Self::Target {
        match self {
            ContextGuard::Global(g) => g,
            ContextGuard::ThreadLocal(l) => l,
        }
    }
}

impl DerefMut for ContextGuard<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            ContextGuard::Global(g) => g,
            ContextGuard::ThreadLocal(l) => l,
        }
    }
}

enum ContextV7Scope {
    Global,
    ThreadLocal,
}

#[inline]
fn get_context(scope: ContextV7Scope) -> ContextGuard<'static> {
    match scope {
        ContextV7Scope::Global => ContextGuard::Global(CONTEXT_GLOBAL.lock().unwrap()),
        ContextV7Scope::ThreadLocal => {
            // SAFETY: CONTEXT_LOCAL lives for the entire thread lifetime, and RefMut
            // is !Send/!Sync so it cannot outlive this thread.
            let borrow = CONTEXT_LOCAL.with(|cell| unsafe {
                let ptr = cell as *const RefCell<ContextV7>;
                (*ptr).borrow_mut()
            });
            ContextGuard::ThreadLocal(borrow)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{builder::UuidBuilder, print_bits, timestamp::UnixTimestamp};
    use std::thread;

    /*
    #[test]
    fn additional_precision_and_no_strict_mode_may_not_maintain_monotonicity_in_same_sub_ms_bucket()
    {
        let timestamp = UnixTimestamp::from_nanos(1795395060835011000);

        let options = Version7 {
            strict: false,
            additional_precision: true,
            timestamp,
            global: false,
            crypto: false,
        };

        let mut ctx = ContextV7 {
            ts_millis: timestamp.as_millis_saturating(),
            counter: COUNTER_MAX / 2,
        };

        let counter = 0;
        let (_, actual_counter) = ensure_monotonicity(&options, &mut ctx, counter);

        assert_eq!(actual_counter, 0);
    }
    */

    #[test]
    fn uuid_example_from_rfc() {
        //for _ in 0..1000 {
        loop {
            let v7 = UuidBuilder::version7().build();
            println!("{}", v7.as_hex());
            // println!();
        }
    }

    #[test]
    fn ensure_thread_local_monotonicity_when_multithreading() {
        const NUM_THREADS: usize = 12;
        const NUM_IDS_PER_THREAD: usize = 500_000;

        let mut handles = Vec::with_capacity(NUM_THREADS);

        for _ in 0..NUM_THREADS {
            handles.push(thread::spawn(move || {
                let mut ids = Vec::with_capacity(NUM_IDS_PER_THREAD);

                for _ in 0..NUM_IDS_PER_THREAD {
                    ids.push(UuidBuilder::version7().strict().build());
                }

                ids
            }))
        }

        for handle in handles {
            let results = handle.join().expect("failed to join handle");

            let mut prev = Uuid::zero();
            for id in results {
                assert!(prev < id);
                prev = id;
            }
        }
    }

    #[test]
    fn v7_single_thread() {
        fn new_v7() -> Uuid {
            UuidBuilder::version7().build()
        }

        let start = new_v7();
        let mut prev: Uuid = start;
        let mut prev_ms_boundary = start;
        let mut num_ids_created_in_same_ms = 0;

        let mut is_sub_ms_monotonic = true;

        for _ in 0..1_000_000 {
            let v7 = new_v7();
            // let hex = v7.as_hex();

            let v7_time = v7.get_timestamp().0;
            let prev_time = prev.get_timestamp().0;

            if v7_time > prev_time {
                //println!("- should be next ms!");
                println!(
                    "prev ms boundary      = {} | {}",
                    prev_ms_boundary.as_hex(),
                    prev_ms_boundary.as_u128()
                );
                println!("curr_ms_boundary      = {} | {}", v7.as_hex(), v7.as_u128());
                println!("curr_ms_boundary_time = {prev_time} -> {v7_time}");

                let is_monotonic_across_ms_bounds = v7 > prev_ms_boundary;
                println!("    curr > prev? {is_monotonic_across_ms_bounds}",);
                println!(
                    "    there were {num_ids_created_in_same_ms} ids created in this ms. were they all monotonic? {is_sub_ms_monotonic}"
                );
                assert!(
                    v7 > prev_ms_boundary,
                    "ms monotonicity is a requirement of v7!"
                );

                num_ids_created_in_same_ms = 0;
                is_sub_ms_monotonic = true;

                if !is_monotonic_across_ms_bounds {
                    println!("{}", "*".repeat(100));
                }

                println!();
                prev_ms_boundary = v7;
            } else if v7_time == prev_time {
                num_ids_created_in_same_ms += 1;
                // if we found a non mono sub-ms dont overwrite it.
                if is_sub_ms_monotonic {
                    is_sub_ms_monotonic = prev < v7;
                }
            } else {
                println!("\n\nTIME WENT BACKWARDS! \n\n");
            }

            // print!("{hex}");

            // if v7 <= prev {
            // print!("  *******");
            //println!(
            // "NOT MONO\n\tprev = ({} | {})\n\tcurr = ({} | {})",
            // prev.as_hex(),
            // prev.get_timestamp().1,
            // v7.as_hex(),
            // v7.get_timestamp().1,
            //);
            // }

            prev = v7;
            // println!();
        }
    }

    #[test]
    fn uuid_single_thread() {
        fn new_v7() -> uuid::Uuid {
            uuid::Uuid::now_v7()
        }

        let start = new_v7();
        let mut prev: uuid::Uuid = start;
        let mut prev_ms_boundary = start;
        let mut num_ids_created_in_same_ms = 0;

        let mut is_sub_ms_monotonic = true;

        for _ in 0..1_000_000 {
            let v7 = new_v7();
            // let hex = v7.as_hex();

            let v7_time = v7
                .get_timestamp()
                .map(|ts| {
                    let (seconds, nanos) = ts.to_unix();
                    seconds * 1_000 + u64::from(nanos / 1_000_000)
                })
                .unwrap();

            let prev_time = prev
                .get_timestamp()
                .map(|ts| {
                    let (seconds, nanos) = ts.to_unix();
                    seconds * 1_000 + u64::from(nanos / 1_000_000)
                })
                .unwrap();

            if v7_time > prev_time {
                //println!("- should be next ms!");
                println!(
                    "prev ms boundary = {} | {}",
                    prev_ms_boundary.as_hyphenated(),
                    prev_ms_boundary.as_u128()
                );
                println!(
                    "curr_ms_boundary = {} | {}",
                    v7.as_hyphenated(),
                    v7.as_u128()
                );

                let is_monotonic_across_ms_bounds = v7 > prev_ms_boundary;
                println!("    curr > prev? {is_monotonic_across_ms_bounds}",);
                println!("    there were {num_ids_created_in_same_ms} ids created in this ms");
                println!("    were they all monotonic? {is_sub_ms_monotonic}");
                assert!(
                    v7 > prev_ms_boundary,
                    "ms monotonicity is a requirement of v7!"
                );

                num_ids_created_in_same_ms = 0;
                is_sub_ms_monotonic = true;

                if !is_monotonic_across_ms_bounds {
                    println!("{}", "*".repeat(100));
                }

                println!();
                prev_ms_boundary = v7;
            } else if v7_time == prev_time {
                num_ids_created_in_same_ms += 1;
                // if we found a non mono sub-ms dont overwrite it.
                if is_sub_ms_monotonic {
                    is_sub_ms_monotonic = prev < v7;
                }
            } else {
                println!("\n\nTIME WENT BACKWARDS! \n\n");
            }

            // print!("{hex}");

            // if v7 <= prev {
            // print!("  *******");
            //println!(
            // "NOT MONO\n\tprev = ({} | {})\n\tcurr = ({} | {})",
            // prev.as_hex(),
            // prev.get_timestamp().1,
            // v7.as_hex(),
            // v7.get_timestamp().1,
            //);
            // }

            prev = v7;
            // println!();
        }
    }

    #[test]
    fn v7_multithread() {
        let mut handles = vec![];

        for i in 0..10 {
            handles.push(thread::spawn(move || {
                let mut prev: Uuid = Uuid::zero();

                for _ in 0..100_000 {
                    let v7 = UuidBuilder::version7()
                        .additional_precision()
                        .global()
                        .strict()
                        .build();
                    // println!("{}", v7.as_hex());

                    if v7 <= prev {
                        println!(
                            "thread {i} : NOT MONO\n\tprev = ({} | {})\n\tcurr = ({} | {})",
                            prev.as_hex(),
                            prev.get_timestamp().1,
                            v7.as_hex(),
                            v7.get_timestamp().1,
                        );
                    }

                    prev = v7;
                    // println!();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        /*
                let mut prev: Uuid = Uuid::zero();
                loop {
                    let v7 = UuidBuilder::version7().global().build();
                    println!("{}", v7.as_hex());
                    if v7 <= prev {
                        println!(
                            "\tNOT MONO | (prev={} @ {}) ; curr={}",
                            prev.as_hex(),
                            prev.as_u128(),
                            v7.as_u128()
                        );
                    }
                    prev = v7;
                    // println!();
                }
        */
    }
}
