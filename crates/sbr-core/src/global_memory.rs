//! Port of the conversion-relevant settings in `GlobalMemory.cs`.

use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Clone)]
pub struct GlobalMemory {
    pub parse_bible: bool,
    pub parse_bible_bkgnd_cleaner: bool,
    pub console_mode: bool,
    pub locale: String,
    /// `GlobalMemory.Direction` (0 = left to right).
    pub direction: i32,
    /// `Environment.NewLine` used by `StreamWriter.WriteLine`.
    pub newline: String,
    pub display_bib_raw: bool,
    pub commentary_icon: bool,
    pub selected_theme: String,
    pub font_family_bib: String,
    pub font_family_cmt: String,
}

impl Default for GlobalMemory {
    fn default() -> Self {
        Self {
            parse_bible: true,
            parse_bible_bkgnd_cleaner: true,
            console_mode: false,
            locale: "en".into(),
            direction: 0,
            newline: if cfg!(windows) { "\r\n" } else { "\n" }.into(),
            display_bib_raw: false,
            commentary_icon: true,
            selected_theme: "Default".into(),
            font_family_bib: String::new(),
            font_family_cmt: String::new(),
        }
    }
}

fn cell() -> &'static RwLock<GlobalMemory> {
    static G: OnceLock<RwLock<GlobalMemory>> = OnceLock::new();
    G.get_or_init(|| RwLock::new(GlobalMemory::default()))
}

/// `GlobalMemory.getInstance()` for reading.
pub fn get() -> RwLockReadGuard<'static, GlobalMemory> {
    cell().read().unwrap()
}

pub fn get_mut() -> RwLockWriteGuard<'static, GlobalMemory> {
    cell().write().unwrap()
}
