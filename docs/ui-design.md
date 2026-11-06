# UI Design

SQLRestore uses a simple interface built with egui (eframe).

## Principles

- Simple and clean.
- Cross-platform.
- One main screen.
- All text comes from translation files.

## Themes

| Element | Light | Dark |
|---------|-------|------|
| Background | `#FFFFFF` | `#121212` |
| Panel | `#F5F5F5` | `#1E1E1E` |
| Text | `#1A1A1A` | `#EDEDED` |
| Primary | `#2563EB` | `#3B82F6` |
| Success | `#16A34A` | `#22C55E` |
| Error | `#DC2626` | `#EF4444` |

The theme is saved in `config.json`.

## Main Screen

1. **Top bar:** app title, theme switch, language selector.
2. **Connection:** host, port, user, password, database.
3. **Backup file:** file path and a button to select the `.sql` file.
4. **Action:** restore button and cancel button.
5. **Result area:** progress and success or error message.
6. **Database recreation:** a checkbox in the connection area enables deleting and recreating the target database before restoring. The confirmation dialog appears before every restore.

## Layout

```
+--------------------------------------------+
| Title                    [Theme] [Language]|
+--------------------------------------------+
| Connection                                 |
|  Host [________]  Port [____]              |
|  User [________]  Password [________]      |
|  Database [______________]                 |
|                                            |
| Backup file                                |
|  [______________________] [Select]         |
|                                            |
| [ Restore ]  [ Cancel ]                    |
|                                            |
| Result / Progress                          |
+--------------------------------------------+
```

## Behavior

- The restore button is disabled until the connection and the file are filled.
- When "Delete and recreate the database" is enabled, a confirmation dialog with the database name is shown before each restore.
- The cancel button is always present. It is enabled only while a restore is running.
- Cancel stops the restore and shows a translated message.
- The UI does not freeze during restore (work runs outside the UI thread).
- Errors show a clear translated message.
