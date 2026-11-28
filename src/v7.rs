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
//      - If disabled, use MathRandom (xorshift128 implementation)
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

use crate::CLEAR_VARIANT_MASK;
use crate::CLEAR_VERSION_MASK;
use crate::Uuid;
use crate::VARIANT_SHIFT;
use crate::VERSION_SHIFT;
use crate::builder;
use crate::builder::Version7;
use crate::time;
use rand::RngExt;
use rand::distr::{Distribution, StandardUniform};
use std::cell::{RefCell, RefMut};
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

const RANDOM_A_BITS: u32 = 12;
const RANDOM_B_BITS: u32 = 62;
const TIMESTAMP_BITS: u32 = 48;
const RANDOM_A_MAX: u16 = 1 << RANDOM_A_BITS;
const RANDOM_A_MASK: u16 = RANDOM_A_MAX - 1;
const RANDOM_B_MASK: u64 = (1 << RANDOM_B_BITS) - 1;
const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;
const TIMESTAMP_SHIFT: u32 = 80;
const RANDOM_A_SHIFT: u32 = 64;
const VERSION: u128 = 7;
const VARIANT: u128 = 2;

thread_local! {
    static CONTEXT_LOCAL: RefCell<ContextV7> = const { RefCell::new(ContextV7::new()) };
}

static CONTEXT_GLOBAL: Mutex<ContextV7> = Mutex::new(ContextV7::new());

impl From<builder::Version7> for Uuid {
    fn from(options: builder::Version7) -> Self {
        let (seconds, subsec_nanos) = options.timestamp.to_parts();

        let mut context_guard = get_context(if options.global {
            ContextV7Scope::Global
        } else {
            ContextV7Scope::ThreadLocal
        });

        let (unix_ts_ms, rand_a, rand_b) =
            generate_timestamp_sequence(&options, &mut context_guard);

        let raw_uuid = ((unix_ts_ms as u128) << TIMESTAMP_SHIFT) & CLEAR_VERSION_MASK
            | (VERSION << VERSION_SHIFT)
            | ((rand_a as u128) << RANDOM_A_SHIFT) & CLEAR_VARIANT_MASK
            | (VARIANT << VARIANT_SHIFT)
            | (rand_b as u128);

        Uuid(raw_uuid.to_be_bytes())
    }
}

/// Returns (counter_value, seconds, subsec_nanos)
fn generate_timestamp_sequence(options: &Version7, context: &mut ContextGuard) -> (u64, u16, u64) {
    let (prev_ts_ms, prev_rand_a, prev_rand_b) = unpack_context(context.0);

    let curr_ts_ms = options.timestamp.as_millis_saturating();
    let curr_ts_nanos = options.timestamp.as_nanos();

    // Enforce nondecreasing millisecond timestamp.
    let mut unix_ts_ms = curr_ts_ms.max(prev_ts_ms);

    // Generate initial random/precision fields.
    let mut rand_a = if options.additional_precision {
        derive_random_a(curr_ts_nanos) & RANDOM_A_MASK
    } else {
        get_entropy::<u16>(options.crypto) & RANDOM_A_MASK
    };

    let mut rand_b = get_entropy::<u64>(options.crypto) & RANDOM_B_MASK;

    if unix_ts_ms == prev_ts_ms {
        if options.strict {
            // Strict mode: rand_a is the primary counter.
            if rand_a <= prev_rand_a {
                if prev_rand_a < RANDOM_A_MASK {
                    rand_a = prev_rand_a + 1;
                } else {
                    // rand_a exhausted: hold it and advance rand_b.
                    rand_a = RANDOM_A_MASK;

                    if prev_rand_b < RANDOM_B_MASK {
                        rand_b = prev_rand_b + 1;
                    } else if unix_ts_ms < TIMESTAMP_MASK {
                        // Both random fields exhausted: advance timestamp.
                        unix_ts_ms += 1;
                        rand_a = 0;
                        rand_b = 0;
                    } else {
                        panic!("all fields exhausted");
                    }
                }
            }
        } else if options.additional_precision {
            // Non-strict mode: preserve nondecreasing sub-ms precision.
            rand_a = rand_a.max(prev_rand_a);
        }
    }

    context.0 = pack_context(unix_ts_ms, rand_a, rand_b);

    (unix_ts_ms, rand_a, rand_b)
}

