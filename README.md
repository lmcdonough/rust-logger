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
| `merge_stream` | K-way merge of pre-sorted `LogRecord` streams using a `BinaryHeap` min-heap (one candidate per stream). `merge_k_streams` returns a fully sorted `Vec`; `merge_streaming` / `MergeIter` provide a lazy `Iterator` version. Runs in O(N log K) time, O(K) space. |
| `concurrent_kv` | Thread-safe key-value store (`ConcurrentKvStore`) using the `Arc<RwLock<HashMap>>` pattern for concurrent reads and exclusive writes. Cheap to clone (shares one heap-allocated map). Supports optional TTL, `get`/`set`/`delete`/`purge_expired`/`active_len`. |
| `concurrent_rate_limiter` | Thread-safe sliding-window rate limiter (`ConcurrentRateLimiter`) wrapping the inner limiter in `Arc<Mutex<_>>`. The lock makes check-and-record atomic, closing the TOCTOU race that would otherwise let concurrent callers exceed capacity. |
| `log_pipeline` | Multi-producer, single-consumer log pipeline (`LogPipeline`) over an `mpsc` channel. A consumer thread drains `LogLine`s until all senders drop; `producer()` hands out cloneable senders and `shutdown()` joins the consumer and returns the collected lines. |

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
- Thread-safe shared state with `Arc<Mutex<T>>` and `Arc<RwLock<T>>`
- Concurrent reads vs. exclusive writes (`RwLock` read/write locks)
- Avoiding TOCTOU races by making check-and-record atomic under a lock
- Message passing with `mpsc` channels (multi-producer, single-consumer)
- Spawning threads, cloning senders, and joining with `JoinHandle`
- K-way merge with a `BinaryHeap` min-heap (custom reversed `Ord`)
- Implementing the `Iterator` trait and returning `impl Iterator`
- Trait objects (`Box<dyn Iterator>`) for type erasure
- Derived `Ord` and field-order-driven comparison

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
