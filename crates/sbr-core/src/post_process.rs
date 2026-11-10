//! Port of `BibleFormat.postProcessContent`.
//!
//! The RTF↔HTML conversions it can apply arrive with `sbr-rtf`; with the
//! original's default settings (`parseBibleBkgndCleaner = true`) loaders
//! do not call it.

pub fn post_process_content(input: &str) -> String {
    input.to_string()
}
