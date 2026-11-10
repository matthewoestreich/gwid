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

use rand::RngExt as _;

const V4_VERSION_MASK: u128      = 0b00000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000000000000000000;
const V4_VERSION_KEEP_MASK: u128 = 0b11111111111111111111111111111111111111111111111100001111111111111111111111111111111111111111111111111111111111111111111111111111;
const V4_VARIANT_MASK: u128      = 0b00000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000;
const V4_VARIANT_KEEP_MASK: u128 = 0b11111111111111111111111111111111111111111111111111111111111111110011111111111111111111111111111111111111111111111111111111111111;

pub fn generate() -> String {
    generate_with(rand::rng().random::<u128>())
}

fn generate_with(random_bits: u128) -> String {
    let uuid = (random_bits & V4_VERSION_KEEP_MASK & V4_VARIANT_KEEP_MASK)
        | V4_VERSION_MASK
        | V4_VARIANT_MASK;

    crate::add_dashes(uuid)
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

        // Version is represented by the first nibble of the third group.
        assert_eq!(uuid.as_bytes()[14], b'4');
    }

    #[test]
    fn test_uuid_v4_variant() {
        let uuid = generate();

        // Variant must be 10xx in binary.
        // Therefore, the first hex digit of the fourth group
        // must be 8, 9, A, or B.
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

            let fourteen_bytes = uuid.as_bytes()[14];
            let fourteen_expected = b'4';

            let nineteen_bytes = uuid.as_bytes()[19];

            assert_eq!(
                fourteen_bytes, fourteen_expected,
                "expected these to equal : '{fourteen_bytes}' != '{fourteen_expected}'",
            );
            assert!(
                matches!(nineteen_bytes, b'8' | b'9' | b'a' | b'b'),
                "expected match : '{nineteen_bytes}' != '{} | {} | {} | {}'",
                b'8',
                b'9',
                b'a',
                b'b'
            );
        }
    }
}
