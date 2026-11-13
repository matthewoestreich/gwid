pub mod v4;
pub mod v7;

pub(crate) mod nums;

/// Formats `n` as hex and inserts hyphens at appropriate indicies.
pub fn add_hyphens(n: u128) -> String {
    let uuid = format!("{n:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        // 8 is index of first hyphen
        &uuid[..8],
        // 12 is index of second hyphen
        &uuid[8..12],
        // 16 is index of third hyphen
        &uuid[12..16],
        // 20 is index of fourth hyphen
        &uuid[16..20],
        &uuid[20..]
    )
}

/// Unchecked method! Panics if something goes wrong.
/// Converts a `uuid` string into a u128.
pub fn uuid_to_raw_unchecked(uuid: &str) -> u128 {
    u128::from_str_radix(&uuid.replace("-", ""), 16).unwrap()
}

/// Converts a `uuid` string into a u128.
pub fn uuid_to_raw(uuid: &str) -> Result<u128, std::num::ParseIntError> {
    u128::from_str_radix(&uuid.replace("-", ""), 16)
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

#[cfg(test)]
mod lib_test {
    use super::*;

    #[test]
    fn add_hyphens_to_raw_uuid() {
        let raw: u128 = 213409605541016776915642275756511741574;
        let expect = "a08d31e9-c256-443e-8425-73c382264686";
        let got = add_hyphens(raw);
        assert_eq!(
            expect, &got,
            "add_hyphens_to_raw_uuid | expected= {expect} got= {got}"
        );
    }

    // Tests both unchecked and "regular" variants.
    #[test]
    fn convert_uuid_to_u128() {
        let uuid = "a08d31e9-c256-443e-8425-73c382264686";
        let expect: u128 = 213409605541016776915642275756511741574;
        let got = uuid_to_raw_unchecked(uuid);
        assert_eq!(
            expect, got,
            "convert_uuid_to_u128_unchecked | expected= {expect} got= {got}"
        );
        let got = uuid_to_raw(uuid).unwrap();
        assert_eq!(
            expect, got,
            "convert_uuid_to_u128 | expected= {expect} got= {got}"
        );
    }
}
