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
- [x] Select backup file
- [x] Restore progress and result
- [x] Separate loading spinner and progress bar
- [x] Remember the last `.sql` file in the configuration
- [x] Optional use of the `.sql` file name as the database name
- [x] Show the restore duration in the result
- [x] Lock connection fields and database recreation checkbox while restoring
- [x] Stream large `.sql` files and `COPY` data without loading the whole file
- [x] Light and dark themes
- [x] i18n with `.json` files (English)
- [x] Config file (`config.json` in AppData): theme, language and connection
- [x] Log file in AppData with size limit and rotation (`Logger` port)
- [x] Detailed PostgreSQL errors in the log and the exact failing source line
- [x] Log entries on the terminal in debug builds
- [x] Cross-platform support
- [x] Automated tests for the core and the application

## Pending

- [ ] Manual end-to-end restore test on a live PostgreSQL server
- [ ] Additional languages
