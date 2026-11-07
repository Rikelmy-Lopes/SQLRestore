//! File-backed configuration repository.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::domain::config::{
    AppConfig, ConnectionSettings, FALLBACK_LANGUAGE, RECENT_BACKUP_FILES_LIMIT,
    SUPPORTED_LANGUAGES, Theme, same_path,
};
use crate::domain::error::ConfigError;
use crate::infrastructure::paths;
use crate::ports::config_repository::ConfigRepository;

/// Stores the configuration in a JSON file.
pub struct FileConfigRepository {
    path: PathBuf,
}

impl FileConfigRepository {
    /// Uses the given file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Uses the platform AppData location.
    pub fn default_location() -> Self {
        Self::new(paths::config_path())
    }

    /// File used by this repository.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ConfigRepository for FileConfigRepository {
    fn load(&self) -> Result<AppConfig, ConfigError> {
        if !self.path.is_file() {
            let config = AppConfig::default();
            self.save(&config)?;
            return Ok(config);
        }

        let raw = fs::read_to_string(&self.path)?;
        let value: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let config = config_from_value(&value);

        if serde_json::to_value(&config).ok().as_ref() != Some(&value) {
            // Repair invalid or missing values. A write failure must not stop
            // the application from using the repaired in-memory configuration.
            let _ = self.save(&config);
        }

        Ok(config)
    }

    fn save(&self, config: &AppConfig) -> Result<(), ConfigError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut json = serde_json::to_string_pretty(config)
            .map_err(|error| ConfigError::Parse(error.to_string()))?;
        json.push('\n');
        fs::write(&self.path, json)?;
        Ok(())
    }
}

fn config_from_value(value: &Value) -> AppConfig {
    let defaults = AppConfig::default();

    let theme = value
        .get("theme")
        .and_then(Value::as_str)
        .and_then(Theme::parse)
        .unwrap_or(defaults.theme);

    let language = value
        .get("language")
        .and_then(Value::as_str)
        .filter(|language| SUPPORTED_LANGUAGES.contains(language))
        .unwrap_or(FALLBACK_LANGUAGE)
        .to_string();

    let connection = value
        .get("connection")
        .map(|connection| connection_from_value(connection, &defaults.connection))
        .unwrap_or(defaults.connection);

    let recreate_database = value
        .get("recreate_database")
        .and_then(Value::as_bool)
        .unwrap_or(defaults.recreate_database);

    let use_file_name_as_database = value
        .get("use_file_name_as_database")
        .and_then(Value::as_bool)
        .unwrap_or(defaults.use_file_name_as_database);

    let last_backup_file = value
        .get("last_backup_file")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .or(defaults.last_backup_file);

    let recent_backup_files = recent_files_from_value(value, last_backup_file.as_ref());

    AppConfig {
        theme,
        language,
        connection,
        recreate_database,
        use_file_name_as_database,
        last_backup_file,
        recent_backup_files,
    }
}

/// Reads the recent backup list, removes duplicates and keeps the newest five.
fn recent_files_from_value(value: &Value, last_backup_file: Option<&PathBuf>) -> Vec<PathBuf> {
    let mut recent: Vec<PathBuf> = Vec::new();
    if let Some(entries) = value.get("recent_backup_files").and_then(Value::as_array) {
        for entry in entries {
            let Some(path) = entry
                .as_str()
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
            else {
                continue;
            };
            if !recent.iter().any(|known| same_path(known, &path)) {
                recent.push(path);
            }
            if recent.len() == RECENT_BACKUP_FILES_LIMIT {
                break;
            }
        }
    }

    if recent.is_empty()
        && let Some(path) = last_backup_file
    {
        recent.push(path.clone());
    }

    recent
}

