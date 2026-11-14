/// # gwid
///
/// [![Crates.io Version](https://img.shields.io/crates/v/gwid?style=flat-square&logoSize=auto&color=%233d7ae3)](https://crates.io/crates/gwid) [![docs.rs](https://img.shields.io/docsrs/gwid?style=flat-square&label=docs.rs)](https://docs.rs/gwid/latest/gwid/)
///
/// Implementing [RFC 9562](https://www.rfc-editor.org/info/rfc9562/) ccompliant UUIDs (also known as GUIDs). The name gwid is derived from the phonetic pronunciation of GUID.
///
/// # UUID Version 4
///
/// UUIDv4 is a 128-bit UUID generated using cryptographically secure random bits, providing a high degree of uniqueness without encoding a timestamp or providing chronological ordering.
///
/// ## `generate() -> String`
///
/// **Usage**
///
/// ```rust
/// use gwid;
/// let uuid = gwid::v4::generate();
/// ```
///
/// ## `generate_raw() -> u128`
///
/// Returns the raw `u128` UUID. Use [`encode`](#encodeuuid-u128---string) to format raw `u128` as UUID.
///
/// **Usage**
///
/// ```rust
/// use gwid;
/// let uuid = gwid::v4::generate_raw();
/// let encoded = gwid::encode(uuid);
/// ```
///
/// # UUID Version 7
///
/// UUIDv7 is a 128-bit, time-ordered UUID that embeds a Unix timestamp (milliseconds) and uses random bits for uniqueness. Its time-based structure makes UUIDs naturally sortable by creation time.
///
/// ## `generate() -> String`
///
/// ```rust
/// use gwid;
/// let uuidv7 = gwid::v7::generate().expect("error generating");
/// ```
///
/// Generates UUIDv7 values with sub-millisecond [timestamp](#monotonic-precision) precision. Although thread-safe, UUIDs generated across multiple threads are not guaranteed to be globally monotonic.
///
/// For example, two UUIDs generated within the same sub-millisecond bucket may be out of order:
///
/// ```rust
/// // First UUID (greater)
/// let first = "01a1d02e-eedb-7995-82da-278257b8340d";
/// // Second UUID (smaller)
/// let second = "01a1d02e-eedb-7995-808d-0ba60b4a6dd4";
/// ```
///
/// If you require strict sub-millisecond monotonicity, use [`generate_strict()`](#generate_strict---string).
///
/// [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure. **To use cryptographically secure randomness, enable the [`v7-crypto-rand`](#v7-crypto-rand) feature flag.**
///
/// ## `generate_strict() -> String`
///
/// ```rust
/// use gwid;
/// let uuidv7 = gwid::v7::generate_strict().expect("error generating strict");
/// ```
///
/// Guarantees **[strict](#monotonic-precision) thread-local monotonicity**. By default, monotonicity is maintained independently per thread. To guarantee **strict global monotonicity across threads**, enable the [`v7-global-monotonicity`](#v7-global-monotonicity) feature flag.
///
/// [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure. **To use cryptographically secure randomness, enable the [`v7-crypto-rand`](#v7-crypto-rand) feature flag.**
///
/// ## `generate_raw() -> u128`
///
/// Behaves exactly like [`generate()`](#generate---string-1), but returns the raw `u128` UUID. Use [`encode`](#encodeuuid-u128---string) to format raw `u128` as UUID.
///
/// ## `generate_strict_raw() -> u128`
///
/// Behaves exactly like [`generate_strict()`](#generate_strict---string), but returns the raw `u128` UUID. Use [`encode`](#encodeuuid-u128---string) to format raw `u128` as UUID.
///
/// ## Monotonic Precision
///
/// | Feature                | `generate()`                         | `generate_strict()`               |
/// | ---------------------- | ------------------------------------ | --------------------------------- |
/// | Default                | Thread-local timestamp* monotonicity | Thread-local strict* monotonicity |
/// | v7-global-monotonicity | Thread-local timestamp* monotonicity | Global strict* monotonicity       |
///
/// \*_timestamp_ - guarantees sub-millisecond timestamp precision, but **does not guarantee monotonicity within the same sub-millisecond timestamp bucket**
/// <br>\*_strict_ - guarantees monotonicity within same sub-millisecond timestamp bucket
///
/// # Helper Functions
///
/// ## `encode(uuid: u128) -> String`
///
/// Encode a raw `u128` as UUID `String`. Formats as hexadecimal and inserts hyphens at proper indices.
///
/// ```rust
/// use gwid;
/// let raw_uuid: u128 = 54839052801522565720884485497537422415;
/// let encoded = gwid::encode(raw_uuid);
/// ```
///
/// ## `decode(uuid: &str) -> Result<u128, GWIDError>`
///
/// Get the underlying raw `u128` value from a UUID.
///
/// ```rust
/// use gwid;
/// let uuid_str: &str = "a60d0392-a436-4e0f-b560-9f763b1d99a1";
/// let raw_uuid = gwid::decode(uuid_str).expect("raw uuid");
/// ```
///
/// ## `decode_unchecked(uuid: &str) -> u128`
///
/// **Panics if something goes wrong during parsing!**
///
/// Get the underlying raw `u128` value from a UUID.
///
/// ```rust
/// use gwid;
/// let uuid_str: &str = "a60d0392-a436-4e0f-b560-9f763b1d99a1";
/// let raw_uuid = gwid::decode_unchecked(uuid_str);
/// ```
///
/// # Cargo Features
///
/// | Feature                  | Default  | Description                                                                 |
/// | ------------------------ | -------- | --------------------------------------------------------------------------- |
/// | `v7-global-monotonicity` | Disabled | Uses shared synchronization to maintain UUIDv7 monotonicity across threads. |
/// | `v7-crypto-rand`         | Disabled | Uses cryptographically secure random bits for `random_b`                    |
///
/// ## `v7-global-monotonicity`
///
/// Enable the feature in your `Cargo.toml`:
///
/// ```toml
/// [dependencies]
/// gwid = { version = "0.1", features = ["v7-global-monotonicity"] }
/// ```
///
/// When enabled (default), UUIDv7 generation uses a shared `Mutex` to synchronize generator state across threads, ensuring globally monotonic UUID generation within the process.
///
/// When disabled, each thread maintains its own independent generator state using thread-local storage. This eliminates mutex overhead and can improve single-threaded and concurrent generation performance, but **monotonicity is only guaranteed within each individual thread**, not across threads.
///
/// ## `v7-crypto-rand`
///
/// Enable the feature in your `Cargo.toml`:
///
/// ```toml
/// [dependencies]
/// gwid = { version = "0.1", features = ["v7-crypto-rand"] }
/// ```
///
/// When enabled, UUIDv7 generation will use cryptographically secure bits for randomness (specifically, the `random_b` field).
///
/// [RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure, but we give you the option.
pub mod v4;
pub mod v7;

