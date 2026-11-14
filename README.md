# gwid

[![Crates.io Version](https://img.shields.io/crates/v/gwid?style=flat-square&logoSize=auto&color=%233d7ae3)](https://crates.io/crates/gwid) [![docs.rs](https://img.shields.io/docsrs/gwid?style=flat-square&label=docs.rs)](https://docs.rs/gwid/latest/gwid/)

Implementing [RFC 9562](https://www.rfc-editor.org/info/rfc9562/) ccompliant UUIDs (also known as GUIDs). The name gwid is derived from the phonetic pronunciation of GUID.

# UUID Version 4

UUIDv4 is a 128-bit UUID generated using cryptographically secure random bits, providing a high degree of uniqueness without encoding a timestamp or providing chronological ordering.

## `generate() -> String`

**Usage**

```rust
use gwid;
let uuid = gwid::v4::generate();
```

## `generate_raw() -> u128`

Returns the raw `u128` UUID. If you need to format it as a hexadecimal encoded `String` with hyphens, please use [`add_hyphens`](#add_hyphensuuid-u128---string).

**Usage**

```rust
use gwid;
let uuid = gwid::v4::generate_raw();
let with_hyphens = gwid::add_hyphens(uuid);
```

# UUID Version 7

UUIDv7 is a 128-bit, time-ordered UUID that embeds a Unix timestamp (milliseconds) and uses random bits for uniqueness. Its time-based structure makes UUIDs naturally sortable by creation time.

## `generate() -> String`

```rust
use gwid;
let uuidv7 = gwid::v7::generate().expect("error generating");
```

Generates UUIDv7 values with sub-millisecond [timestamp](#monotonic-precision) precision. Although thread-safe, UUIDs generated across multiple threads are not guaranteed to be globally monotonic.

For example, two UUIDs generated within the same sub-millisecond bucket may be out of order:

```rust
// First UUID (greater)
let first = "01a1d02e-eedb-7995-82da-278257b8340d";
// Second UUID (smaller)
let second = "01a1d02e-eedb-7995-808d-0ba60b4a6dd4";
```

If you require strict sub-millisecond monotonicity, use [`generate_strict()`](#generate_strict---string).

[RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure. **To use cryptographically secure randomness, enable the [`v7-crypto-rand`](#v7-crypto-rand) feature flag.**

## `generate_strict() -> String`

```rust
use gwid;
let uuidv7 = gwid::v7::generate_strict().expect("error generating strict");
```

Guarantees **[strict](#monotonic-precision) thread-local monotonicity**. By default, monotonicity is maintained independently per thread. To guarantee **strict global monotonicity across threads**, enable the [`v7-global-monotonicity`](#v7-global-monotonicity) feature flag.

[RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure. **To use cryptographically secure randomness, enable the [`v7-crypto-rand`](#v7-crypto-rand) feature flag.**

## `generate_raw() -> u128`

Behaves exactly like [`generate()`](#generate---string-1), but returns the raw `u128` UUID. If you need to format the raw UUID as a hexadecimal encoded `String` with hyphens, please use [`add_hyphens`](#add_hyphensuuid-u128---string).

## `generate_strict_raw() -> u128`

Behaves exactly like [`generate_strict()`](#generate_strict---string), but returns the raw `u128` UUID. If you need to format the raw UUID as a hexadecimal encoded `String` with hyphens, please use [`add_hyphens`](#add_hyphensuuid-u128---string).

## Monotonic Precision

| Feature                | `generate()`                         | `generate_strict()`               |
| ---------------------- | ------------------------------------ | --------------------------------- |
| Default                | Thread-local timestamp* monotonicity | Thread-local strict* monotonicity |
| v7-global-monotonicity | Thread-local timestamp* monotonicity | Global strict* monotonicity       |

\*_timestamp_ - guarantees sub-millisecond timestamp precision, but **does not guarantee monotonicity within the same sub-millisecond timestamp bucket**
<br>\*_strict_ - guarantees monotonicity within same sub-millisecond timestamp bucket

# Helper Functions

## `add_hyphens(uuid: u128) -> String`

Format a raw `u128` UUID to a hexadecimal encoded `String` with hyphens.

```rust
use gwid;
let raw_uuid: u128 = 54839052801522565720884485497537422415;
let with_hyphens = gwid::add_hyphens(raw_uuid);
```

## `uuid_to_raw(uuid: &str) -> Result<u128, ParseIntError>`

Get the underlying raw `u128` value from a UUID.

```rust
use gwid;
let uuid_str: &str = "a60d0392-a436-4e0f-b560-9f763b1d99a1";
let raw_uuid = gwid::uuid_to_raw(uuid_str).expect("raw uuid");
```

## `uuid_to_raw_unchecked(uuid: &str) -> u128`

**Panics if something goes wrong during parsing!**

Get the underlying raw `u128` value from a UUID.

```rust
use gwid;
let uuid_str: &str = "a60d0392-a436-4e0f-b560-9f763b1d99a1";
let raw_uuid = gwid::uuid_to_raw_unchecked(uuid_str);
```

# Cargo Features

| Feature                  | Default  | Description                                                                 |
| ------------------------ | -------- | --------------------------------------------------------------------------- |
| `v7-global-monotonicity` | Disabled | Uses shared synchronization to maintain UUIDv7 monotonicity across threads. |
| `v7-crypto-rand`         | Disabled | Uses cryptographically secure random bits for `random_b`                    |

## `v7-global-monotonicity`

Enable the feature in your `Cargo.toml`:

```toml
[dependencies]
gwid = { version = "0.1", features = ["v7-global-monotonicity"] }
```

When enabled (default), UUIDv7 generation uses a shared `Mutex` to synchronize generator state across threads, ensuring globally monotonic UUID generation within the process.

When disabled, each thread maintains its own independent generator state using thread-local storage. This eliminates mutex overhead and can improve single-threaded and concurrent generation performance, but **monotonicity is only guaranteed within each individual thread**, not across threads.

## `v7-crypto-rand`

Enable the feature in your `Cargo.toml`:

```toml
[dependencies]
gwid = { version = "0.1", features = ["v7-crypto-rand"] }
```

When enabled, UUIDv7 generation will use cryptographically secure bits for randomness (specifically, the `random_b` field).

[RFC 9562](https://www.rfc-editor.org/rfc/rfc9562/) does not require UUIDv7 random bits to be cryptographically secure, but we give you the option.
