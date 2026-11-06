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

/// Format detection of `BibleFactory(filename)`.
pub fn detect(filename: &str) -> Result<i32> {
    let lower = filename.to_lowercase();
    let ends = |exts: &[&str]| exts.iter().any(|e| lower.ends_with(e));
    let id = if ends(&[".xml"]) {
        let xml = String::from_utf8_lossy(&std::fs::read(filename)?)
            .trim()
            .to_lowercase();
        if xml.contains("<xmlbible") {
            F02_ZEFANIA_BIBLE
        } else if xml.contains("<osis") {
            F05_OSIS
        } else if xml.contains("<usfx") {
            F29_USFX
        } else if xml.contains("<thml") {
            F16_THML
        } else if xml.contains("<scripture") {
            F07_XSEM
        } else if xml.contains("<bible") {
            if xml.contains("<fname") {
                F27_VERSEVIEW
            } else {
                F17_OPENSONG
            }
        } else {
            0
        }
    } else if ends(&[".thm", ".thml"]) {
        F16_THML
    } else if ends(&[".txt"]) {
        let text = crate::io::read_text_default(filename)?;
        let first = crate::io::read_lines(&text).first().copied().unwrap_or("");
        if first.starts_with(';') {
            F30_SWORDSEARCHER
        } else {
            F03_UNBOUND_BIBLE_BCV
        }
    } else if ends(&[".gbf"]) {
        F06_GBF
    } else if ends(&[".ont", ".ot", ".nt", ".ontx", ".otx", ".ntx"]) {
        F08_THEWORD
    } else if ends(&[".bbl"]) {
        F09_EWORD_BBL
    } else if ends(&[".bblx"]) {
        F10_ESWORD_BBLX
    } else if ends(&[".bbli"]) {
        F11_ESWORD_BBLI
    } else if ends(&[".zip"]) {
        F12_SWORDPROJECT
    } else if ends(&[".vpl"]) {
        F13_VPL
    } else if ends(&[".jar"]) {
        F15_GOBIBLE
    } else if ends(&[".xmm"]) {
        F17_OPENSONG
    } else if ends(&[".sqlite"]) {
        F18_OPENLP
    } else if ends(&[".mdb"]) {
        F21_EASY_SLIDES
    } else if ends(&[".hbb"]) {
        F26_HEAVENWORLD
    } else if ends(&[".bib"]) {
        // Bible Analyzer and Bible Companion share the extension.
        let mut head = [0u8; 6];
        let n = std::io::Read::read(&mut std::fs::File::open(filename)?, &mut head)?;
        if &head[..n] == b"SQLite" {
            F20_BIBLE_ANALYZER
        } else {
            F14_BIBLE_COMPANION
        }
    } else if ends(&[".bbl.mybible.gz", ".bbl.mybible"]) {
        F19_MYSWORD
    } else if ends(&[".mp3"]) {
        F22_MP3
    } else if ends(&[".docx"]) {
        F23_LOGOS
    } else if ends(&[".exp"]) {
        F24_ONLINEBIBLE
    } else if ends(&[".sqlite3"]) && !ends(&[".commentaries.sqlite3", ".dictionary.sqlite3"]) {
        F28_MYBIBLE
    } else if ends(&[".pdb"]) {
        F25_PALMBIBLE
    } else if ends(&[".ss8bible", ".ss7bible", ".ss6bible", ".ss5bible"]) {
        F30_SWORDSEARCHER
    } else if Path::new(filename).join("mods.d").is_dir() {
        F12_SWORDPROJECT
    } else {
        0
    };
    if id == 0 {
        return Err(Error::Unknown(filename.into()));
    }
    Ok(id)
}

