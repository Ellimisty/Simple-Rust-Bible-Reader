//! Port of `BibleFormat.cs`: format ids, shared Zefania document and dispatch.

use crate::formats;
use sbr_xml::{Document, NodeId};
use std::path::Path;

pub const F02_ZEFANIA_BIBLE: i32 = 1;
pub const F03_UNBOUND_BIBLE_BCV: i32 = 2;
pub const F04_UNBOUND_BIBLE_BCVS: i32 = 3;
pub const F05_OSIS: i32 = 4;
pub const F06_GBF: i32 = 5;
pub const F07_XSEM: i32 = 6;
pub const F08_THEWORD: i32 = 7;
pub const F09_EWORD_BBL: i32 = 8;
pub const F10_ESWORD_BBLX: i32 = 9;
pub const F11_ESWORD_BBLI: i32 = 10;
pub const F12_SWORDPROJECT: i32 = 11;
pub const F13_VPL: i32 = 12;
pub const F14_BIBLE_COMPANION: i32 = 13;
pub const F15_GOBIBLE: i32 = 14;
pub const F16_THML: i32 = 15;
pub const F17_OPENSONG: i32 = 16;
pub const F18_OPENLP: i32 = 17;
pub const F19_MYSWORD: i32 = 18;
pub const F20_BIBLE_ANALYZER: i32 = 19;
pub const F21_EASY_SLIDES: i32 = 20;
pub const F22_MP3: i32 = 21;
pub const F23_LOGOS: i32 = 22;
pub const F24_ONLINEBIBLE: i32 = 23;
pub const F25_PALMBIBLE: i32 = 24;
pub const F26_HEAVENWORLD: i32 = 25;
pub const F27_VERSEVIEW: i32 = 26;
pub const F28_MYBIBLE: i32 = 27;
pub const F29_USFX: i32 = 28;
pub const F30_SWORDSEARCHER: i32 = 29;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("xml: {0}")]
    Xml(#[from] sbr_xml::ParseError),
    #[error("xpath: {0:?}")]
    XPath(sbr_xml::xpath::XPathError),
    #[error("format {0} is not ported yet")]
    Unsupported(i32),
    #[error("could not detect the format of {0}")]
    Unknown(String),
    #[error("{0}")]
    Other(String),
}

impl From<sbr_xml::xpath::XPathError> for Error {
    fn from(e: sbr_xml::xpath::XPathError) -> Self {
        Error::XPath(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// A loaded Bible: the original's static `BibleXmlDocument` plus the
/// metadata fields of the `BibleFormat` instance that loaded it.
pub struct Bible {
    pub doc: Document,
    pub abbreviation: String,
    pub description: String,
    pub comments: String,
    pub cleaning: bool,
    pub filter_index: i32,
}

impl Bible {
    pub fn new(filter_index: i32) -> Self {
        Self {
            doc: Document::new(),
            abbreviation: "BIB".into(),
            description: "Bible".into(),
            comments: format!(
                "Exported from Simple Bible Reader ({})",
                crate::AUTHOR_WEBSITE
            ),
            cleaning: false,
            filter_index,
        }
    }

    /// `BibleFormat.getVerse` node lookup.
    pub fn verse_node(&self, book: i32, chapter: i32, verse: i32) -> Option<NodeId> {
        let x = format!("/XMLBIBLE/BIBLEBOOK[@bnumber='{book}']/CHAPTER[@cnumber='{chapter}']/VERS[@vnumber='{verse}']");
        self.doc
            .select_single_node(self.doc.root(), &x)
            .ok()
            .flatten()
    }

    pub fn get_verse(&self, book: i32, chapter: i32, verse: i32) -> Option<String> {
        self.verse_node(book, chapter, verse)
            .map(|n| self.doc.inner_xml(n))
    }
}

/// Metadata as adjusted by `doExportBible` for the export target.
pub struct ExportMeta {
    pub abbreviation: String,
    pub description: String,
    pub comments: String,
    pub cleaning: bool,
}

pub fn export_meta(b: &Bible, filename: &str) -> ExportMeta {
    let stem = Path::new(filename)
        .file_stem()
        .map(|s| s.to_string_lossy().replace('\'', "''"))
        .unwrap_or_default();
    let mut m = ExportMeta {
        abbreviation: b.abbreviation.replace('\'', "''"),
        description: b.description.replace('\'', "''"),
        comments: b.comments.replace('\'', "''"),
        cleaning: b.cleaning,
    };
    if m.abbreviation == "BIB" {
        m.abbreviation = stem.clone();
    }
    if m.description == "Bible" {
        m.description = stem;
    }
    m
}

pub fn cleaning_required(filter_idx: i32) -> bool {
    [F09_EWORD_BBL, F10_ESWORD_BBLX, F11_ESWORD_BBLI].contains(&filter_idx)
}

pub fn open_not_supported(filter_idx: i32) -> bool {
    [F22_MP3, F23_LOGOS].contains(&filter_idx)
}

/// Format detection subset of `BibleFactory(filename)`.
pub fn detect(filename: &str) -> Result<i32> {
    let lower = filename.to_lowercase();
    if lower.ends_with(".xml") {
        let head = std::fs::read(filename)?;
        let head = String::from_utf8_lossy(&head[..head.len().min(4096)]).to_lowercase();
        if head.contains("<xmlbible") {
            return Ok(F02_ZEFANIA_BIBLE);
        }
    } else if lower.ends_with(".txt") {
        let text = crate::io::read_text_default(filename)?;
        let first = crate::io::read_lines(&text).first().copied().unwrap_or("");
        return if first.starts_with(';') {
            Ok(F30_SWORDSEARCHER)
        } else {
            Ok(F03_UNBOUND_BIBLE_BCV)
        };
    } else if lower.ends_with(".vpl") {
        return Ok(F13_VPL);
    } else if lower.ends_with(".bbl.mybible.gz") || lower.ends_with(".bbl.mybible") {
        return Ok(F19_MYSWORD);
    }
    Err(Error::Unknown(filename.into()))
}

/// `BibleFactory(filename, filterIdx)` followed by `Load()`.
pub fn load(filename: &str, filter_idx: i32) -> Result<Bible> {
    let idx = if filter_idx == 0 {
        detect(filename)?
    } else {
        filter_idx
    };
    let mut b = Bible::new(idx);
    b.cleaning = cleaning_required(idx);
    match idx {
        F02_ZEFANIA_BIBLE => formats::zefania::load(&mut b, filename)?,
        F03_UNBOUND_BIBLE_BCV | F04_UNBOUND_BIBLE_BCVS => formats::unbound::load(&mut b, filename)?,
        F13_VPL => formats::vpl::load(&mut b, filename)?,
        F19_MYSWORD => formats::mysword::load(&mut b, filename)?,
        _ => return Err(Error::Unsupported(idx)),
    }
    Ok(b)
}

/// `doExportBible(filename, idx).ExportBible(filename, idx)`.
pub fn export(b: &mut Bible, filename: &str, filter_idx: i32) -> Result<()> {
    let meta = export_meta(b, filename);
    match filter_idx {
        F02_ZEFANIA_BIBLE => formats::zefania::export(b, &meta, filename),
        F03_UNBOUND_BIBLE_BCV | F04_UNBOUND_BIBLE_BCVS => {
            formats::unbound::export(b, &meta, filename, filter_idx)
        }
        F13_VPL => formats::vpl::export(b, &meta, filename),
        F19_MYSWORD => formats::mysword::export(b, &meta, filename),
        _ => Err(Error::Unsupported(filter_idx)),
    }
}
