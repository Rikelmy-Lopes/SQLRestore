//! JSON dictionary translation provider.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::domain::config::FALLBACK_LANGUAGE;
use crate::domain::error::TranslationError;
use crate::ports::translation_provider::TranslationProvider;

/// Loads translations from flat JSON dictionaries.
pub struct JsonTranslationProvider {
    dictionaries: HashMap<String, HashMap<String, String>>,
    fallback_language: String,
}

impl JsonTranslationProvider {
    /// Builds a provider from in-memory JSON sources. Used for embedded files.
    pub fn from_json_sources<I>(sources: I) -> Result<Self, TranslationError>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut dictionaries = HashMap::new();
        for (language, json) in sources {
            let entries: HashMap<String, String> =
                serde_json::from_str(&json).map_err(|error| TranslationError::Parse {
                    file: language.clone(),
                    message: error.to_string(),
                })?;
            dictionaries.insert(language, entries);
        }
        Ok(Self {
            dictionaries,
            fallback_language: FALLBACK_LANGUAGE.to_string(),
        })
    }

    /// Loads every `<language>.json` file in a directory.
    pub fn from_directory(path: &Path) -> Result<Self, TranslationError> {
        let mut sources = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("json")
            {
                continue;
            }
            let Some(language) = file_path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            sources.push((language.to_string(), fs::read_to_string(&file_path)?));
        }
        Self::from_json_sources(sources)
    }

    /// Language used when the requested one lacks a key.
    pub fn fallback_language(&self) -> &str {
        &self.fallback_language
    }
}

impl TranslationProvider for JsonTranslationProvider {
    fn translate(&self, key: &str, language: &str) -> String {
        self.dictionaries
            .get(language)
            .and_then(|dictionary| dictionary.get(key))
            .or_else(|| {
                self.dictionaries
                    .get(&self.fallback_language)
                    .and_then(|dictionary| dictionary.get(key))
            })
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> JsonTranslationProvider {
        JsonTranslationProvider::from_json_sources([
            (
                "en".to_string(),
                r#"{"app.title": "SQLRestore", "action.restore": "Restore"}"#.to_string(),
            ),
            (
                "xx".to_string(),
                r#"{"app.title": "SQLRestore XX"}"#.to_string(),
            ),
        ])
        .expect("provider")
    }

    #[test]
    fn returns_requested_language() {
        assert_eq!(provider().translate("app.title", "xx"), "SQLRestore XX");
    }

    #[test]
    fn falls_back_to_english() {
        assert_eq!(provider().translate("action.restore", "xx"), "Restore");
    }

    #[test]
    fn unknown_language_and_key_return_the_key() {
        assert_eq!(provider().translate("missing.key", "zz"), "missing.key");
    }

    #[test]
    fn loads_from_directory() {
        let directory = tempfile::tempdir().expect("temporary directory");
        fs::write(
            directory.path().join("en.json"),
            r#"{"app.title": "SQLRestore"}"#,
        )
        .expect("write");
        let provider = JsonTranslationProvider::from_directory(directory.path()).expect("load");
        assert_eq!(provider.translate("app.title", "en"), "SQLRestore");
    }
}
