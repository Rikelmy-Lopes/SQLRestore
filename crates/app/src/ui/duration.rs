//! Restore duration formatting.

use std::time::Duration;

use crate::i18n::I18n;

/// Formats a restore duration with translated units.
pub fn format_duration(i18n: &I18n, elapsed: Duration) -> String {
    let total_seconds = elapsed.as_secs();
    if total_seconds < 60 {
        i18n.t_args(
            "duration.seconds",
            &[("seconds", format!("{:.1}", elapsed.as_secs_f64()))],
        )
    } else if total_seconds < 3600 {
        i18n.t_args(
            "duration.minutes",
            &[
                ("minutes", (total_seconds / 60).to_string()),
                ("seconds", (total_seconds % 60).to_string()),
            ],
        )
    } else {
        i18n.t_args(
            "duration.hours",
            &[
                ("hours", (total_seconds / 3600).to_string()),
                ("minutes", ((total_seconds % 3600) / 60).to_string()),
                ("seconds", (total_seconds % 60).to_string()),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrestore_core::infrastructure::translations::JsonTranslationProvider;
    use std::sync::Arc;

    use crate::i18n::ENGLISH_JSON;

    fn i18n() -> I18n {
        let provider = JsonTranslationProvider::from_json_sources([(
            "en".to_string(),
            ENGLISH_JSON.to_string(),
        )])
        .expect("translations");
        I18n::new(Arc::new(provider), "en".to_string())
    }

    #[test]
    fn formats_seconds() {
        assert_eq!(
            format_duration(&i18n(), Duration::from_millis(1250)),
            "1.2 s"
        );
    }

    #[test]
    fn formats_minutes() {
        assert_eq!(
            format_duration(&i18n(), Duration::from_secs(125)),
            "2 min 5 s"
        );
    }

    #[test]
    fn formats_hours() {
        assert_eq!(
            format_duration(&i18n(), Duration::from_secs(3725)),
            "1 h 2 min 5 s"
        );
    }
}
