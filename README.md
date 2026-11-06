# SQLRestore

Cross-platform desktop application to restore `.sql` backup files on PostgreSQL. The backup is restored using the database connection. Other databases may be supported in the future.

## Features

- Restore `.sql` files on PostgreSQL using the database connection.
- Optional delete and recreate of the target database before a restore, with a confirmation dialog.
- Cancel a restore in progress.
- Light and dark themes.
- Multiple languages (English is the primary language).
- Configuration saved in a `config.json` file in the AppData folder of each system. The password is saved in the system keychain, not in the file.
- Log file in the AppData folder of each system, without secrets.
- Cross-platform.

## Requirements

- Rust 1.99.x

## Stack

- Language: Rust
- UI: egui (eframe)
- Architecture: Commands Pattern + Clean Architecture + Single Responsibility Principle

## Build and Run

```bash
cargo run -p app
cargo test --workspace
cargo build --release
```

## Documentation

- [AGENTS.md](AGENTS.md) (all rules)
- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Status](docs/status.md)
- [UI Design](docs/ui-design.md)
