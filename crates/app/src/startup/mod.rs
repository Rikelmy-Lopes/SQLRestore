//! Application startup and dependency wiring.

use std::sync::Arc;

use eframe::egui;
use sqlrestore_core::domain::config::AppConfig;
use sqlrestore_core::infrastructure::config::FileConfigRepository;
use sqlrestore_core::infrastructure::database::PostgresRestorer;
use sqlrestore_core::infrastructure::history::FileHistoryRepository;
use sqlrestore_core::infrastructure::logger::FileLogger;
use sqlrestore_core::infrastructure::password::KeyringPasswordStore;
use sqlrestore_core::infrastructure::translations::JsonTranslationProvider;
use sqlrestore_core::ports::logger::{LogLevel, Logger};
use sqlrestore_core::ports::translation_provider::TranslationProvider;

use crate::commands::AppCommands;
use crate::i18n::{ENGLISH_JSON, I18n};
use crate::ui::SqlRestoreApp;

/// Starts the desktop application.
pub fn run() -> eframe::Result<()> {
    #[cfg(debug_assertions)]
    let logger: Arc<dyn Logger> = {
        use sqlrestore_core::infrastructure::logger::{StderrLogger, TeeLogger};

        Arc::new(TeeLogger::new(vec![
            Arc::new(FileLogger::default_location()),
            Arc::new(StderrLogger),
        ]))
    };
    #[cfg(not(debug_assertions))]
    let logger: Arc<dyn Logger> = Arc::new(FileLogger::default_location());

    let config_repository = Arc::new(FileConfigRepository::default_location());
    let password_store = Arc::new(KeyringPasswordStore);
    let restorer = Arc::new(PostgresRestorer::new());
    let history_repository = Arc::new(FileHistoryRepository::default_location());
    let commands = AppCommands::new(
        config_repository,
        password_store,
        restorer,
        history_repository,
        Arc::clone(&logger),
    );

    let config = match commands.load_config() {
        Ok(config) => config,
        Err(error) => {
            logger.log(
                LogLevel::Error,
                &format!("Configuration could not be loaded: {error}"),
            );
            AppConfig::default()
        }
    };

    let password = match commands.load_password(&config.connection) {
        Ok(password) => password.unwrap_or_default(),
        Err(error) => {
            logger.log(
                LogLevel::Warning,
                &format!("Password could not be loaded: {error}"),
            );
            String::new()
        }
    };

    let i18n = I18n::new(translation_provider(&*logger), config.language.clone());
    let window_title = i18n.t("app.title");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 700.0])
            .with_min_inner_size([600.0, 520.0])
            .with_title(window_title),
        ..Default::default()
    };

    eframe::run_native(
        "SQLRestore",
        options,
        Box::new(move |creation_context| {
            Ok(Box::new(SqlRestoreApp::new(
                creation_context,
                commands,
                i18n,
                config,
                password,
            )))
        }),
    )
}

fn translation_provider(logger: &dyn Logger) -> Arc<dyn TranslationProvider> {
    match JsonTranslationProvider::from_json_sources([("en".to_string(), ENGLISH_JSON.to_string())])
    {
        Ok(provider) => Arc::new(provider),
        Err(error) => {
            logger.log(
                LogLevel::Error,
                &format!("Translations could not be loaded: {error}"),
            );
            Arc::new(KeysTranslationProvider)
        }
    }
}

/// Emergency provider that shows translation keys.
struct KeysTranslationProvider;

impl TranslationProvider for KeysTranslationProvider {
    fn translate(&self, key: &str, _language: &str) -> String {
        key.to_string()
    }
}
