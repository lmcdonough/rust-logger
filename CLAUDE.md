# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # compile
cargo run            # build and run
cargo test           # run all tests
cargo test <name>    # run a single test by name (substring match)
cargo check          # fast type-check without producing a binary
cargo clippy         # lint
```

## Project Intent

This is a hands-on Rust learning project. The user is practicing writing Rust by hand — no autocomplete, no inlay hints, no AI suggestions. The `.vscode/settings.json` intentionally disables rust-analyzer, copilot, and all editor assistance.

When helping here: explain concepts, point to the right module or standard library type, but don't silently write large blocks of code. Prefer short targeted edits that the user can study and type out themselves. The goal is understanding, not output.

## Structure

Single-binary crate. Modules are declared in `src/main.rs` with `mod <name>;` and implemented in their own files under `src/`.

Current modules:
- `src/circular_buffer.rs` — generic fixed-capacity ring buffer (`CircularBuffer<T>`). Supports `push` (overwrites oldest when full), `get` by logical index (0 = oldest), `iter`, `len`, `is_empty`, `is_full`.
- `src/kv_store.rs` — `KvStore` with optional TTL support. Store values with or without expiry times. Methods: `set` (store with optional TTL), `get` (returns `Option<&str>`, respects expiry), `delete` (remove key), `purge_expired` (clean up expired entries), `len` (total keys), `active_len` (non-expired only), `contains_key` (exists and not expired).
- `src/log_parser.rs` — `LogParser` and `LogEntry`. Parses `[LEVEL] timestamp source: message` lines into owned `LogEntry` structs via `parse_line` (returns `Option<LogEntry>`), aggregates with `count_by_level` (`HashMap<String, usize>`) and `top_n_sources`, and implements `fmt::Display` for a summary report.
- `src/rolling_window.rs` — generic fixed-size sliding window (`RollingWindow<T>` where `T: Copy + Into<f64>`) backed by a `VecDeque`. `push` evicts the oldest value when at capacity. Computes aggregates via `stats` (returns `Option<WindowStats>` with min/max/mean/sum/count), plus convenience methods `mean` (`Option<f64>`) and `count_above` (count of values over a threshold). Also `len`, `is_empty`, `is_full`, `values`.
- `src/rate_limiter.rs` — sliding-window `RateLimiter` backed by a `VecDeque<u64>` of Unix timestamps (from `SystemTime`). `new` validates config and returns `Result<Self, RateLimitError>`; `check` evicts expired timestamps then allows (`Ok(())`) or rejects (`Err(RateLimitError::LimitExceeded { .. })`) based on `capacity`/`window_secs`. Also `allow` (bool alias), `current_count`, `remaining`, `is_throttled`, `reset`, and `capacity`/`window_secs` accessors. `RateLimitError` is a `thiserror`-derived enum (`LimitExceeded`, `InvalidConfig`).
- `src/lru_cache.rs` — `LruCache` (LRU eviction) backed by two synchronized structures: a `HashMap<String, (String, u64)>` for O(1) lookup by key and a `BTreeMap<u64, String>` mapping a monotonic access counter to keys for O(log n) access to the oldest entry. `get` returns `Option<String>` and refreshes recency; `put` inserts/updates and evicts the LRU entry at capacity. Also `len`, `is_empty`. `ThreadSafeLruCache` wraps it in `Arc<Mutex<LruCache>>` for cheap-clone sharing across threads, exposing `get`/`put`/`len`.
- `src/merge_stream.rs` — K-way merge of pre-sorted log streams. `LogRecord` derives `Ord` with field order (timestamp, stream_id, message) driving comparison. `HeapItem` wraps a record + originating stream index and has a hand-written reversed `Ord` so a `BinaryHeap` (max-heap) behaves as a min-heap. `merge_k_streams` seeds the heap with one record per stream and refills from the popped record's stream, returning a fully sorted `Vec<LogRecord>` in O(N log K) time / O(K) space. `MergeIter` implements `Iterator<Item = LogRecord>` over `Box<dyn Iterator>` streams for a lazy version, exposed via `merge_streaming` (returns `impl Iterator`).
- `src/concurrent_kv.rs` — thread-safe `ConcurrentKvStore` built on the `Arc<RwLock<HashMap<String, StoreEntry>>>` pattern. `#[derive(Clone)]` clones are cheap (just bump the `Arc`) and share one heap-allocated map. `get`/`active_len` take a read lock (concurrent readers); `set`/`delete`/`purge_expired` take a write lock (exclusive). `StoreEntry` is a `WithTTL { value, expires_at }` / `Permanent { value }` enum with `is_expired`/`value` helpers; `get` returns owned `Option<String>` so the lock releases immediately.
- `src/concurrent_rate_limiter.rs` — thread-safe `ConcurrentRateLimiter` wrapping a private `RateLimiterInner` (same sliding-window logic as `rate_limiter`) in `Arc<Mutex<_>>`. The mutex makes check-and-record a single atomic operation, closing the TOCTOU race that would otherwise let concurrent callers exceed capacity. `new` asserts `capacity > 0` and `window_secs > 0`; exposes `allow` (bool), `remaining`, `reset`.
- `src/log_pipeline.rs` — multi-producer, single-consumer `LogPipeline` over an `mpsc::channel::<LogLine>`. `new` spawns a consumer thread that drains lines into a `Vec` until every sender drops (the `for line in rx` loop ends). The sender is held in `Arc<Mutex<mpsc::Sender>>`; `producer()` returns a cloned `Sender` to move into producer threads. `shutdown(self)` drops the pipeline's own sender, `join`s the consumer (`JoinHandle` kept in an `Option` so it can be `take`n), and returns the collected `Vec<LogLine>`. `LogLine { source, content }` is `Send`.

The `concurrent_*` and `log_pipeline` modules make up a concurrency milestone: `Arc<RwLock<_>>` for read-heavy shared state, `Arc<Mutex<_>>` for atomic check-and-record, and `mpsc` channels for message passing between threads. The `log_pipeline` tests exercise all three (they import `ConcurrentKvStore` and `ConcurrentRateLimiter` from their sibling modules).

As the logger grows, expect modules like `src/level.rs` (log levels) and `src/logger.rs` (the logger itself that uses `CircularBuffer`).

## Rust Edition

Uses edition 2024 (`Cargo.toml`). One external dependency: `thiserror` (used by `rate_limiter` for ergonomic error enums). Otherwise std-only.
