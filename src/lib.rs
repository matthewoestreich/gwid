use std::fmt::Binary;

pub mod v4;
pub mod v7;

pub(crate) const V7_TIMESTAMP_BITS: u128 = 48;
pub(crate) const V7_TIMESTAMP_MASK: u128 = (1 << V7_TIMESTAMP_BITS) - 1;
pub(crate) const V7_RANDOM_A_MAX: u64 =
    0b0000000000000000000000000000000000000000000000000000111111111111; // 4095;
pub(crate) const V7_RANDOM_B_MASK: u64 =
    0b0011111111111111111111111111111111111111111111111111111111111111;

pub(crate) const VERSION_MASK: u128 = 0b00000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000;
pub(crate) const VARIANT_MASK: u128 = 0b00000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000;

const FIRST_DASH_INDEX: usize = 8;
const SECOND_DASH_INDEX: usize = 12;
const THIRD_DASH_INDEX: usize = 16;
const FOURTH_DASH_INDEX: usize = 20;

pub(crate) fn add_dashes(uuid: u128) -> String {
    let uuid = format!("{uuid:032x}");

    format!(
        "{}-{}-{}-{}-{}",
        &uuid[..FIRST_DASH_INDEX],
        &uuid[FIRST_DASH_INDEX..SECOND_DASH_INDEX],
        &uuid[SECOND_DASH_INDEX..THIRD_DASH_INDEX],
        &uuid[THIRD_DASH_INDEX..FOURTH_DASH_INDEX],
        &uuid[FOURTH_DASH_INDEX..]
    )
}

// Helper function to print bits
pub(crate) fn print_bits<T: Binary>(value: T) {
    let width = std::mem::size_of::<T>() * 8;
    let bits = format!("{value:0width$b}");

    for (i, byte) in bits.as_bytes().chunks(8).enumerate() {
        if i > 0 {
            print!(" ");
        }

        print!("{}", std::str::from_utf8(byte).unwrap());
    }

    println!();
}
