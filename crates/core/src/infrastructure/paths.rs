//! Platform-specific application paths.

use std::path::PathBuf;

/// Folder name used inside the platform configuration directory.
pub const APP_NAME: &str = "SQLRestore";

/// Configuration directory for the current user.
pub fn app_data_dir() -> PathBuf {
    base_dir().join(APP_NAME)
}

/// Path of `config.json`.
pub fn config_path() -> PathBuf {
    app_data_dir().join("config.json")
}

/// Path of the active log file.
pub fn log_path() -> PathBuf {
    app_data_dir().join("logs").join("sqlrestore.log")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(target_os = "windows")]
fn base_dir() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join("AppData").join("Roaming")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(target_os = "macos")]
fn base_dir() -> PathBuf {
    home_dir()
        .map(|home| home.join("Library").join("Application Support"))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn base_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_end_with_application_name() {
        assert!(app_data_dir().ends_with(APP_NAME));
        assert!(config_path().ends_with("config.json"));
        assert!(log_path().ends_with("sqlrestore.log"));
    }
}