/// `BibleFactory(filename, filterIdx)` followed by `Load()`.
pub fn load(filename: &str, filter_idx: i32) -> Result<Bible> {
    let detected = filter_idx == 0;
    let idx = if detected {
        detect(filename)?
    } else {
        filter_idx
    };
    let mut b = Bible::new(idx);
    b.cleaning = cleaning_required(idx);
    if detected {
        // Only the auto-detecting factory names the Bible after its file.
        let stem = Path::new(filename)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        b.abbreviation = stem.clone();
        b.description = stem;
    }
    match idx {
        F02_ZEFANIA_BIBLE => formats::zefania::load(&mut b, filename)?,
        F03_UNBOUND_BIBLE_BCV | F04_UNBOUND_BIBLE_BCVS => formats::unbound::load(&mut b, filename)?,
        F13_VPL => formats::vpl::load(&mut b, filename)?,
        F19_MYSWORD => formats::mysword::load(&mut b, filename)?,
        F05_OSIS => formats::osis::load(&mut b, filename)?,
        F06_GBF => formats::gbf::load(&mut b, filename)?,
        F07_XSEM => formats::xsem::load(&mut b, filename)?,
        F08_THEWORD => formats::theword::load(&mut b, filename)?,
        F09_EWORD_BBL => formats::bbl::load(&mut b, filename)?,
        F10_ESWORD_BBLX => formats::bblx::load(&mut b, filename)?,
        F11_ESWORD_BBLI => formats::bbli::load(&mut b, filename)?,
        F12_SWORDPROJECT => formats::sword::load(&mut b, filename)?,
        F14_BIBLE_COMPANION => formats::bible_companion::load(&mut b, filename)?,
        F15_GOBIBLE => formats::gobible::load(&mut b, filename)?,
        F16_THML => formats::thml::load(&mut b, filename)?,
        F17_OPENSONG => formats::opensong::load(&mut b, filename)?,
        F18_OPENLP => formats::openlp::load(&mut b, filename)?,
        F20_BIBLE_ANALYZER => formats::bible_analyzer::load(&mut b, filename)?,
        F21_EASY_SLIDES => formats::easyslides::load(&mut b, filename)?,
        F22_MP3 => formats::mp3::load(&mut b, filename)?,
        F23_LOGOS => formats::logos::load(&mut b, filename)?,
        F24_ONLINEBIBLE => formats::online_bible::load(&mut b, filename)?,
        F25_PALMBIBLE => formats::pdb::load(&mut b, filename)?,
        F26_HEAVENWORLD => formats::heavenworld::load(&mut b, filename)?,
        F27_VERSEVIEW => formats::verseview::load(&mut b, filename)?,
        F28_MYBIBLE => formats::mybible::load(&mut b, filename)?,
        F29_USFX => formats::usfx::load(&mut b, filename)?,
        F30_SWORDSEARCHER => formats::swordsearcher::load(&mut b, filename)?,
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
        F05_OSIS => formats::osis::export(b, &meta, filename, filter_idx),
        F06_GBF => formats::gbf::export(b, &meta, filename, filter_idx),
        F07_XSEM => formats::xsem::export(b, &meta, filename, filter_idx),
        F08_THEWORD => formats::theword::export(b, &meta, filename, filter_idx),
        F09_EWORD_BBL => formats::bbl::export(b, &meta, filename, filter_idx),
        F10_ESWORD_BBLX => formats::bblx::export(b, &meta, filename, filter_idx),
        F11_ESWORD_BBLI => formats::bbli::export(b, &meta, filename, filter_idx),
        F12_SWORDPROJECT => formats::sword::export(b, &meta, filename, filter_idx),
        F14_BIBLE_COMPANION => formats::bible_companion::export(b, &meta, filename, filter_idx),
        F15_GOBIBLE => formats::gobible::export(b, &meta, filename, filter_idx),
        F16_THML => formats::thml::export(b, &meta, filename, filter_idx),
        F17_OPENSONG => formats::opensong::export(b, &meta, filename, filter_idx),
        F18_OPENLP => formats::openlp::export(b, &meta, filename, filter_idx),
        F20_BIBLE_ANALYZER => formats::bible_analyzer::export(b, &meta, filename, filter_idx),
        F21_EASY_SLIDES => formats::easyslides::export(b, &meta, filename, filter_idx),
        F22_MP3 => formats::mp3::export(b, &meta, filename, filter_idx),
        F23_LOGOS => formats::logos::export(b, &meta, filename, filter_idx),
        F24_ONLINEBIBLE => formats::online_bible::export(b, &meta, filename, filter_idx),
        F25_PALMBIBLE => formats::pdb::export(b, &meta, filename, filter_idx),
        F26_HEAVENWORLD => formats::heavenworld::export(b, &meta, filename, filter_idx),
        F27_VERSEVIEW => formats::verseview::export(b, &meta, filename, filter_idx),
        F28_MYBIBLE => formats::mybible::export(b, &meta, filename, filter_idx),
        F29_USFX => formats::usfx::export(b, &meta, filename, filter_idx),
        F30_SWORDSEARCHER => formats::swordsearcher::export(b, &meta, filename, filter_idx),
        _ => Err(Error::Unsupported(filter_idx)),
    }
}
