//! Language-aware date and time formatting.

use chrono::{NaiveDate, NaiveTime};

use crate::i18n::I18n;

/// Stored date format in the history file.
const STORED_DATE_FORMAT: &str = "%Y-%m-%d";
/// Stored time format in the history file.
const STORED_TIME_FORMAT: &str = "%H:%M:%S";

/// Formats a stored history date for the active language.
///
/// The history file always stores `YYYY-MM-DD`. English displays
/// `MM/DD/YYYY`; Brazilian Portuguese displays `dd/MM/yyyy`.
pub fn format_date(i18n: &I18n, date: &str) -> String {
    let Ok(date) = NaiveDate::parse_from_str(date, STORED_DATE_FORMAT) else {
        return date.to_string();
    };
    date.format(date_pattern(i18n.language())).to_string()
}

/// Formats a stored history time for the active language.
///
/// The history file always stores `HH:MM:SS`. English uses a 12-hour clock
/// with AM/PM; Brazilian Portuguese uses a 24-hour clock.
pub fn format_time(i18n: &I18n, time: &str) -> String {
    let Ok(time) = NaiveTime::parse_from_str(time, STORED_TIME_FORMAT) else {
        return time.to_string();
    };
    time.format(time_pattern(i18n.language())).to_string()
}

fn date_pattern(language: &str) -> &'static str {
    match language {
        "pt-BR" => "%d/%m/%Y",
        _ => "%m/%d/%Y",
    }
}

fn time_pattern(language: &str) -> &'static str {
    match language {
        "pt-BR" => "%H:%M:%S",
        _ => "%I:%M:%S %p",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrestore_core::infrastructure::translations::JsonTranslationProvider;
    use std::sync::Arc;

    use crate::i18n::{ENGLISH_JSON, PORTUGUESE_BR_JSON};

    fn i18n(language: &str) -> I18n {
        let provider = JsonTranslationProvider::from_json_sources([
            ("en".to_string(), ENGLISH_JSON.to_string()),
            ("pt-BR".to_string(), PORTUGUESE_BR_JSON.to_string()),
        ])
        .expect("translations");
        I18n::new(Arc::new(provider), language.to_string())
    }

    #[test]
    fn formats_english_date() {
        assert_eq!(format_date(&i18n("en"), "2026-11-06"), "11/06/2026");
    }

    #[test]
    fn formats_portuguese_date() {
        assert_eq!(format_date(&i18n("pt-BR"), "2026-11-06"), "06/11/2026");
    }

    #[test]
    fn formats_english_time_with_am_pm() {
        assert_eq!(format_time(&i18n("en"), "14:35:09"), "02:35:09 PM");
    }

    #[test]
    fn formats_portuguese_time_with_24_hour_clock() {
        assert_eq!(format_time(&i18n("pt-BR"), "14:35:09"), "14:35:09");
    }

    #[test]
    fn keeps_unparseable_values() {
        assert_eq!(format_date(&i18n("en"), "not-a-date"), "not-a-date");
        assert_eq!(format_time(&i18n("en"), "not-a-time"), "not-a-time");
    }
}
