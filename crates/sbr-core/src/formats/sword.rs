//! Port of `SwordBibleFormat.cs`. Not ported yet: both directions return
//! `Error::Unsupported`.

use crate::bible_format::{Bible, Error, ExportMeta, Result, F12_SWORDPROJECT};

pub fn load(_b: &mut Bible, _filename: &str) -> Result<()> {
    Err(Error::Unsupported(F12_SWORDPROJECT))
}

pub fn export(_b: &mut Bible, _meta: &ExportMeta, _filename: &str, _filter_idx: i32) -> Result<()> {
    Err(Error::Unsupported(F12_SWORDPROJECT))
}
