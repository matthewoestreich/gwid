# UUID

Just messing around trying to implement UUIDs in Rust.

[UUID RFC/spec](https://www.rfc-editor.org/info/rfc9562/)

# Usage

## UUID Version 4

- Uses cryptographically secure randomness.

```rust
use rs_uuid;
let my_id = rs_uuid::v4::generate();
```

## UUID Version 7

### generate()

```rust
use rs_uuid;
let uuidv7 = rs_uuid::v7::generate().expect("error generating");
```

Sub-millisecond timestamp precision with **no monotonicity guarantee within the same sub-ms bucket**. Meaning, if 2 UUIDs are created within the same sub-millisecond bucket, those UUIDs are not guaranteed to be monotonic.

While it is thread-safe, UUIDs generated across multiple threads are not guaranteed to have global sub-millisecond timestamp monotonicity! If you require **global, sub-millisecond timestamp precision with same sub-millisecond bucket monotonicity**, use the [`generate_strict()`](#generatestrict) method with the [global-monotonicity](#v7-global-monotonicity) feature flag!

The following example displays what non-sub-millisecond monotonicity looks like, eg; the first UUID is greater than the second even though they were generated in the same sub-millisecond bucket!:

```rust
// `first` raw `u128` value  =  2169410249624819581018843866751054861
let first  = "01a1d02e-eedb-7995-82da-278257b8340d";
// `second` raw `u128` value =  2169410249624819580853024472479460820
let second = "01a1d02e-eedb-7995-808d-0ba60b4a6dd4";
```

### generate_strict()

Strict **thread-local** monotonicity, including when multiple UUIDs are generated within the same sub-millisecond bucket. If you require **strict global, thread-safe monotonicity**, you can enable the [global-monotonicity](#v7-global-monotonicity) feature flag.

```rust
use rs_uuid;
let uuidv7 = rs_uuid::v7::generate_strict().expect("error generating strict");
```

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
