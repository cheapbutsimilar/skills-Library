//! Messages the core sends to the UI (errors, validation issues) in English or Korean.
//!
//! The app tells the core which language the UI uses ([`set_lang`]); every message is written
//! with [`tr!`](crate::tr) as a Korean/English pair, so both languages live next to each other.

use std::sync::atomic::{AtomicBool, Ordering};

static EN: AtomicBool = AtomicBool::new(false);

pub const LANGS: &[&str] = &["system", "en", "ko"];

/// `"ko"`, `"ko-KR"` → Korean. Anything else → English.
pub fn resolve(code: &str) -> &'static str {
    if code.trim().to_ascii_lowercase().starts_with("ko") {
        "ko"
    } else {
        "en"
    }
}

pub fn set_lang(code: &str) {
    EN.store(resolve(code) == "en", Ordering::SeqCst);
}

pub fn is_en() -> bool {
    EN.load(Ordering::SeqCst)
}

pub fn lang() -> &'static str {
    if is_en() {
        "en"
    } else {
        "ko"
    }
}

/// Language to use before the UI has told us: the `language` setting, or the OS locale
/// (`os`, then `LC_ALL`, `LANG`) when it is `system`. The UI corrects it as soon as it loads.
pub fn guess(setting: &str, os: Option<&str>) -> &'static str {
    if setting == "en" || setting == "ko" {
        return resolve(setting);
    }
    if let Some(o) = os.filter(|o| !o.is_empty()) {
        return resolve(o);
    }
    for k in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(v) = std::env::var(k) {
            if !v.is_empty() {
                return resolve(&v);
            }
        }
    }
    "en"
}

/// `tr!("한국어 {x}", "English {x}")` → `String` in the current language.
/// Both arguments are `format!` strings, so inline captures work in either.
#[macro_export]
macro_rules! tr {
    ($ko:literal, $en:literal) => {
        if $crate::i18n::is_en() {
            format!($en)
        } else {
            format!($ko)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_codes() {
        assert_eq!(resolve("ko-KR"), "ko");
        assert_eq!(resolve("KO"), "ko");
        assert_eq!(resolve("en-US"), "en");
        assert_eq!(resolve("ja"), "en");
        assert_eq!(guess("ko", Some("en-US")), "ko");
        assert_eq!(guess("en", None), "en");
        assert_eq!(guess("system", Some("ko-KR")), "ko");
        assert_eq!(guess("system", Some("de-DE")), "en");
    }
}
