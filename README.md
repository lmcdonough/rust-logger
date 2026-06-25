# rust-logger

A logging system built in Rust from scratch.

## Modules

| Module | Description |
|---|---|
| `circular_buffer` | Generic fixed-capacity ring buffer. Overwrites oldest entry when full. |
| `kv_store` | Key-value store with optional TTL (time-to-live). Entries can be permanent or expire after a set duration. |
| `log_parser` | Parses `[LEVEL] timestamp source: message` log lines into `LogEntry` structs, counts entries by level, ranks top sources by level, and implements `Display` for a summary report. |
| `rolling_window` | Generic fixed-size sliding window (`RollingWindow<T>`) over numeric values, backed by a `VecDeque`. Evicts the oldest value when full and computes aggregate stats (min/max/mean/sum/count), plus `mean` and `count_above` convenience methods. |
| `rate_limiter` | Sliding-window rate limiter (`RateLimiter`) backed by a `VecDeque` of Unix timestamps. Evicts expired timestamps before each `check`, rejecting requests over capacity with a typed `RateLimitError` (via `thiserror`). Exposes `check`/`allow`, `current_count`, `remaining`, `is_throttled`, and `reset`. |

## Topics Covered

- Rust project structure and Cargo
- Generics and type parameters
- Structs, methods, and associated functions
- Ownership, borrowing, and references
- Enums and pattern matching
- Traits and implementations
- Error handling
- File I/O and output formatting
- Time and duration (SystemTime, Duration, UNIX_EPOCH)
- HashMap operations and iteration patterns
- VecDeque and sliding-window data structures
- Trait bounds for generic numeric code (`Copy + Into<f64>`)
- Iterator adaptors and folds (sum, fold, filter, map)
- Custom error types with `thiserror` (derived `Display` + `Error`)
- Sliding-window rate limiting over Unix timestamps

## Running

```bash
cargo run
```

## Building

```bash
cargo build
```

## Testing

```bash
cargo test
```
