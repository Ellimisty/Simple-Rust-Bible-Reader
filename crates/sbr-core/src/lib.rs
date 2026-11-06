//! Port of the non-UI core of Simple Bible Reader 5.6.2.

pub mod bible_format;
pub mod book;
pub mod chapter_heading;
pub mod commentary;
pub mod dictionary;
pub mod dotnet;
pub mod format_util;
pub mod formats;
pub mod global_memory;
pub mod io;
pub mod localization;
pub mod post_process;
pub mod versification;

pub const AUTHOR_WEBSITE: &str = "trumpet-call.org/simplebiblereader/";

/// `GlobalMemory.Locale`.
pub fn global_locale() -> String {
    global_memory::get().locale.clone()
}

pub fn set_global_locale(l: &str) {
    global_memory::get_mut().locale = l.to_string();
}
