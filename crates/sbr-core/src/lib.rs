//! Port of the non-UI core of Simple Bible Reader 5.6.2.

pub mod format_util;
pub mod localization;

static LOCALE: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// `GlobalMemory.Locale` (defaults to `"en"`).
pub fn global_locale() -> String {
    let l = LOCALE.read().unwrap();
    if l.is_empty() {
        "en".into()
    } else {
        l.clone()
    }
}

pub fn set_global_locale(l: &str) {
    *LOCALE.write().unwrap() = l.to_string();
}
