# rust-logger

A logging system built in Rust from scratch.

## Modules

| Module | Description |
|---|---|
| `circular_buffer` | Generic fixed-capacity ring buffer. Overwrites oldest entry when full. |
| `kv_store` | Key-value store with optional TTL (time-to-live). Entries can be permanent or expire after a set duration. |
| `log_parser` | Parses `[LEVEL] timestamp source: message` log lines into `LogEntry` structs, counts entries by level, ranks top sources by level, and implements `Display` for a summary report. |

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
