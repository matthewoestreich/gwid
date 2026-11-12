use std::fmt::Binary;

pub mod v4;
pub mod v7;

// `n & CLEAR_VERSION_MASK` will clear the bits for a UUID verrsion (bits 48, 49, 50 and 51)
// 11111111 11111111 11111111 11111111 11111111 11111111 00001111 11111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111
pub(crate) const CLEAR_VERSION_MASK: u128 = 0xFFFFFFFFFFFF0FFFFFFFFFFFFFFFFFFF;
// `n & CLEAR_VARIANT_MASK` will clear the bits for a UUID variant (bits 64 and 65)
// 11111111 11111111 11111111 11111111 11111111 11111111 11111111 00111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111 11111111
pub(crate) const CLEAR_VARIANT_MASK: u128 = 0xFFFFFFFFFFFFFFFF3FFFFFFFFFFFFFFF;
pub(crate) const VERSION_SHIFT: u128 = 76;
pub(crate) const VARIANT_SHIFT: u128 = 62;

/// Formats `n` as hex and inserts hyphens at appropriate indicies.
pub(crate) fn add_hyphens(n: u128) -> String {
    const FIRST_HYPHEN_INDEX: usize = 8;
    const SECOND_HYPHEN_INDEX: usize = 12;
    const THIRD_HYPHEN_INDEX: usize = 16;
    const FOURTH_HYPHEN_INDEX: usize = 20;

    let uuid = format!("{n:032x}");

    format!(
        "{}-{}-{}-{}-{}",
        &uuid[..FIRST_HYPHEN_INDEX],
        &uuid[FIRST_HYPHEN_INDEX..SECOND_HYPHEN_INDEX],
        &uuid[SECOND_HYPHEN_INDEX..THIRD_HYPHEN_INDEX],
        &uuid[THIRD_HYPHEN_INDEX..FOURTH_HYPHEN_INDEX],
        &uuid[FOURTH_HYPHEN_INDEX..]
    )
}

// Helper function to print bits
#[allow(dead_code)]
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

/// Unchecked method! Panics if something goes wrong.
/// Converts a `uuid` string into a u128.
#[allow(dead_code)]
pub(crate) fn uuid_to_u128(uuid: &str) -> u128 {
    u128::from_str_radix(&uuid.replace("-", ""), 16).unwrap()
}
