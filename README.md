# SQLRestore

Cross-platform desktop application to restore `.sql` backup files on PostgreSQL. The backup is restored using the database connection. Other databases may be supported in the future.

## Features

- Restore `.sql` files on PostgreSQL using the database connection.
- Optional delete and recreate of the target database before a restore, with a confirmation dialog.
- Cancel a restore in progress.
- Light and dark themes.
- Multiple languages (English is the primary language).
- Configuration saved in a `config.json` file in the local application data folder of each system. The password is saved in the system keychain, not in the file.
- Log file in the local application data folder of each system, without secrets.
- Restore history (last 100 successful restores) saved in `history.json`, with a dialog to view it.
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

## Installers

Installers are built with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager). Install the tool first:

```bash
cargo install cargo-packager --locked
```

Windows (NSIS `setup.exe`):

```bash
cargo packager -p app --release -f nsis
```

Linux (`.deb` and `.AppImage`):

```bash
cargo packager -p app --release -f deb,appimage
```

Packages are written to `dist/`. On Windows, the NSIS toolchain is downloaded automatically. On Linux, install the development libraries required to build eframe, plus `patchelf` and `file` for AppImage packaging. Tagged releases (`v*`) build both platforms in GitHub Actions.

## Documentation

- [AGENTS.md](AGENTS.md) (all rules)
- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Status](docs/status.md)
- [UI Design](docs/ui-design.md)
