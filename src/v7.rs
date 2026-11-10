// UUID VERSION 7
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
// [000000000000000000000000000000000000000000000000] [0100] [000000000000] [10] [00000000000000000000000000000000000000000000000000000000000000]
// |<-------------------- unix_ts_ms -------------->| | ver | | random_a | |var| |<------------------------ random_b ------------------------>|
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

use crate::{V7_RANDOM_A_MAX, V7_RANDOM_B_MASK, V7_TIMESTAMP_MASK, print_bits};
use rand::RngExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn generate() -> String {
    let timestamp_duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("v7::generate");

    let _timestamp = timestamp_duration.as_millis();
    let _unix_ts_ms = _timestamp & V7_TIMESTAMP_MASK;
    let _random_a = generate_random_a(timestamp_duration);
    let _random_b = rand::rng().random::<u64>() & V7_RANDOM_B_MASK;

    let uuid: u128 = 0;

    println!("unix_ts_ms = {_unix_ts_ms}");
    print_bits(_unix_ts_ms);
    println!("random_a = {_random_a}");
    print_bits(_random_a);
    println!("random_b = {_random_b}");
    print_bits(_random_b);

    "".to_string()
}

// https://www.rfc-editor.org/info/rfc9562/#section-6.2
//
// This is for generating the `random_a` bits as to satisfy monotonic constraints.
//
// We have chosen Method 3:
//
//  Replace Leftmost Random Bits with Increased Clock Precision (Method 3):
//
//    For UUIDv7, which has millisecond timestamp precision, it is possible to use additional
//    clock precision available on the system to substitute for up to 12 random bits immediately
//    following the timestamp. This can provide values that are time ordered with sub-millisecond
//    precision, using however many bits are appropriate in the implementation environment. With
//    this method, the additional time precision bits MUST follow the timestamp as the next
//    available bit in the rand_a field for UUIDv7.
//
//    To calculate this value, start with the portion of the timestamp expressed as a fraction
//    of the clock's tick value (fraction of a millisecond for UUIDv7). Compute the count of
//    possible values that can be represented in the available bit space, 4096 for the UUIDv7
//    rand_a field. Using floating point or scaled integer arithmetic, multiply this fraction of
//    a millisecond value by 4096 and round down (toward zero) to an integer result to arrive at
//    a number between 0 and the maximum allowed for the indicated bits, which sorts monotonically
//    based on time. Each increasing fractional value will result in an increasing bit field value
//    to the precision available with these bits.
//
//    For example, let's assume a system timestamp of 1 Jan 2023 12:34:56.1234567. Taking the
//    precision greater than 1 ms gives us a value of 0.4567, as a fraction of a millisecond.
//    If we wish to encode this as 12 bits, we can take the count of possible values that fit in
//    those bits (4096 or 212), multiply it by our millisecond fraction value of 0.4567, and
//    truncate the result to an integer, which gives an integer value of 1870. Expressed as
//    hexadecimal, it is 0x74E or the binary bits 0b011101001110. One can then use those 12 bits
//    as the most significant (leftmost) portion of the random section of the UUID (e.g., the
//    rand_a field in UUIDv7). This works for any desired bit length that fits into a UUID,
//    and applications can decide the appropriate length based on available clock precision;
//    for UUIDv7, it is limited to 12 bits at maximum to reserve sufficient space for random bits.
//    The main benefit to encoding additional timestamp precision is that it utilizes additional
//    time precision already available in the system clock to provide values that are more likely
//    to be unique; thus, it may simplify certain implementations. This technique can also be used
//    in conjunction with one of the other methods, where this additional time precision would
//    immediately follow the timestamp. Then, if any bits are to be used as a clock sequence, they
//    would follow next.
fn generate_random_a(duration: Duration) -> u16 {
    // Scale sub-millisecond precision into the random_a range.
    let sub_ms_nanos = (duration.subsec_nanos() % 1_000_000) as u64;
    let extra_precision = V7_RANDOM_A_MAX * sub_ms_nanos;

    // random_a
    (extra_precision / 1_000_000) as u16
}

#[cfg(test)]
mod test {
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn it_works() {
        let _uuid = crate::v7::generate();
    }

    #[test]
    fn random_a_method_3() {
        let expected: u16 = 1870;

        // 1 Jan 2023 12:34:56.1234567
        let jan_1_2023_12_34_56_1234567 = UNIX_EPOCH + Duration::new(1_672_576_496, 123_456_700);

        let duration = jan_1_2023_12_34_56_1234567
            .duration_since(UNIX_EPOCH)
            .unwrap();

        let timestamp = duration.as_millis();
        let random_a = crate::v7::generate_random_a(duration);

        println!("\ntimestamp = {timestamp}\nrandom_a = {random_a}\n");

        assert_eq!(
            expected, random_a,
            "expected = {expected} ; got = {random_a}"
        );
    }
}