pub(crate) mod nums;

/// Encodes a `u128` as UUID.
/// Formats as hexadecimal and inserts hyphens at proper indices.
pub fn encode(n: u128) -> String {
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
/// Decodes a UUID `String` into a `u128`.
pub fn decode_unchecked(uuid: &str) -> u128 {
    u128::from_str_radix(&uuid.replace("-", ""), 16).unwrap()
}

/// Decodes a UUID `String` into a `u128`.
pub fn decode(uuid: &str) -> Result<u128, GWIDError> {
    Ok(u128::from_str_radix(&uuid.replace("-", ""), 16)?)
}

// Clears UUID Version bits (bits 48, 49, 50, and 51 when counting left->right and starting at 0)
// 11111111111111111111111111111111111111111111111100001111111111111111111111111111111111111111111111111111111111111111111111111111
pub(crate) const CLEAR_VERSION_MASK: u128 = 0xFFFFFFFFFFFF0FFFFFFFFFFFFFFFFFFF;
// Clears UUID Variant bits (bits 64 and 65 when counting left->right and starting at 0)
// 11111111111111111111111111111111111111111111111111111111111111110011111111111111111111111111111111111111111111111111111111111111
pub(crate) const CLEAR_VARIANT_MASK: u128 = 0xFFFFFFFFFFFFFFFF3FFFFFFFFFFFFFFF;
pub(crate) const VERSION_SHIFT: u128 = 76;
pub(crate) const VARIANT_SHIFT: u128 = 62;

#[derive(Debug)]
pub enum GWIDError {
    ParseUUIDError(std::num::ParseIntError),
}

impl std::fmt::Display for GWIDError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GWIDError::*;
        match self {
            ParseUUIDError(parse_int_error) => write!(f, "Error parsing UUID {parse_int_error}"),
        }
    }
}

impl From<std::num::ParseIntError> for GWIDError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::ParseUUIDError(error)
    }
}

impl std::error::Error for GWIDError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseUUIDError(parse_int_err) => Some(parse_int_err),
            // _ => None,
        }
    }
}

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
    fn encode_raw_uuid() {
        let raw: u128 = 213409605541016776915642275756511741574;
        let expect: &str = "a08d31e9-c256-443e-8425-73c382264686";
        let got = encode(raw);
        assert_eq!(
            expect, &got,
            "add_hyphens_to_raw_uuid | expected= {expect} got= {got}"
        );
    }

    // Tests both unchecked and "regular" variants.
    #[test]
    fn decode_u128() {
        let uuid = "a08d31e9-c256-443e-8425-73c382264686";
        let expect: u128 = 213409605541016776915642275756511741574;
        let got = decode_unchecked(uuid);
        assert_eq!(
            expect, got,
            "convert_uuid_to_u128_unchecked | expected= {expect} got= {got}"
        );
        let got = decode(uuid).unwrap();
        assert_eq!(
            expect, got,
            "convert_uuid_to_u128 | expected= {expect} got= {got}"
        );
    }
}
