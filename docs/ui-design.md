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
| Background | `#F4F6FA` | `#121212` |
| Panel | `#FFFFFF` | `#1E1E1E` |
| Card | `#FFFFFF` | `#1E1E1E` |
| Input | `#FAFBFD` | `#262626` |
| Border | `#D6DCE7` | `#333A45` |
| Text | `#1A1A1A` | `#EDEDED` |
| Weak text | `#6B7280` | `#9CA3AF` |
| Primary | `#2563EB` | `#3B82F6` |
| Success | `#16A34A` | `#22C55E` |
| Error | `#DC2626` | `#EF4444` |

The theme is saved in `config.json`.

## Main Screen

1. **Top bar:** app title, history button, theme switch, language selector.
2. **Connection:** host, port, user, password, database. A test button checks the connection before restoring.
3. **Backup file:** file path, its size in MB or GB, and a button to select the `.sql` file. Clicking the path shows the last five used files.
4. **Action:** restore button and cancel button.
5. **Result area:** loading spinner, progress bar and success or error message. The success message includes the restore duration.
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
|  [ Test connection ]                       |
|  Connection successful. / Connection failed: ... |
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
- The test connection button is enabled when the connection fields are filled. The result is shown right below it: success or the database error. Editing a connection field clears the result.
- The last selected `.sql` file is restored from the configuration at startup.
- Clicking the selected file path opens a dropdown with the last five used `.sql` files, full path included. Choosing one selects it without opening the file dialog. Duplicates are not stored.
- Hovering the file path shows a primary-color border, a light tint and a pointing-hand cursor.
- The loading spinner is separate from the progress bar.
- When "Delete and recreate the database" is enabled, a confirmation dialog with the database name is shown before each restore.
- The cancel button is always present. It is enabled only while a restore is running.
- While a restore is running, the connection fields, the database recreation checkbox and the file selector are disabled. Only Cancel, the theme switch and the language selector stay enabled.
- Cancel stops the restore and shows a translated message.
- The History button opens a dialog with all successful restores. The newest entry is first.
- The UI does not freeze during restore (work runs outside the UI thread).
- Errors show a clear translated message.
