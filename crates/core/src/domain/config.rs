//! Application configuration model.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Languages shipped with the application.
pub const SUPPORTED_LANGUAGES: &[&str] = &["en"];

/// Language used when the configured language is not available.
pub const FALLBACK_LANGUAGE: &str = "en";

const DEFAULT_HOST: &str = "localhost";
const DEFAULT_PORT: u16 = 5432;
const DEFAULT_USER: &str = "postgres";
const DEFAULT_USE_FILE_NAME_AS_DATABASE: bool = true;

/// Database name taken from a backup file name, without the `.sql` extension.
///
/// Returns `None` when the file name is empty or contains only the extension.
pub fn database_name_from_backup(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    let name = stem.trim();
    if name.is_empty() || name.eq_ignore_ascii_case(".sql") {
        None
    } else {
        Some(name.to_string())
    }
}

/// Interface theme.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl Theme {
    /// Text stored in the configuration file.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parses a theme value. Unknown values return `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// Returns the opposite theme.
    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

/// Database connection settings. The password is stored separately.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionSettings {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
            user: DEFAULT_USER.to_string(),
            database: String::new(),
        }
    }
}

impl ConnectionSettings {
    /// True when every value required to connect is filled.
    pub fn is_complete(&self) -> bool {
        !self.host.trim().is_empty()
            && self.port > 0
            && !self.user.trim().is_empty()
            && !self.database.trim().is_empty()
    }
}

/// Configuration saved in `config.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme: Theme,
    pub language: String,
    pub connection: ConnectionSettings,
    /// Deletes and recreates the target database before a restore.
    #[serde(default)]
    pub recreate_database: bool,
    /// Uses the backup file name, without `.sql`, as the target database.
    #[serde(default = "default_use_file_name_as_database")]
    pub use_file_name_as_database: bool,
    /// Backup file used by the last restore.
    #[serde(default)]
    pub last_backup_file: Option<PathBuf>,
}

fn default_use_file_name_as_database() -> bool {
    DEFAULT_USE_FILE_NAME_AS_DATABASE
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::default(),
            language: FALLBACK_LANGUAGE.to_string(),
            connection: ConnectionSettings::default(),
            recreate_database: false,
            use_file_name_as_database: DEFAULT_USE_FILE_NAME_AS_DATABASE,
            last_backup_file: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documentation() {
        let config = AppConfig::default();
        assert_eq!(config.theme, Theme::Light);
        assert_eq!(config.language, "en");
        assert_eq!(config.connection.host, "localhost");
        assert_eq!(config.connection.port, 5432);
        assert_eq!(config.connection.user, "postgres");
        assert!(config.connection.database.is_empty());
        assert!(!config.recreate_database);
        assert!(config.use_file_name_as_database);
        assert!(config.last_backup_file.is_none());
    }

    #[test]
    fn database_name_comes_from_the_file_stem() {
        assert_eq!(
            database_name_from_backup(Path::new("C:/backups/shop.sql")),
            Some("shop".to_string())
        );
        assert_eq!(
            database_name_from_backup(Path::new("C:/backups/shop.backup.sql")),
            Some("shop.backup".to_string())
        );
        assert_eq!(
            database_name_from_backup(Path::new("C:/backups/SHOP.SQL")),
            Some("SHOP".to_string())
        );
    }

    #[test]
    fn database_name_is_none_without_a_file_name() {
        assert_eq!(database_name_from_backup(Path::new("")), None);
        assert_eq!(
            database_name_from_backup(Path::new("C:/backups/.sql")),
            None
        );
    }

    #[test]
    fn theme_round_trips() {
        assert_eq!(Theme::parse("light"), Some(Theme::Light));
        assert_eq!(Theme::parse("dark"), Some(Theme::Dark));
        assert_eq!(Theme::parse("blue"), None);
        assert_eq!(Theme::Light.toggled(), Theme::Dark);
    }

    #[test]
    fn connection_is_complete_only_with_database() {
        let mut connection = ConnectionSettings::default();
        assert!(!connection.is_complete());
        connection.database = "app".to_string();
        assert!(connection.is_complete());

        connection.host.clear();
        assert!(!connection.is_complete());
    }
}