fn connection_from_value(value: &Value, defaults: &ConnectionSettings) -> ConnectionSettings {
    let host = value
        .get("host")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .unwrap_or(&defaults.host)
        .to_string();

    let port = value
        .get("port")
        .and_then(Value::as_u64)
        .filter(|port| (1..=u64::from(u16::MAX)).contains(port))
        .map(|port| port as u16)
        .unwrap_or(defaults.port);

    let user = value
        .get("user")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|user| !user.is_empty())
        .unwrap_or(&defaults.user)
        .to_string();

    let database = value
        .get("database")
        .and_then(Value::as_str)
        .unwrap_or(&defaults.database)
        .to_string();

    ConnectionSettings {
        host,
        port,
        user,
        database,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository() -> (tempfile::TempDir, FileConfigRepository) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("config.json");
        (directory, FileConfigRepository::new(path))
    }

    #[test]
    fn missing_file_creates_defaults() {
        let (directory, repository) = repository();
        let config = repository.load().expect("load");
        assert_eq!(config, AppConfig::default());
        assert!(directory.path().join("config.json").is_file());
    }

    #[test]
    fn round_trips_configuration() {
        let (_directory, repository) = repository();
        let config = AppConfig {
            theme: Theme::Dark,
            language: "en".to_string(),
            connection: ConnectionSettings {
                host: "db.local".to_string(),
                port: 6543,
                user: "admin".to_string(),
                database: "shop".to_string(),
            },
            recreate_database: true,
            use_file_name_as_database: false,
            last_backup_file: Some(PathBuf::from("C:/backups/shop.sql")),
            recent_backup_files: vec![
                PathBuf::from("C:/backups/shop.sql"),
                PathBuf::from("C:/backups/other.sql"),
            ],
        };
        repository.save(&config).expect("save");
        assert_eq!(repository.load().expect("load"), config);
    }

    #[test]
    fn empty_last_backup_file_falls_back_to_none() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{"last_backup_file": "   "}"#,
        )
        .expect("write");
        assert!(repository.load().expect("load").last_backup_file.is_none());
    }

    #[test]
    fn recent_files_are_deduplicated_and_limited() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{
                "recent_backup_files": [
                    "a.sql", "b.sql", "a.sql", "c.sql", "d.sql", "e.sql", "f.sql"
                ]
            }"#,
        )
        .expect("write");

        let recent = repository.load().expect("load").recent_backup_files;
        let expected = ["a.sql", "b.sql", "c.sql", "d.sql", "e.sql"]
            .map(PathBuf::from)
            .to_vec();
        assert_eq!(recent, expected);
    }

    #[test]
    fn recent_files_start_from_the_last_backup_file() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{"last_backup_file": "C:/backups/shop.sql"}"#,
        )
        .expect("write");

        let config = repository.load().expect("load");
        assert_eq!(
            config.recent_backup_files,
            vec![PathBuf::from("C:/backups/shop.sql")]
        );
    }

    #[test]
    fn invalid_values_fall_back_individually() {
        let (directory, repository) = repository();
        let path = directory.path().join("config.json");
        fs::write(
            &path,
            r#"{
                "theme": "purple",
                "language": "xx",
                "connection": {
                    "host": "db.local",
                    "port": 0,
                    "user": "",
                    "database": "shop"
                }
            }"#,
        )
        .expect("write");

        let config = repository.load().expect("load");
        assert_eq!(config.theme, Theme::Light);
        assert_eq!(config.language, "en");
        assert_eq!(config.connection.host, "db.local");
        assert_eq!(config.connection.port, 5432);
        assert_eq!(config.connection.user, "postgres");
        assert_eq!(config.connection.database, "shop");
        assert!(!config.recreate_database);
        assert!(config.use_file_name_as_database);
        assert!(config.last_backup_file.is_none());
    }

    #[test]
    fn invalid_use_file_name_flag_falls_back_to_true() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{"use_file_name_as_database": "yes"}"#,
        )
        .expect("write");
        assert!(repository.load().expect("load").use_file_name_as_database);
    }

    #[test]
    fn invalid_recreate_flag_falls_back_to_false() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{"recreate_database": "yes"}"#,
        )
        .expect("write");
        assert!(!repository.load().expect("load").recreate_database);
    }

    #[test]
    fn invalid_file_is_replaced_with_defaults() {
        let (directory, repository) = repository();
        fs::write(directory.path().join("config.json"), "not json").expect("write");
        assert_eq!(repository.load().expect("load"), AppConfig::default());
    }

    #[test]
    fn port_above_range_falls_back() {
        let (directory, repository) = repository();
        fs::write(
            directory.path().join("config.json"),
            r#"{"connection": {"port": 70000}}"#,
        )
        .expect("write");
        assert_eq!(repository.load().expect("load").connection.port, 5432);
    }
}
