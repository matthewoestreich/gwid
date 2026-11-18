use crate::{CLEAR_VARIANT_MASK, CLEAR_VERSION_MASK, VARIANT_SHIFT, VERSION_SHIFT, print_bits};
use rand::RngExt as _;
use std::{
    cell::{Cell, RefCell},
    sync::atomic::AtomicU16,
};

thread_local! {
    static V1_CONTEXT: RefCell<AtomicU16> = RefCell::new(AtomicU16::new(rand::rng().random())) ;
}

const TIMESTAMP_BITS: u64 = 60;
const TIMESTAMP_LOW_BITS: u64 = 32;
const TIMESTAMP_MIDDLE_BITS: u64 = 16;
const TIMESTAMP_HIGH_BITS: u64 = 12;
const CLOCK_SEQUENCE_BITS: u16 = 14;
const NODE_BITS: u64 = 48;

const TIMESTAMP_HIGH_MAX: u16 = (1 << 12) - 1;
const NODE_MASK: u64 = (1 << 48) - 1;

const VERSION: u8 = 1;
const VARIANT: u8 = 2;

pub fn generate_raw() -> u128 {
    let ts = crate::timestamp::gregorian_now();
    let clock_seq =
        V1_CONTEXT.with_borrow_mut(|x| x.fetch_add(1, std::sync::atomic::Ordering::AcqRel));
    let node = rand::rng().random();
    generate_raw_with(ts, clock_seq, node)
}

fn generate_raw_with(gregorian_timestamp: u128, clock_seq: u16, node: u64) -> u128 {
    // println!();
    // println!("timestamp:");
    // print_bits(gregorian_timestamp as u64);
    let (low, mid, hi) = timestamp_to_masked_parts(gregorian_timestamp as u64);
    // print_bits(low as u64);
    // print_bits(mid as u64);
    // print_bits(hi as u64);

    let mut o = 0u128;

    o |= (low as u128) << 96;
    // println!("output after shifting in low");
    // print_bits(o);
    o |= (mid as u128) << 80;
    // println!("output after shifting in mid");
    // print_bits(o);
    o |= (VERSION as u128) << VERSION_SHIFT;
    // println!("output after shifting in version");
    // print_bits(o);
    o |= (hi as u128) << 64;
    // println!("output after shifting in hi");
    // print_bits(o);
    o |= (VARIANT as u128) << VARIANT_SHIFT;
    // println!("output after shifting in variant");
    // print_bits(o);
    o |= (clock_seq as u128) << 48;
    // println!("output after shifting in clock sequence");
    // print_bits(o);
    o |= (node & NODE_MASK) as u128;
    // println!("output after shifting in node");
    // print_bits(o);

    o
}

// 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000

fn timestamp_to_masked_parts(gregorian_timestamp: u64) -> (u32, u16, u16) {
    (
        gregorian_timestamp as u32,
        (gregorian_timestamp >> 32) as u16,
        (gregorian_timestamp >> 48) as u16 & TIMESTAMP_HIGH_MAX,
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{LazyLock, atomic::Ordering};
    use uuid::{ClockSequence, Timestamp, Uuid};

    static CONTEXT: LazyLock<ContextV1> = LazyLock::new(ContextV1::new_random);

    pub(crate) fn shared_context_v1() -> &'static ContextV1 {
        &CONTEXT
    }

    #[derive(Debug)]
    pub struct ContextV1 {
        count: AtomicU16,
    }

    impl ContextV1 {
        /// Construct a new context that's initialized with the given value.
        ///
        /// The starting value should be a random number, so that UUIDs from
        /// different systems with the same timestamps are less likely to collide.
        /// When the `rng` feature is enabled, prefer the [`ContextV1::new_random`] method.
        pub const fn new(count: u16) -> Self {
            Self {
                count: AtomicU16::new(count),
            }
        }

        /// Construct a new context that's initialized with a random value.
        pub fn new_random() -> Self {
            Self {
                count: AtomicU16::new(rand::rng().random()),
            }
        }
    }

    impl ClockSequence for ContextV1 {
        type Output = u16;

        fn generate_sequence(&self, _seconds: u64, _nanos: u32) -> Self::Output {
            // RFC 9562 reserves 2 bits of the clock sequence so the actual
            // maximum value is smaller than `u16::MAX`. Since we unconditionally
            // increment the clock sequence we want to wrap once it becomes larger
            // than what we can represent in a "u14". Otherwise there'd be patches
            // where the clock sequence doesn't change regardless of the timestamp
            self.count.fetch_add(1, Ordering::AcqRel) & (u16::MAX >> 2)
        }

        fn usable_bits(&self) -> usize {
            14
        }
    }

    fn encode_node_id(counter: u16, node_id: &[u8; 6]) -> u64 {
        let mut d4 = [0; 8];

        d4[0] = (((counter & 0x3F00) >> 8) as u8) | 0x80;
        d4[1] = (counter & 0xFF) as u8;
        d4[2] = node_id[0];
        d4[3] = node_id[1];
        d4[4] = node_id[2];
        d4[5] = node_id[3];
        d4[6] = node_id[4];
        d4[7] = node_id[5];

        u64::from_be_bytes(d4)
    }

    #[test]
    fn v1_foo() {
        let ctx = shared_context_v1();
        let node_bytes: [u8; 6] = [1, 2, 3, 4, 5, 6];
        let mut prev: u128 = 0;

        loop {
            let ts = Timestamp::now(ctx);
            let node_u64 = encode_node_id(ts.to_gregorian().1, &node_bytes);
            let v1_raw = generate_raw_with(
                crate::timestamp::gregorian_now(),
                ts.to_gregorian().1,
                node_u64,
            );
            let v1_id = crate::encode(v1_raw);
            println!("mine = {v1_id}");

            // let uuid = Uuid::now_v1(&[1, 2, 3, 4, 5, 6]);
            let uuid = Uuid::new_v1(ts, &node_bytes);
            println!("uuid = {uuid}\n");

            if prev >= v1_raw {
                println!("\n\nNOT MONOTONIC\n\n");
            }
            prev = v1_raw
        }
    }
}
