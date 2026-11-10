use crate::{VARIANT_MASK, VERSION_MASK};
use rand::RngExt as _;
use std::time;

pub fn generate() -> String {
    todo!()
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
fn generate_random_a() -> u16 {
    // 1 Jan 2023 12:34:56.1234567
    let ts_secs = 1_672_576_496;
    let ts_nanos = 123_456_700;

    let timestamp = time::UNIX_EPOCH + time::Duration::new(ts_secs, ts_nanos);

    let mut last_ms: u128 = 0;
    let mut already_logged = false;

    for _ in 0..1 {
        // time::SystemTime::now()
        let duration = timestamp.duration_since(time::UNIX_EPOCH).unwrap();

        let millis = duration.as_millis();

        if millis == last_ms {
            if !already_logged {
                println!("\n ---- SAME MS DETECTED ---- \n");
                already_logged = true;
            }
        } else {
            already_logged = false;
        }

        let sub_ms_nanos = duration.subsec_nanos() % 1_000_000;
        let extra_precision = sub_ms_nanos as f64 / 1_000_000.0;
        let rand_val = (4096.0 * extra_precision).floor();

        println!("millis          = {millis}");
        println!("sub_ms_nanos    = {sub_ms_nanos}");
        println!("extra_precision = {extra_precision}");
        println!("rand_val        = {rand_val} : {:b}", rand_val as u16);
        println!();

        last_ms = millis
    }

    0
}

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {}
}
