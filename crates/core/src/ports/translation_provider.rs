//! Translation port.

/// Returns user-visible text by key and language.
pub trait TranslationProvider: Send + Sync {
    /// Looks up a key. Falls back to English, then to the key itself.
    fn translate(&self, key: &str, language: &str) -> String;
}
