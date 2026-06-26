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
| `lru_cache` | LRU cache (`LruCache`) pairing a `HashMap` for O(1) key lookup with a `BTreeMap` keyed by a monotonic access counter for O(log n) eviction of the least-recently-used entry. `ThreadSafeLruCache` wraps it in `Arc<Mutex<_>>` for cheap-clone sharing across threads. |

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
- BTreeMap for ordered access and O(log n) min/oldest lookup
- LRU cache design (HashMap + BTreeMap recency tracking)
- Thread-safe shared state with `Arc<Mutex<T>>`

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
