use std::fmt::Binary;

pub mod v4;
pub mod v7;

const FIRST_HYPHEN_INDEX: usize = 8;
const SECOND_HYPHEN_INDEX: usize = 12;
const THIRD_HYPHEN_INDEX: usize = 16;
const FOURTH_HYPHEN_INDEX: usize = 20;

pub(crate) fn add_hyphens(uuid: u128) -> String {
    let uuid = format!("{uuid:032x}");

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
#[allow(dead_code)]
pub(crate) fn uuid_to_u128(uuid: &str) -> u128 {
    u128::from_str_radix(&uuid.replace("-", ""), 16).unwrap()
}
