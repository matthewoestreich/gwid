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
use crate::Uuid;
use crate::VARIANT_SHIFT;
use crate::VERSION_SHIFT;
use crate::builder;
use crate::builder::Version7;
use crate::print_bits;
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

// # BIT LAYOUT
//
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//
// [<- Bit 127]                                                                                                            [Bit 0 ->]
// [<----------------- 48 Bits -------------------->][< 12 bits ->][<-------- 30 bits ----------->][<---------- 32 bits ----------->]
// [000000000000000000000000000000000000000000000000][000000000000][000000000000000000000000000000][00000000000000000000000000000000]
// [<------- Timestamp, millisecond precision ----->][<- Sub-ms ->][<--------- counter ---------->][<--------- random bits -------->]
//                                                   [  precision ]
//                                                   [ or counter ]
//
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
//
// # DEFINITIONS
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

// When in strict mode:
//      - Timestamps should always be, at minimum, **thread-local** monotonic, even inside same ms
//      timestamp
//      - If GLOBAL is enabled, we don't have to do anything bc our context is now in a Mutex.
//
//

thread_local! {
    static CONTEXT_LOCAL: RefCell<ContextV7> = const { RefCell::new(ContextV7::new()) };
}

static CONTEXT_GLOBAL: Mutex<ContextV7> = Mutex::new(ContextV7::new());

const RANDOM_A_BITS: u32 = 12;
const RANDOM_B_HIGH_BITS: u32 = 30;
const TIMESTAMP_BITS: u32 = 48;
const COUNTER_BITS: u32 = RANDOM_A_BITS + RANDOM_B_HIGH_BITS;
const RANDOM_A_MAX: u64 = (1u64 << RANDOM_A_BITS) - 1;
const COUNTER_LOW_MASK: u64 = (1u64 << RANDOM_B_HIGH_BITS) - 1;
const COUNTER_MAX: u64 = (1u64 << COUNTER_BITS) - 1;
const RESEED_MASK: u64 = (1u64 << (COUNTER_BITS - 1)) - 1;
const TIMESTAMP_MAX: u64 = (1 << TIMESTAMP_BITS) - 1;
const TIMESTAMP_SHIFT: u32 = 80;
const RANDOM_A_SHIFT: u32 = 64;
const COUNTER_SHIFT: u32 = 32;
const VERSION: u128 = 7;
const VARIANT: u128 = 2;

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

        // print_bits(timestamp, "millis");
        //  print_bits(random_a, "random_a");
        //  print_bits(counter, "counter : 30 bits");
        // print_bits(random_b, "random_b : 32 bits");

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
    // prev_timestamp.advance(curr_unix_nanos)
    let next_timestamp = curr_unix_nanos.max(prev_nanos);

    // check if ms changed
    let prev_millis = nanos_as_millis_saturating(prev_nanos);
    let next_millis = nanos_as_millis_saturating(next_timestamp);

    let precision_params = calculate_precision_parameters(options.additional_precision);

    let next_counter = if next_millis > prev_millis {
        let reseeded_counter = get_entropy::<u64>(options.crypto) & RESEED_MASK;
        apply_precision(reseeded_counter, next_timestamp, precision_params)
    } else if next_millis == prev_millis && options.strict {
        let next = context.counter + 1;
        std::cmp::max(
            next,
            apply_precision(next, next_timestamp, precision_params),
        )
    } else {
        get_entropy::<u64>(options.crypto) & RESEED_MASK
    };

    let (next_secs, next_subsec_nanos) = crate::timestamp::nanos_to_parts(next_timestamp);

    (next_counter, next_secs, next_subsec_nanos)
}