fn derive_random_a(ts_ms_as_nanos: u128) -> u16 {
    let sub_ms_nanos = ts_ms_as_nanos % time::NANOSECONDS_PER_MILLISECOND;
    ((RANDOM_A_MAX as u128 * sub_ms_nanos) / time::NANOSECONDS_PER_MILLISECOND) as u16
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

fn unpack_context(context: u128) -> (u64, u16, u64) {
    (
        (context >> (RANDOM_A_BITS + RANDOM_B_BITS)) as u64,
        ((context >> RANDOM_B_BITS) & RANDOM_A_MASK as u128) as u16,
        (context & RANDOM_B_MASK as u128) as u64,
    )
}

fn pack_context(unix_ts_ms: u64, random_a: u16, random_b: u64) -> u128 {
    (((unix_ts_ms & TIMESTAMP_MASK) as u128) << (RANDOM_A_BITS + RANDOM_B_BITS))
        | (((random_a & RANDOM_A_MASK) as u128) << RANDOM_B_BITS)
        | ((random_b & RANDOM_B_MASK) as u128)
}

#[derive(Default, Debug)]
struct ContextV7(pub(crate) u128);

impl ContextV7 {
    pub const fn new() -> Self {
        Self(0)
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
    use std::{thread, time::Duration};

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
        let mut prev: Uuid = Uuid::zero();
        let mut prev_ms_boundary: Uuid = Uuid::zero();

        loop {
            let v7 = UuidBuilder::version7().build();

            if prev.get_timestamp().0 < v7.get_timestamp().0 {
                println!(
                    "  prev_bound = {} | new_bound = {}",
                    prev_ms_boundary.as_hex(),
                    v7.as_hex()
                );
                assert!(prev_ms_boundary < v7);
                prev_ms_boundary = v7;
            }

            println!("{}", v7.as_hex());
            prev = v7;
        }
    }

    #[test]
    fn strict() {
        let mut prev: Uuid = Uuid::zero();
        let mut prev_ms_boundary: Uuid = Uuid::zero();

        loop {
            let v7 = UuidBuilder::version7().strict().build();

            if prev.get_timestamp().0 < v7.get_timestamp().0 {
                println!(
                    "  prev_bound = {} | new_bound = {}",
                    prev_ms_boundary.as_hex(),
                    v7.as_hex()
                );
                assert!(prev_ms_boundary < v7);
                prev_ms_boundary = v7;
            }

            println!("{}", v7.as_hex());
            assert!(prev < v7);
            prev = v7;
        }
    }

    #[test]
    fn uuid_crate_custom() {
        const RANDOM_A_BITS: u8 = 12;
        const RANDOM_B_BITS: u8 = 62;
        const TIMESTAMP_BITS: u8 = 48;
        const RANDOM_A_TOTAL_VALUES: u16 = 1 << RANDOM_A_BITS;
        /// 00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000111111111111
        const RANDOM_A_MAX: u16 = RANDOM_A_TOTAL_VALUES - 1;
        /// 00000000000000000000000000000000000000000000000000000000000000000011111111111111111111111111111111111111111111111111111111111111
        const RANDOM_B_MAX: u64 = (1 << RANDOM_B_BITS) - 1;
        /// 00000000000000000000000000000000000000000000000000000000000000000000000000000000111111111111111111111111111111111111111111111111
        const TIMESTAMP_MAX: u64 = (1 << TIMESTAMP_BITS) - 1;
        const TIMESTAMP_SHIFT: u8 = 80;
        const RANDOM_A_SHIFT: u128 = 64;
        const VERSION: u8 = 7; // 0b111
        const VARIANT: u8 = 2; // 0b10
        struct MyContext;

        impl uuid::ClockSequence for MyContext {
            type Output = u128;

            fn generate_sequence(&self, seconds: u64, subsec_nanos: u32) -> Self::Output {
                let ts = Duration::new(seconds, subsec_nanos);
                let sub_ms_nanos = ts.subsec_nanos() % 1_000_000;
                let random_a = ((4096_u64 * sub_ms_nanos as u64) / 1_000_000) as u16;

                0
            }
        }

        let u = uuid::Uuid::new_v7(uuid::Timestamp::now(MyContext));
        println!("{}", u.hyphenated());
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
