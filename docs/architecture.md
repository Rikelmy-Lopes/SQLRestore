# Architecture

Mandatory rules are in [AGENTS.md](../AGENTS.md).

## Patterns

- **Clean Architecture:** dependencies point inward. Inner layers do not know outer layers.
- **Commands Pattern:** each user action is a command with one handler.
- **Single Responsibility Principle:** one purpose per file, folder, struct and function.

## Crates

| Crate | Purpose |
|-------|---------|
| `crates/core` | Business logic, ports, infrastructure adapters, configuration. No UI or egui dependency. |
| `crates/app` | egui (eframe) interface, application entry point, commands and i18n. Calls the core through its public command interfaces. |

Infrastructure is implemented inside `crates/core`, under its `infrastructure` module. It is not a separate workspace crate.

## Project Structure

```text
SQLRestore/
├── crates/
│   ├── core/
│   │   └── src
│   └── app/
│       └── src
│           ├── i18n
│           ├── startup
│           ├── ui
│           ├── commands
│           └── main.rs
│
├── locales/
├── docs/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── README.md
└── AGENTS.md
```

This is the intended organization. Add modules under the relevant directory as the implementation grows. Use `mod.rs` for module folders, as required by `AGENTS.md`.

## Restore

The backup is restored using the database connection. The adapter reads the `.sql` file as a stream, statement by statement. `COPY ... FROM stdin` data is streamed directly to the database in bounded chunks, so large backups are not loaded into memory as a whole. The restore does not use a transaction. If an error occurs or the user cancels, the restore stops.

## Ports

- `DatabaseRestorer`: restores a file using a database connection and supports cancel. Each supported database has its own adapter (first: PostgreSQL).
- `ConfigRepository`: reads and writes the config.
- `HistoryRepository`: reads and writes the restore history.
- `PasswordStore`: saves, reads and deletes the connection password. The adapter uses the system keychain (`keyring` crate).
- `TranslationProvider`: returns text by key and language.
- `Logger`: writes log entries. The core uses only this port, not a specific log library.

To support a new database, add a new adapter that implements `DatabaseRestorer`. No other layer changes.

## Configuration

File `config.json`, saved in the local application data folder of each system:

| System | Path |
|--------|------|
| Windows | `%LOCALAPPDATA%\\SQLRestore\\config.json` |
| macOS | `~/Library/Application Support/SQLRestore/config.json` |
| Linux | `$XDG_DATA_HOME/SQLRestore/config.json` (default `~/.local/share/SQLRestore/config.json`) |

```json
{
  "theme": "light",
  "language": "en",
  "connection": {
    "host": "localhost",
    "port": 5432,
    "user": "postgres",
    "database": ""
  },
  "recreate_database": false,
  "last_backup_file": null,
  "recent_backup_files": []
}
```

`last_backup_file` is the `.sql` file selected by the user in the previous session. `recent_backup_files` holds the last five used `.sql` files, newest first.

## History

Each successful restore adds one entry to `history.json` in the local application data folder. The entry has the file name, target database, connection, local date and time, and duration. The file is a JSON array, oldest first. When it has more than 100 entries, the oldest entries are removed. The History button opens a dialog with every entry, newest first.

| System | Path |
|--------|------|
| Windows | `%LOCALAPPDATA%\\SQLRestore\\history.json` |
| macOS | `~/Library/Application Support/SQLRestore/history.json` |
| Linux | `$XDG_DATA_HOME/SQLRestore/history.json` (default `~/.local/share/SQLRestore/history.json`) |

## Password

The password is not saved in `config.json`. It is saved in the system keychain:

| System | Keychain |
|--------|----------|
| Windows | Credential Manager |
| macOS | Keychain |
| Linux | Secret Service (GNOME Keyring or KWallet) |

- Service name: `SQLRestore`. The entry name is built from host, port and user.
- If the keychain is not available, the password is not saved and the user types it each time.
- The password is never written in the log.

### Default Config

If `config.json` is missing, a new file is created with default values. If a value is invalid, only that value is restored to its default. The other values are kept.

Default values:

```json
{
  "theme": "light",
  "language": "en",
  "connection": {
    "host": "localhost",
    "port": 5432,
    "user": "postgres",
    "database": ""
  },
  "recreate_database": false,
  "last_backup_file": null
}
```

`recreate_database` is disabled by default. When enabled, the application connects to the `postgres` maintenance database, terminates open sessions, runs `DROP DATABASE IF EXISTS` and `CREATE DATABASE` on the target database, then restores. The user must confirm the deletion before every restore. The maintenance databases `postgres`, `template0` and `template1` are never deleted.

## Log

The program writes a log file to find problems after a restore fails.

| System | Path |
|--------|------|
| Windows | `%LOCALAPPDATA%\\SQLRestore\\logs\\sqlrestore.log` |
| macOS | `~/Library/Application Support/SQLRestore/logs/sqlrestore.log` |
| Linux | `$XDG_DATA_HOME/SQLRestore/logs/sqlrestore.log` (default `~/.local/share/SQLRestore/logs/sqlrestore.log`) |

- Each entry has date, time, level and message.
- In debug builds each entry is also written to the terminal (stderr).
- Record the start, end, cancel and errors of each restore. For errors, include the line where it happened.
- Error entries include the PostgreSQL message, detail, hint, context, SQLSTATE and the exact source line of the failing statement.
- Never write passwords or other secrets in the log.
- Maximum size per file: 5 MB. When the limit is reached, the file is rotated. Keep the 3 most recent old files and delete the others.
- If the log file cannot be written, the program keeps working.
- Log messages are for developers and are written in English. They are not translated.

## Internationalization

- Files: `locales/<language-code>.json` (example: `en.json`).
- Available languages: `en`, `pt-BR`.
- Code uses keys only (example: `restore.button.start`).
- English is the fallback language.
