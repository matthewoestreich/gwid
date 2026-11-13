pub mod v4;
pub mod v7;

pub(crate) mod nums;

/// Formats `n` as hex and inserts hyphens at appropriate indicies.
pub fn add_hyphens(n: u128) -> String {
    // const FIRST_HYPHEN_INDEX: usize = 8;
    // const SECOND_HYPHEN_INDEX: usize = 12;
    // const THIRD_HYPHEN_INDEX: usize = 16;
    // const FOURTH_HYPHEN_INDEX: usize = 20;

    let uuid = format!("{n:032x}");

    format!(
        "{}-{}-{}-{}-{}",
        &uuid[..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..]
    )
}

// Clears UUID Version bits (bits 48, 49, 50, and 51 when counting left->right and starting at 0)
// 11111111111111111111111111111111111111111111111100001111111111111111111111111111111111111111111111111111111111111111111111111111
pub(crate) const CLEAR_VERSION_MASK: u128 = 0xFFFFFFFFFFFF0FFFFFFFFFFFFFFFFFFF;
// Clears UUID Variant bits (bits 64 and 65 when counting left->right and starting at 0)
// 11111111111111111111111111111111111111111111111111111111111111110011111111111111111111111111111111111111111111111111111111111111
pub(crate) const CLEAR_VARIANT_MASK: u128 = 0xFFFFFFFFFFFFFFFF3FFFFFFFFFFFFFFF;
pub(crate) const VERSION_SHIFT: u128 = 76;
pub(crate) const VARIANT_SHIFT: u128 = 62;

// Helper function to print bits
#[allow(dead_code)]
pub(crate) fn print_bits<T: std::fmt::Binary>(value: T) {
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
pub(crate) fn uuid_to_u128_unchecked(uuid: &str) -> u128 {
    u128::from_str_radix(&uuid.replace("-", ""), 16).unwrap()
}

/// Converts a `uuid` string into a u128.
#[allow(dead_code)]
pub(crate) fn uuid_to_u128(uuid: &str) -> Result<u128, std::num::ParseIntError> {
    u128::from_str_radix(&uuid.replace("-", ""), 16)
}
