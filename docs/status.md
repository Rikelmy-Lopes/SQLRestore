# Status

Features already done.

## Done

- [x] Project documentation (README, AGENTS, architecture, development, status, UI design)
- [x] Workspace with `core` and `app` crates
- [x] Restore `.sql` file on PostgreSQL using the database connection
- [x] Cancel restore
- [x] Save password in the system keychain (`PasswordStore` port)
- [x] Optional delete and recreate of the target database before restore (with confirmation)
- [x] Default config values when the file is missing or a value is invalid
- [x] Database connection form
- [x] Test the database connection before restoring (success and error shown)
- [x] Select backup file and show its size in MB or GB
- [x] Restore progress and result
- [x] Separate loading spinner and progress bar
- [x] Remember the last `.sql` file in the configuration
- [x] Recent `.sql` files dropdown: last 5 used paths, without duplicates
- [x] After-restore SQL script pasted in a dialog and saved in `config.json`, with a checkbox that enables running it only after a successful restore
- [x] Optional use of the `.sql` file name as the database name
- [x] Show the restore duration in the result
- [x] Lock connection fields and database recreation checkbox while restoring
- [x] Stream large `.sql` files and `COPY` data without loading the whole file
- [x] Light and dark themes
- [x] i18n with `.json` files (English and Brazilian Portuguese)
- [x] Config file (`config.json` in the local application data folder): theme, language and connection
- [x] Log file in the local application data folder with size limit and rotation (`Logger` port)
- [x] Restore history in `history.json` (last 100 successful restores) with a dialog to view it
- [x] Clear all restore history from the history dialog (with confirmation)
- [x] History date and time formatted for the selected language (`MM/DD/YYYY` and 12-hour clock in English, `dd/MM/yyyy` and 24-hour clock in Brazilian Portuguese)
- [x] Detailed PostgreSQL errors in the log and the exact failing source line
- [x] Log entries on the terminal in debug builds
- [x] Cross-platform support
- [x] Styled interface: rounded inputs and buttons, centered card layout, colored success and error banners
- [x] Automated tests for the core and the application
- [x] Windows installer (NSIS) and Linux packages (`.deb`, `.AppImage`) with cargo-packager
- [x] Release workflow that builds both platforms and publishes tagged releases

## Pending

- [ ] Manual end-to-end restore test on a live PostgreSQL server
- [x] Close confirmation while a restore is running
