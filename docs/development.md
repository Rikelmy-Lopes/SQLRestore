# Development

## Setup

1. Install Rust 1.99.x.
2. Clone the repository.

## Project Structure

The workspace contains two crates. Infrastructure belongs inside `crates/core`; it is a module of the core crate, not a separate crate.

```text
SQLRestore/
├── crates/
│   ├── core/
│   └── app/
├── locales/
├── docs/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
└── AGENTS.md
```

Follow the dependency direction and module responsibilities described in [Architecture](architecture.md). Use `mod.rs` for module folders, as required by [AGENTS.md](../AGENTS.md).

## Commands

| Task | Command |
|------|---------|
| Test | `cargo test --workspace` |
| Format | `cargo fmt --all` |
| Check formatting | `cargo fmt --all -- --check` |
| Lint | `cargo clippy --workspace -- -D warnings` |
| Check compilation | `cargo check --workspace` |
| Release build | `cargo build --release` |

## Rules

- All mandatory rules are in [AGENTS.md](../AGENTS.md).
- Follow [Architecture](architecture.md).
- Core logic and infrastructure implementations go in `crates/core`.
- UI code goes in `crates/app`.

## Adding a Feature

1. Understand the existing architecture and requirements.
2. Implement business logic in the appropriate `crates/core` module.
3. Implement required infrastructure adapters inside `crates/core/src/infrastructure`.
4. Add the UI in `crates/app` when necessary.
5. Add translation keys to all files in `locales/`.
6. Write unit tests.
7. Verify error handling and cancellation when applicable.
8. Run formatting, Clippy, and the complete test suite.
9. Update relevant documentation.
10. Update [Status](status.md).

## Tests

- Unit tests stay next to the code.
- `unwrap()` and `panic!` are allowed in tests.