fn apply_precision(counter: u64, next_timestamp: u128, params: PrecisionParameters) -> u64 {
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

    let counter_bits = COUNTER_BITS as usize;
    let counter_bits_remaining = counter_bits - bits;

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
    // NonNull<MyContext> automatically makes ContextGuard !Send and !Sync,
    // preventing it from ever escaping the thread that created it.
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

        for i in 0..100 {
            handles.push(thread::spawn(move || {
                let mut prev: Uuid = Uuid::zero();

                for _ in 0..100_000 {
                    let v7 = UuidBuilder::version7().additional_precision().build();
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

// ----------------- OLD ------------------------------------------------------------------------

/*
pub fn generate() -> Result<String, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = RNG.with_borrow_mut(|entropy_source| entropy_source.random());
    Ok(generate_with(timestamp, random_b))
}

pub fn generate_strict() -> Result<String, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = RNG.with_borrow_mut(|entropy_source| entropy_source.random());
    generate_strict_with(timestamp, random_b)
}

pub fn generate_raw() -> Result<u128, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = RNG.with_borrow_mut(|entropy_source| entropy_source.random());
    Ok(generate_raw_with(timestamp, random_b))
}

pub fn generate_strict_raw() -> Result<u128, UUIDv7Error> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let random_b = RNG.with_borrow_mut(|entropy_source| entropy_source.random());
    generate_strict_raw_with(timestamp, random_b)
}

/// We return the UUID as a tuple with four elements. Each element represents a "word",
/// which is equivalent to a row in the following diagram:
///
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                           unix_ts_ms                          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |          unix_ts_ms           |  ver  |       rand_a          |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |var|                        rand_b                             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                            rand_b                             |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
///
/// Each row corresponds to it's index in the tuple, starting from the top.
/// Eg; first row is `.0`, second row from top is `.1`....
pub fn uuid_to_words(uuid: &str) -> Result<(u32, u32, u32, u32), crate::GwidError> {
    let value = crate::decode(uuid)?;
    Ok((
        (value >> 96) as u32,
        (value >> 64) as u32,
        (value >> 32) as u32,
        value as u32,
    ))
}

/// Returns tuple of parts where each element is: (unix_ts_ms, random_a, random_b)
pub fn uuid_to_parts(uuid: &str) -> Result<(u128, u16, u64), crate::GwidError> {
    let value = crate::decode(uuid)?;
    let unix_ts_ms = (value >> TIMESTAMP_SHIFT) & TIMESTAMP_MAX as u128;
    let random_a = (value >> RANDOM_A_SHIFT) & RANDOM_A_MAX as u128;
    let random_b = value & RANDOM_B_MAX as u128;
    Ok((unix_ts_ms, random_a as u16, random_b as u64))
}

fn generate_raw_with(timestamp: Duration, random_b: u64) -> u128 {
    let unix_ts_ms = timestamp.as_millis() & TIMESTAMP_MAX as u128;
    let random_a = derive_random_a(timestamp) & RANDOM_A_MAX;
    let random_b = random_b & RANDOM_B_MAX;

    CONTEXT.with_borrow_mut(|ctx| ensure_monotonicity(unix_ts_ms, random_a, random_b, ctx));

    raw_uuid_from_parts(unix_ts_ms, random_a, random_b)
}

fn generate_with(timestamp: Duration, random_b: u64) -> String {
    crate::encode(generate_raw_with(timestamp, random_b))
}

fn generate_strict_raw_with(timestamp: Duration, random_b: u64) -> Result<u128, UUIDv7Error> {
    let unix_ts_ms = timestamp.as_millis() & TIMESTAMP_MAX as u128;
    let random_a = derive_random_a(timestamp) & RANDOM_A_MAX;
    let random_b = random_b & RANDOM_B_MAX;

    #[cfg(feature = "v7-global-monotonicity")]
    let (unix_ts_ms, random_a, random_b) = {
        let mut ctx = STRICT_CONTEXT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        ensure_strict_monotonicity(unix_ts_ms, random_a, random_b, &mut ctx)?
    };

    #[cfg(not(feature = "v7-global-monotonicity"))]
    let (unix_ts_ms, random_a, random_b) = CONTEXT
        .with_borrow_mut(|ctx| ensure_strict_monotonicity(unix_ts_ms, random_a, random_b, ctx))?;

    Ok(raw_uuid_from_parts(unix_ts_ms, random_a, random_b))
}

fn generate_strict_with(timestamp: Duration, random_b: u64) -> Result<String, UUIDv7Error> {
    Ok(crate::encode(generate_strict_raw_with(
        timestamp, random_b,
    )?))
}

/// Generate `random_a` bits using additional clock  precision as to satisfy monotonic
/// constraints following **Method 3** described in [RFC 9562, Section 6.2](https://www.rfc-editor.org/rfc/rfc9562.html#section-6.2)
fn derive_random_a(from_timestamp: Duration) -> u16 {
    let sub_ms_nanos = from_timestamp.subsec_nanos() % 1_000_000;
    ((RANDOM_A_TOTAL_VALUES as u64 * sub_ms_nanos as u64) / 1_000_000) as u16
}

/// ----------------------------------------------------------------------------
/// **IMPORTANT** this method assumes all parts have been properly masked!!!!
/// We do not apply masks to any part.
/// ----------------------------------------------------------------------------
fn raw_uuid_from_parts(unix_ts_ms: u128, random_a: u16, random_b: u64) -> u128 {
    (unix_ts_ms << TIMESTAMP_SHIFT) & CLEAR_VERSION_MASK
        | ((VERSION as u128) << VERSION_SHIFT)
        | ((random_a as u128) << RANDOM_A_SHIFT) & CLEAR_VARIANT_MASK
        | ((VARIANT as u128) << VARIANT_SHIFT)
        | (random_b as u128)
}

/// This method packs values into a u128, it is meant to handle global/shared context updates.
///
/// Our context/state is a packed u128 where (starting from left going -> right):
///
/// | Bit Count | Bit Range | Field      | Purpose                             |
/// | --------- | --------- | ---------- | ----------------------------------- |
/// | 6 bits    | 0-5       | <empty>    | No use                              |
/// | 48 bits   | 6-53      | unix-ts-ms | Unix timestamp                      |
/// | 12 bits   | 54-65     | random_a   | Sub-millisecond timestmap precision |
/// | 62 bits   | 66-127    | random_b   | Entropy                             |
///
/// BIT LAYOUT:
/// [000000][000000000000000000000000000000000000000000000000][000000000000][00000000000000000000000000000000000000000000000000000000000000]
/// [unused][<---------------- unix_ts_ms ------------------>][<-random_a->][<-------------------- radom_b ------------------------------->]
fn pack_context(ts: u128, a: u16, b: u64) -> u128 {
    ((ts & TIMESTAMP_MAX as u128) << (RANDOM_A_BITS + RANDOM_B_BITS))
        | (((a & RANDOM_A_MAX) as u128) << RANDOM_B_BITS)
        | ((b & RANDOM_B_MAX) as u128)
}

/// This method unpacks values from a u128, it is meant to handle reading global/shared context updates.
///
/// Our context/state is a packed u128 where (starting from left going -> right):
///
/// | Bit Count | Bit Range | Field      | Purpose                             |
/// | --------- | --------- | ---------- | ----------------------------------- |
/// | 6 bits    | 0-5       | <empty>    | No use                              |
/// | 48 bits   | 6-53      | unix-ts-ms | Unix timestamp                      |
/// | 12 bits   | 54-65     | random_a   | Sub-millisecond timestmap precision |
/// | 62 bits   | 66-127    | random_b   | Entropy                             |
///
/// BIT LAYOUT:
/// [000000][000000000000000000000000000000000000000000000000][000000000000][00000000000000000000000000000000000000000000000000000000000000]
/// [unused][<---------------- unix_ts_ms ------------------>][<-random_a->][<-------------------- radom_b ------------------------------->]
fn unpack_context(value: u128) -> (u128, u16, u64) {
    let ts = value >> (RANDOM_A_BITS + RANDOM_B_BITS);
    let a = ((value >> RANDOM_B_BITS) & RANDOM_A_MAX as u128) as u16;
    let b = (value & RANDOM_B_MAX as u128) as u64;
    (ts, a, b)
}

/// For use in non-strict monotonicity.
///   - Only handles clock drift
fn ensure_monotonicity(
    unix_ts_ms: u128,
    random_a: u16,
    random_b: u64,
    packed_ctx: &mut u128,
) -> (u128, u16) {
    let (prev_ts, prev_a, _) = unpack_context(*packed_ctx);

    // Enforce nondecreasing time.
    let unix_ts_ms = unix_ts_ms.max(prev_ts);

    // Only enforce nondecreasing random_a when the timestamp
    // has not advanced.
    let random_a = if unix_ts_ms == prev_ts {
        random_a.max(prev_a)
    } else {
        random_a
    };

    *packed_ctx = pack_context(unix_ts_ms, random_a, random_b);
    (unix_ts_ms, random_a)
}

/// For use in strict monotonicity.
///   - Enforces nondecreasing time.
///   - Only need monotonicity if timestamps were in same sub-ms precision,
///     in which case, only `random_b` is incremented.
///   - Handles overflow.
///   - Updates context.
fn ensure_strict_monotonicity(
    unix_ts_ms: u128,
    random_a: u16,
    random_b: u64,
    packed_ctx: &mut u128,
) -> Result<(u128, u16, u64), UUIDv7Error> {
    let (prev_ts, prev_a, prev_b) = unpack_context(*packed_ctx);

    // Enforce nondecreasing time.
    let mut unix_ts_ms = unix_ts_ms.max(prev_ts);
    let mut random_a = random_a;
    let mut random_b = random_b;

    // Only need monotonicity if timestamps are equal.
    if unix_ts_ms == prev_ts && random_a <= prev_a {
        // Prevent sub-ms precision from decreasing.
        random_a = random_a.max(prev_a);

        // Only advance rand_b when rand_a has not advanced
        // and rand_b is not already greater than its previous value.
        if random_a == prev_a && random_b <= prev_b {
            // If random_b has not overflowed, increment it.
            if prev_b < RANDOM_B_MAX {
                random_b = random_b.max(prev_b + 1);
            }
            // If random_b overflowed and random_a has not,
            // increment random_a and reset random_b.
            else if prev_a < RANDOM_A_MAX {
                random_a = prev_a + 1;
                random_b = 0;
            }
            // If both random fields are exhausted, advance the timestamp.
            else if prev_ts < TIMESTAMP_MAX as u128 {
                unix_ts_ms = prev_ts + 1;
                random_a = 0;
                random_b = 0;
            }
            // All fields are exhausted.
            else {
                return Err(UUIDv7Error::Exhausted);
            }
        }
    }

    *packed_ctx = pack_context(unix_ts_ms, random_a, random_b);
    Ok((unix_ts_ms, random_a, random_b))
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
            Exhausted => {
                let e = "The 'timestamp', 'random_a', and 'random_b' fields have all overflowed! Rethink your threading strategy!";
                write!(f, "{e}")
            }
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

struct EntropySourceOld {
    #[cfg(not(feature = "v7-crypto-rand"))]
    rng: MathRandom,
    #[cfg(feature = "v7-crypto-rand")]
    rng: ThreadRng,
}

impl EntropySourceOld {
    #[rustfmt::skip]
    pub fn new() -> Self {
        #[cfg(not(feature = "v7-crypto-rand"))]
        { Self { rng: MathRandom::initialize(Some(rand::rng().random())) } }
        #[cfg(feature = "v7-crypto-rand")]
        { Self { rng: rand::rng() } }
    }

    pub fn random(&mut self) -> u64 {
        self.rng.random()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::RngExt;
    use std::time::{Duration, UNIX_EPOCH};
    use std::{
        collections::HashSet,
        sync::{Arc, Barrier},
        thread,
    };

    fn get_timestamp(raw_uuid: u128) -> u64 {
        (raw_uuid >> 80_u128) as u64
    }

    #[test]
    fn it_works() {
        let _uuid = crate::v7::generate_strict().unwrap();
    }

    #[test]
    fn v7_foo() {
        let mut handles = Vec::with_capacity(10);
        let num_loops = 1_000_000;

        for h in 0..10 {
            let handle = thread::spawn(move || {
                for i in 0..num_loops {
                    if i % (num_loops / 4) == 0 {
                        println!("thread {h} is at {i}");
                    }
                    let mut prev_relaxed: u128 = 0;
                    for _ in 0..10 {
                        let relaxed = crate::v7::generate_raw().unwrap();
                        // println!("{}", crate::encode(relaxed));
                        let relaxed_ts = get_timestamp(relaxed);
                        let prev_relaxed_ts = get_timestamp(prev_relaxed);
                        assert!(
                            // IMPORTANT!! WE NEED TO USE <= BC THIS IS NOT STRICT GENERATION!
                            prev_relaxed_ts <= relaxed_ts,
                            "expected prev_relaxed ({prev_relaxed} | {prev_relaxed_ts}) to be less than or equal to relaxed ({relaxed} | {relaxed_ts})"
                        );
                        prev_relaxed = relaxed;
                    }

                    let relaxed = crate::v7::generate_raw().unwrap();
                    let strict = crate::v7::generate_strict_raw().unwrap();

                    // println!("{}", crate::encode(relaxed));
                    // println!("{}", crate::encode(strict));

                    assert!(
                        relaxed < strict,
                        "expected relaxed ({relaxed}) to still be less than strict ({strict})"
                    );

                    let mut prev_strict = strict;
                    for _ in 0..10 {
                        let sub_strict = crate::v7::generate_strict_raw().unwrap();
                        // println!("{}", crate::encode(sub_strict));
                        assert!(
                            prev_strict < sub_strict,
                            "expected prev_strict ({prev_strict}) to be less than sub_strict ({sub_strict})"
                        );
                        prev_strict = sub_strict;
                    }

                    // println!();
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            _ = handle.join().unwrap();
        }
    }

    #[test]
    #[ignore]
    // This test is just for me to watch creation.
    // `clear && cargo nextest run generate_until_duplicate_or_manually_stopped --run-ignored only --no-capture`
    fn generate_until_duplicate_or_manually_stopped() {
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
            let current_id_u128 = crate::decode_unchecked(&current_id);

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
    fn non_monotonic_clock_drift_timestamp() {
        let ctx_ts: u128 = 100;
        let ctx_a: u16 = 100;
        let ctx_b: u64 = 100;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let ts = 99;
        let a = 100;
        let b = 100;

        let (ts, a) = ensure_monotonicity(ts, a, b, &mut ctx);

        assert_eq!(ts, 100);
        assert_eq!(a, 100);
        assert_eq!(b, 100);
    }

    #[test]
    fn non_monotonic_clock_drift_random_a_sub_ms() {
        let ctx_ts: u128 = 100;
        let ctx_a: u16 = 100;
        let ctx_b: u64 = 100;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let ts = 100;
        let a = 99;
        let b = 100;

        let (ts, a) = ensure_monotonicity(ts, a, b, &mut ctx);

        assert_eq!(ts, 100);
        assert_eq!(a, 100);
        assert_eq!(b, 100);
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
        let sys_time = UNIX_EPOCH + Duration::new(1_672_576_496, 123_456_700);
        let ts = sys_time.duration_since(UNIX_EPOCH).unwrap();

        let a = generate_strict_with(ts, rand::rng().random::<u64>()).unwrap();
        let b = generate_strict_with(ts, rand::rng().random::<u64>()).unwrap();

        let a_u128 = crate::decode_unchecked(&a);
        let b_u128 = crate::decode_unchecked(&b);

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
        assert_eq!(&uuid[14..15], "7");
        assert!(matches!(&uuid[19..20], "8" | "9" | "a" | "b"));
    }

    #[test]
    fn uuid_to_parts_extracts_correctly() {
        let timestamp_system_time = UNIX_EPOCH + Duration::new(1_672_576_496, 123_456_700);
        let timestamp = timestamp_system_time.duration_since(UNIX_EPOCH).unwrap();
        let unix_ts_ms = timestamp.as_millis();
        let random_a = derive_random_a(timestamp);
        let random_b: u64 = 123456700;

        let uuid = generate_with(timestamp, random_b);
        let parts = uuid_to_parts(&uuid).unwrap();

        assert_eq!(
            parts.0, unix_ts_ms,
            "unix_ts_ms | expected {unix_ts_ms} got {}",
            parts.0
        );
        assert_eq!(
            parts.1, random_a,
            "random_a | expected {random_a} got {}",
            parts.1
        );
        assert_eq!(
            parts.2, random_b,
            "random_b | expected {random_b} got {}",
            parts.2
        );

        let uuid = generate_strict_with(timestamp, random_b).unwrap();
        let parts = uuid_to_parts(&uuid).unwrap();

        assert_eq!(
            parts.0, unix_ts_ms,
            "[strict] unix_ts_ms | expected {unix_ts_ms} got {}",
            parts.0
        );
        assert_eq!(
            parts.1, random_a,
            "[strict] random_a | expected {random_a} got {}",
            parts.1
        );
        assert_eq!(
            parts.2, random_b,
            "[strict] random_b | expected {random_b} got {}",
            parts.2
        );
    }

    #[test]
    fn uuid_to_parts_rejects_invalid_uuid() {
        let result = uuid_to_parts("not-a-valid-uuid");
        assert!(result.is_err());
    }

    #[test]
    fn timestamp_is_encoded_correctly() {
        let timestamp = 1_700_000_000_000u128;

        let uuid = generate_strict_with(Duration::from_millis(timestamp as u64), 12345).unwrap();

        let value = crate::decode_unchecked(&uuid);
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
        let ctx_ts = 100;
        let ctx_a = 100;
        let ctx_b = 200;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 100;
        let random_a = 100;
        let random_b = 150;

        let (timestamp, random_a, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 100);
        assert_eq!(random_a, 100);
        assert_eq!(
            random_b, 201,
            "expected random_b to equal 201, got={random_b}"
        );
    }

    #[test]
    fn monotonicity_preserves_higher_random_a() {
        let ctx_ts = 100;
        let ctx_a = 100;
        let ctx_b = 200;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 100;
        let random_a = 101;
        let random_b = 50;

        let (_, random_a, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(random_a, 101);
        assert_eq!(random_b, 50);
    }

    #[test]
    fn monotonicity_preserves_higher_random_b_when_timestamp_and_random_a_match() {
        let ctx_ts = 100;
        let ctx_a = 100;
        let ctx_b = 200;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 101;
        let random_a = 100;
        let random_b = 300;

        let (_, _, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(random_b, 300);
    }

    #[test]
    fn monotonicity_does_not_increment_random_b_when_random_a_differs() {
        let ctx_ts = 100;
        let ctx_a = 100;
        let ctx_b = 200;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 100;
        let random_a = 101;
        let random_b = 50;

        let (_, _, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(random_b, 50);
    }

    #[test]
    fn monotonicity_clamps_backwards_timestamp() {
        let ctx_ts = 100;
        let ctx_a = 100;
        let ctx_b = 200;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 99;
        let random_a = 100;
        let random_b = 200;

        let (timestamp, random_a, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 100);
        assert_eq!(random_a, 100);
        assert_eq!(random_b, 201);
    }

    #[test]
    fn monotonicity_allows_timestamp_to_advance() {
        let ctx_ts = 100;
        let ctx_a = RANDOM_A_MAX;
        let ctx_b = RANDOM_B_MAX;
        let mut ctx = pack_context(ctx_ts, ctx_a, ctx_b);

        let timestamp = 101;
        let random_a = 10;
        let random_b = 20;

        let (timestamp, random_a, random_b) =
            ensure_strict_monotonicity(timestamp, random_a, random_b, &mut ctx).unwrap();

        assert_eq!(timestamp, 101);
        assert_eq!(random_a, 10);
        assert_eq!(random_b, 20);
    }

    // Tests whether each thread still produces strictly monotonic ids
    #[test]
    fn concurrent_generation_produces_thread_local_strict_monotonic_uuids() {
        const THREADS: usize = 8;
        const UUIDS_PER_THREAD: usize = 10_000;

        let barrier = Arc::new(Barrier::new(THREADS));

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    barrier.wait();

                    (0..UUIDS_PER_THREAD)
                        .map(|_| generate_strict().unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        for (idx, handle) in handles.into_iter().enumerate() {
            let uuids = handle.join().unwrap();
            assert!(
                uuids.windows(2).all(|w| w[0] < w[1]),
                "thread {idx} did not produce strictly monotonic UUIDs!"
            );
        }
    }
}

#[cfg(all(test, feature = "v7-global-monotonicity"))]
mod strict_global_monotonicity {
    use std::{
        collections::HashSet,
        sync::{Arc, Barrier},
        thread,
    };

    use super::*;

    fn generate_strict_and<CB>(mut cb: CB) -> Result<(), UUIDv7Error>
    where
        CB: FnMut(&str),
    {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let random_b = rand::rng().random::<u64>();
        let unix_ts_ms = timestamp.as_millis() & TIMESTAMP_MAX as u128;
        let random_a = derive_random_a(timestamp) & RANDOM_A_MAX;
        let random_b = random_b & RANDOM_B_MAX;

        let mut ctx = STRICT_CONTEXT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let (unix_ts_ms, random_a, random_b) =
            ensure_strict_monotonicity(unix_ts_ms, random_a, random_b, &mut ctx)?;

        let uuid = (unix_ts_ms << TIMESTAMP_SHIFT) & CLEAR_VERSION_MASK
            | ((VERSION as u128) << VERSION_SHIFT)
            | ((random_a as u128) << RANDOM_A_SHIFT) & CLEAR_VARIANT_MASK
            | ((VARIANT as u128) << VARIANT_SHIFT)
            | (random_b as u128);

        cb(&crate::encode(uuid));
        Ok(())
    }

    #[test]
    fn concurrent_generation_produces_global_monotonicy() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const THREADS: usize = 8;
        const UUIDS_PER_THREAD: usize = 10_000;

        let barrier = Arc::new(Barrier::new(THREADS));
        let index = Arc::new(AtomicUsize::new(0));

        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let index = Arc::clone(&index);

                thread::spawn(move || {
                    let mut generated = Vec::with_capacity(UUIDS_PER_THREAD);

                    barrier.wait();

                    for _ in 0..UUIDS_PER_THREAD {
                        generate_strict_and(|uuid| {
                            let uuid_value = crate::decode_unchecked(uuid);
                            let i = index.fetch_add(1, Ordering::Relaxed);
                            generated.push((i, uuid_value));
                        })
                        .unwrap();
                    }

                    generated
                })
            })
            .collect();

        let mut uuids: Vec<_> = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect();

        assert_eq!(uuids.len(), THREADS * UUIDS_PER_THREAD);

        uuids.sort_unstable_by_key(|(index, _)| *index);

        let mut prev_index: isize = -1;
        let mut prev: u128 = 0;
        for (index, uuid) in &uuids {
            assert!(
                prev_index < *index as isize,
                "index not in order! {prev_index} {}",
                *index
            );
            assert!(
                prev < *uuid,
                "not globally strictly mono! {prev} is greater than {}",
                *uuid
            );
            prev = *uuid;
            prev_index = *index as isize;
        }
        // assert!(
        //     uuids.windows(2).all(|w| w[0].1 < w[1].1),
        //     "UUIDs were not globally strictly monotonic!",
        // );

        let unique_uuids: HashSet<_> = uuids.iter().map(|(_, uuid)| uuid).collect();

        assert_eq!(
            unique_uuids.len(),
            uuids.len(),
            "Duplicate UUIDs generated!"
        );
    }

    #[test]
    // This launches 8 threads, each generating 10,000 UUIDs simultaneously, then verifies that all 80,000 UUIDs are unique.
    fn concurrent_generation_produces_unique_uuids() {
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
*/
