//! Port of `GoBibleFormat.cs`. Not ported yet: both directions return
//! `Error::Unsupported`.

use crate::bible_format::{Bible, Error, ExportMeta, Result, F15_GOBIBLE};

pub fn load(_b: &mut Bible, _filename: &str) -> Result<()> {
    Err(Error::Unsupported(F15_GOBIBLE))
}

pub fn export(_b: &mut Bible, _meta: &ExportMeta, _filename: &str, _filter_idx: i32) -> Result<()> {
    Err(Error::Unsupported(F15_GOBIBLE))
}
