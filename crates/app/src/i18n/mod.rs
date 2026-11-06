//! Translation access for the interface.

use std::sync::Arc;

use sqlrestore_core::ports::translation_provider::TranslationProvider;

/// Holds the active language and its dictionaries.
pub struct I18n {
    provider: Arc<dyn TranslationProvider>,
    language: String,
}

impl I18n {
    pub fn new(provider: Arc<dyn TranslationProvider>, language: String) -> Self {
        Self { provider, language }
    }

    pub fn set_language(&mut self, language: String) {
        self.language = language;
    }

    /// Translates a key in the active language.
    pub fn t(&self, key: &str) -> String {
        self.provider.translate(key, &self.language)
    }

    /// Translates a key and replaces `{name}` placeholders.
    pub fn t_args(&self, key: &str, arguments: &[(&str, String)]) -> String {
        let mut text = self.t(key);
        for (name, value) in arguments {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }
}

/// English dictionary embedded at compile time.
pub(crate) const ENGLISH_JSON: &str = include_str!("../../../../locales/en.json");

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrestore_core::infrastructure::translations::JsonTranslationProvider;

    fn i18n() -> I18n {
        let provider = JsonTranslationProvider::from_json_sources([(
            "en".to_string(),
            ENGLISH_JSON.to_string(),
        )])
        .expect("translations");
        I18n::new(Arc::new(provider), "en".to_string())
    }

    #[test]
    fn translates_known_keys() {
        assert_eq!(i18n().t("app.title"), "SQLRestore");
        assert_eq!(i18n().t("action.restore"), "Restore");
    }

    #[test]
    fn replaces_placeholders() {
        let text = i18n().t_args(
            "status.success",
            &[
                ("statements", "2".to_string()),
                ("bytes", "10".to_string()),
                ("duration", "1.2 s".to_string()),
            ],
        );
        assert_eq!(text, "Restore completed in 1.2 s: 2 statements, 10 bytes.");
    }

    #[test]
    fn english_file_has_every_required_key() {
        let required = [
            "app.title",
            "topbar.theme",
            "theme.light",
            "theme.dark",
            "language.label",
            "language.en",
            "connection.title",
            "connection.host",
            "connection.host.hint",
            "connection.port",
            "connection.port.hint",
            "connection.user",
            "connection.user.hint",
            "connection.password",
            "connection.password.hint",
            "connection.database",
            "connection.database.hint",
            "connection.use_file_name_as_database",
            "connection.use_file_name_as_database.hint",
            "connection.recreate_database",
            "connection.recreate_database.hint",
            "backup.title",
            "backup.select",
            "backup.no_file",
            "history.open",
            "history.title",
            "history.empty",
            "history.close",
            "history.date",
            "history.time",
            "history.file",
            "history.database",
            "history.connection",
            "history.duration",
            "action.restore",
            "action.cancel",
            "recreate.title",
            "recreate.warning",
            "recreate.cancel",
            "recreate.confirm",
            "result.title",
            "status.idle",
            "status.running",
            "status.success",
            "status.cancelled",
            "status.failed",
            "status.invalid_port",
            "status.config_save_failed",
            "status.password_save_failed",
            "status.connection_lost",
            "duration.seconds",
            "duration.minutes",
            "duration.hours",
        ];
        let i18n = i18n();
        for key in required {
            assert_ne!(i18n.t(key), key, "missing translation key: {key}");
        }
    }
}
