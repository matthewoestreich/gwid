# UUID

Just messing around trying to implement UUIDs in Rust.

[UUID RFC/spec](https://www.rfc-editor.org/info/rfc9562/)

# UUID Version 4

UUIDv4 is a 128-bit UUID generated using cryptographically secure random bits, providing a high degree of uniqueness without encoding a timestamp or providing chronological ordering.

## `generate()`

```rust
use rs_uuid;
let my_id = rs_uuid::v4::generate();
```

# UUID Version 7

UUIDv7 is a 128-bit, time-ordered UUID that embeds a Unix timestamp (milliseconds) and uses random bits for uniqueness. Its time-based structure makes UUIDs naturally sortable by creation time.

| Feature                | `generate()`                        | `generate_strict()`              |
| ---------------------- | ----------------------------------- | -------------------------------- |
| Default                | Thread-local timestamp monotonicity | Thread-local strict monotonicity |
| v7-global-monotonicity | Thread-local timestamp monotonicity | Global strict monotonicity       |

## `generate()`

Generates UUIDv7 values with sub-millisecond timestamp precision, but **does not guarantee monotonicity within the same sub-millisecond timestamp bucket**.

Although thread-safe, UUIDs generated across multiple threads are not guaranteed to be globally monotonic.

For example, two UUIDs generated within the same sub-millisecond bucket may be out of order:

```rust
// First UUID (greater)
let first = "01a1d02e-eedb-7995-82da-278257b8340d";

// Second UUID (smaller)
let second = "01a1d02e-eedb-7995-808d-0ba60b4a6dd4";
```

If you require strict sub-millisecond monotonicity, use [`generate_strict()`](#generatestrict).

### Usage

```rust
use rs_uuid;

let uuidv7 = rs_uuid::v7::generate().expect("error generating");
```

## `generate_strict()`

```rust
use rs_uuid;

let uuidv7 = rs_uuid::v7::generate_strict().expect("error generating strict");
```

Guarantees **strict thread-local monotonicity**, including when multiple UUIDs are generated within the same sub-millisecond timestamp bucket.

By default, monotonicity is maintained independently per thread. To guarantee **strict global monotonicity across threads**, enable the [`v7-global-monotonicity`](#v7-global-monotonicity) feature flag.

# Cargo Features

| Feature                  | Default  | Description                                                                 |
| ------------------------ | -------- | --------------------------------------------------------------------------- |
| `v7-global-monotonicity` | Disabled | Uses shared synchronization to maintain UUIDv7 monotonicity across threads. |

## `v7-global-monotonicity`

Enable the feature in your `Cargo.toml`:

```toml
[dependencies]
uuid-rs = { version = "0.1", features = ["v7-global-monotonicity"] }
```

When enabled (default), UUIDv7 generation uses a shared `Mutex` to synchronize generator state across threads, ensuring globally monotonic UUID generation within the process.

When disabled, each thread maintains its own independent generator state using thread-local storage. This eliminates mutex overhead and can improve single-threaded and concurrent generation performance, but **monotonicity is only guaranteed within each individual thread**, not across threads.
