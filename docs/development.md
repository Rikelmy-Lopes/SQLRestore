# Development

## Setup

1. Install Rust 1.99.x.
2. Clone the repository.

## Commands

| Task | Command |
|------|---------|
| Test | `cargo test --workspace` |
| Format | `cargo fmt --all` |
| Lint | `cargo clippy --workspace -- -D warnings` |
| Release build | `cargo build --release` |

## Rules

- All mandatory rules are in [AGENTS.md](../AGENTS.md).
- Follow [Architecture](architecture.md).
- Core logic goes in `crates/core`. UI goes in `crates/app`.

## Adding a Feature

1. Implement a new feature or bug fix.
2. Add the UI in `crates/app`.
3. Add translation keys to all files in `locales/`.
4. Write tests.
5. Update [Status](status.md).

## Tests

- Unit tests stay next to the code. Integration tests stay in `tests/`.
- `unwrap()` and `panic!` are allowed in tests.
