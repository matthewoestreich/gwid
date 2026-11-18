// It has been 14,014,252,440 seconds since October 15, 1582, at 00:00:00 UTC (the start
// of the Gregorian calendar reform / UUIDv1 epoch), calculated up to November 17, 2026, at 23:54:00 UTC.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND: u64 = 10_000_000;
// Seconds between 1582-10-15 00:00:00 UTC and 1970-01-01 00:00:00 UTC
const GREGORIAN_TO_UNIX_SECS: u64 = 12_219_292_800;
// In 100-nanosecond ticks (common in UUID v1/v6 and Windows FILETIME math)
const GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS: u64 = 122_192_928_000_000_000;

const SIXTY_BITS_MASK: u64 = (1 << 60) - 1;

fn gregorian_epoch() -> SystemTime {
    UNIX_EPOCH - Duration::from_secs(GREGORIAN_TO_UNIX_SECS)
}

fn current_gregorian_ticks() -> u64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is before Unix epoch");

    // Convert seconds and nanoseconds into 100-nanosecond ticks
    let unix_ticks = duration.as_secs() * 10_000_000 + (duration.subsec_nanos() as u64) / 100;

    unix_ticks + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS
}

pub fn gregorian_now() -> u128 {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();

    duration.as_nanos() / 100 + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS as u128
}

#[cfg(test)]
mod test {
    use rand::RngExt;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn timestamp_bar() {
        let mut r: [u8; 6] = [0; 6];
        loop {
            rand::rng().fill(&mut r);
            let idv1 = Uuid::now_v1(&r);
            println!("{idv1} | {}", idv1.as_u128());

            // let id = Uuid::now_v7();
            // println!("uuid = {id}");
            // let mid = crate::v7::generate_strict().unwrap();
            // println!("mine = {mid}");
            // println!();
        }
    }

    #[test]
    fn timestamp_foo() {
        let unix_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let unix_ts_secs = unix_ts.as_secs();
        let unix_ts_ms = unix_ts.as_millis();
        let unix_ts_ms_calcuated = unix_ts_secs * 1_000;
        let unix_ts_subsec_millis = unix_ts.subsec_millis();

        let unix_ts_secs_as_gregorian_ticks =
            (unix_ts_secs * NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND);
        let gregorian_timestamp_raw =
            unix_ts_secs_as_gregorian_ticks + GREGORIAN_TO_UNIX_OFFSET_IN_100_NANOSECOND_TICKS;
        let gregorian_timestamp = gregorian_timestamp_raw & SIXTY_BITS_MASK;

        const GREGORIAN_OFFSET: u128 = 122_192_928_000_000_000;
        let gregorian_timestamp = unix_ts_ms * 10_000_u128 + GREGORIAN_OFFSET;

        println!("sec      = {unix_ts_secs}");
        println!("ms       = {unix_ts_ms}");
        println!("ms_calc  = {unix_ts_ms_calcuated} + {unix_ts_subsec_millis} subsec_millis");
        println!("greg     = {gregorian_timestamp}");
        println!("greg_now = {}", gregorian_now());
        println!("\n\n");

        println!();

        let mine_greg = SystemTime::now().duration_since(gregorian_epoch()).unwrap();
        let mine_greg_secs = mine_greg.as_secs();
        let mine_greg_as_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH - Duration::from_secs(mine_greg.as_secs()))
            .unwrap();
        let mine_greg_as_unix_nanos = mine_greg_as_unix.as_secs();
        println!("mine    = {mine_greg_secs} unix = {mine_greg_as_unix_nanos}");

        let theirs_greg = Duration::from_secs(
            current_gregorian_ticks() / NUMBER_OF_100_NANOSECOND_TICKS_PER_SECOND,
        );
        let theirs_greg_secs = theirs_greg.as_secs();
        let theirs_greg_as_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH - Duration::from_secs(theirs_greg.as_secs()))
            .unwrap();
        let theirs_greg_as_unix_nanos = theirs_greg_as_unix.as_secs();
        println!("theirs  = {theirs_greg_secs} unix = {theirs_greg_as_unix_nanos}");
    }
}
