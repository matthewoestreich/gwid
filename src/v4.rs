// UUID VERSION 4
//
// https://www.rfc-editor.org/info/rfc9562/#section-5.4
//
//  0                   1                   2                   3
//  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           random_a                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |          random_a             |  ver  |       random_b        |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |var|                       random_c                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
// |                           random_c                            |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//
// BIT LAYOUT:
// [000000000000000000000000000000000000000000000000] [0100] [000000000000] [10] [00000000000000000000000000000000000000000000000000000000000000]
// |<-------------------- random_a ---------------->| | ver | | random_b | |var| |<------------------------ random_c ------------------------>|
//
// random_a:
//     The first 48 bits of the layout that can be filled with random data as specified in Section 6.9. Occupies bits 0 through 47 (octets 0-5).
// ver:
//     The 4-bit version field as defined by Section 4.2, set to 0b0100 (4). Occupies bits 48 through 51 of octet 6.
// random_b:
//     12 more bits of the layout that can be filled random data as per Section 6.9. Occupies bits 52 through 63 (octets 6-7).
// var:
//     The 2-bit variant field as defined by Section 4.1, set to 0b10. Occupies bits 64 and 65 of octet 8.
// random_c:
//     The final 62 bits of the layout immediately following the var field to be filled with random data as per Section 6.9. Occupies bits 66 through 127 (octets 8-15).

use crate::CLEAR_VARIANT_MASK;
use crate::CLEAR_VERSION_MASK;
use crate::VARIANT_SHIFT;
use crate::VERSION_SHIFT;
use rand::RngExt as _;

const VERSION: u8 = 4; // 0b100
const VARIANT: u8 = 2; // 0b10

/// Generates a UUIDv4 using a cryptographically secure random number generator.
pub fn generate() -> String {
    generate_with(generate_raw())
}

pub fn generate_raw() -> u128 {
    generate_raw_with(rand::rng().random())
}

fn generate_raw_with(random_bits: u128) -> u128 {
    (random_bits & CLEAR_VERSION_MASK & CLEAR_VARIANT_MASK)
        | ((VERSION as u128) << VERSION_SHIFT)
        | ((VARIANT as u128) << VARIANT_SHIFT)
}

fn generate_with(random_bits: u128) -> String {
    crate::encode(generate_raw_with(random_bits))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_works() {
        let uuid = crate::v4::generate();
        println!("{uuid}");
    }

    #[test]
    fn example_from_rfc() {
        let expected = "919108f752d143209bacf847db4148a8";
        let rand: u128 = 0b10010001100100010000100011110111010100101101000100110011001000000101101110101100111110000100011111011011010000010100100010101000;
        let result = generate_with(rand).replace("-", "");
        assert_eq!(
            result, expected,
            "expected these to equal : expected={expected} got={result}"
        );
    }

    #[test]
    fn test_uuid_v4_length() {
        let uuid = generate();
        assert_eq!(uuid.len(), 36);
    }

    #[test]
    fn test_uuid_v4_dash_positions() {
        let uuid = generate();
        for position in [8, 13, 18, 23] {
            assert_eq!(uuid.as_bytes()[position], b'-');
        }
    }

    #[test]
    fn test_uuid_v4_version() {
        let uuid = generate();
        assert_eq!(uuid.as_bytes()[14], b'4');
    }

    #[test]
    fn test_uuid_v4_variant() {
        let uuid = generate();
        assert!(matches!(uuid.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    }

    #[test]
    fn test_uuid_v4_valid_hexadecimal() {
        let uuid = generate();
        let hex: String = uuid.chars().filter(|c| *c != '-').collect();
        assert_eq!(hex.len(), 32);
        assert!(u128::from_str_radix(&hex, 16).is_ok());
    }

    #[test]
    fn test_uuid_v4_multiple_generations() {
        for _ in 0..1_000 {
            let uuid = generate();

            let version_bytes = uuid.as_bytes()[14];
            let variant_bytes = uuid.as_bytes()[19];
            let expected_version = b'4';

            assert_eq!(
                version_bytes, expected_version,
                "expected these to equal : '{version_bytes}' != '{expected_version}'",
            );

            assert!(
                matches!(variant_bytes, b'8' | b'9' | b'a' | b'b'),
                "expected match : '{variant_bytes}' != '{} | {} | {} | {}'",
                b'8',
                b'9',
                b'a',
                b'b'
            );
        }
    }
}
