# Architecture

Mandatory rules are in [AGENTS.md](../AGENTS.md).

## Patterns

- **Clean Architecture:** dependencies point inward. Inner layers do not know outer layers.
- **Commands Pattern:** each user action is a command with one handler.
- **Single Responsibility Principle:** one purpose per file, folder, struct and function.

## Crates

| Crate | Purpose |
|-------|---------|
| `crates/core` | Business logic, commands, ports, adapters, config, i18n. No UI. |
| `crates/app` | egui (eframe) interface. Calls the core only through commands. |

## Restore

The backup is restored using the database connection. The adapter connects to the database and runs the `.sql` file through this connection. The restore does not use a transaction. If an error occurs or the user cancels, the restore stops.

## Ports

- `DatabaseRestorer`: restores a file using a database connection and supports cancel. Each supported database has its own adapter (first: PostgreSQL).
- `ConfigRepository`: reads and writes the config.
- `PasswordStore`: saves, reads and deletes the connection password. The adapter uses the system keychain (`keyring` crate).
- `TranslationProvider`: returns text by key and language.
- `Logger`: writes log entries. The core uses only this port, not a specific log library.

To support a new database, add a new adapter that implements `DatabaseRestorer`. No other layer changes.

## Configuration

File `config.json`, saved in the AppData folder of each system:

| System | Path |
|--------|------|
| Windows | `%APPDATA%\SQLRestore\config.json` |
| macOS | `~/Library/Application Support/SQLRestore/config.json` |
| Linux | `~/.config/SQLRestore/config.json` |

```json
{
  "theme": "light",
  "language": "en",
  "connection": {
    "host": "localhost",
    "port": 5432,
    "user": "postgres",
    "database": ""
  }
}
```

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
  }
}
```

## Log

The program writes a log file to find problems after a restore fails.

| System | Path |
|--------|------|
| Windows | `%APPDATA%\SQLRestore\logs\sqlrestore.log` |
| macOS | `~/Library/Application Support/SQLRestore/logs/sqlrestore.log` |
| Linux | `~/.config/SQLRestore/logs/sqlrestore.log` |

- Each entry has date, time, level and message.
- Record the start, end, cancel and errors of each restore. For errors, include the line where it happened.
- Never write passwords or other secrets in the log.
- Maximum size per file: 5 MB. When the limit is reached, the file is rotated. Keep the 3 most recent old files and delete the others.
- If the log file cannot be written, the program keeps working.
- Log messages are for developers and are written in English. They are not translated.

## Internationalization

- Files: `locales/<language-code>.json` (example: `en.json`).
- Code uses keys only (example: `restore.button.start`).
- English is the fallback language.
