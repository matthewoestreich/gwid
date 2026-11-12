# UUID

Just messing around trying to implement UUIDs in Rust.

[UUID RFC/spec](https://www.rfc-editor.org/info/rfc9562/)

# Usage

## UUID Version 4

```rust
use rs_uuid;

let my_id = rs_uuid::v4::generate();
```

## UUID Version 7

By default we offer sub-millisecond monotonicity ***per thread**. If you need

### Sub-Millisecond Monotonicity Precision

# Cargo Features

| Feature               | Default  | Description                                                                 |
| --------------------- | -------- | --------------------------------------------------------------------------- |
| `global-monotonicity` | Disabled | Uses shared synchronization to maintain UUIDv7 monotonicity across threads. |

## `global-monotonicity`

When enabled (default), UUIDv7 generation uses a shared `Mutex` to synchronize generator state across threads, ensuring globally monotonic UUID generation within the process.

When disabled, each thread maintains its own independent generator state using thread-local storage. This eliminates mutex overhead and can improve single-threaded and concurrent generation performance, but **monotonicity is only guaranteed within each individual thread**, not across threads.

Enable the feature in your `Cargo.toml`:

```toml
[dependencies]
uuid-rs = { version = "0.1", features = ["global-monotonicity"] }
```
