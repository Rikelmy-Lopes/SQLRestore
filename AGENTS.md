# AGENTS.md

Rules for any agent working on SQLRestore. This file is the single source of rules. Other docs link here.

## Project

SQLRestore is a desktop app (Rust 1.99.x + egui/eframe) that restores `.sql` backup files on PostgreSQL, using the database connection. The application must be cross-platform.

## Mandatory Rules

1. **Architecture:** Commands Pattern + Clean Architecture + Single Responsibility Principle.
2. **Core crate:** The application core lives in a separate crate (`crates/core`). It must not depend on egui.
3. **Single responsibility:** Each file and folder contains only related methods or classes.
4. **Modules:** Every module folder uses `mod.rs`. Do not use the `folder.rs` style.
5. **Restore:** The backup is restored using the database connection. The restore does not use a transaction. If an error occurs or the user cancels, the restore stops.
6. **Errors:** Do not use `panic!` or `unwrap()` in the code. They are allowed only in tests. Return errors instead.
7. **Tests:** Every new feature has tests. All tests must pass after any change, bug fix or feature.
8. **i18n:** No raw text in the code. All user-visible text comes from `.json` files in `locales/`. English is the primary language.
9. **Config:** Settings (theme, language, connection data) are saved in a `config.json` file, in the AppData folder of each system. The password is not saved in `config.json`. It is saved in the system keychain (Windows Credential Manager, macOS Keychain, Linux Secret Service), using the `PasswordStore` port. If the file is missing, it is created with default values. If a value is invalid, only that value is restored to its default.
10. **UI:** Simple interface with light and dark themes. A cancel restore button must be present.
11. **Log:** The program writes a log file in the AppData folder of each system. Record the start, end, cancel and errors of each restore. Never write passwords or other secrets in the log. Limit the file size and rotate old files. The core uses the `Logger` port.
12. **Status:** After finishing a feature, update `docs/status.md`.

## Before Finishing Any Task

```bash
cargo fmt --all
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

All commands must pass.

## Documentation

- [Architecture](docs/architecture.md)
- [Development](docs/development.md)
- [Status](docs/status.md)
- [UI Design](docs/ui-design.md)
