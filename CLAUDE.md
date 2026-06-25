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

As the logger grows, expect modules like `src/level.rs` (log levels) and `src/logger.rs` (the logger itself that uses `CircularBuffer`).

## Rust Edition

Uses edition 2024 (`Cargo.toml`). One external dependency: `thiserror` (used by `rate_limiter` for ergonomic error enums). Otherwise std-only.
