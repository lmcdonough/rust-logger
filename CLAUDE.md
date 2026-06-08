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

Single-binary crate (`src/main.rs`). As the logger grows it will likely expand into modules under `src/` — e.g. `src/logger.rs`, `src/level.rs` — and be pulled into `main.rs` with `mod` declarations.

## Rust Edition

Uses edition 2024 (`Cargo.toml`). No external dependencies yet.
