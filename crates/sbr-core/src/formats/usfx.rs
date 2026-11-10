//! Port of `USFXBibleFormat.cs`.

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use fancy_regex::Regex;
use sbr_xml::Document;
use std::sync::OnceLock;

const USFX_NS: &str = "http://eBible.org/usfx/usfx-2009-07-12.xsd";
const BOOK_NAMES: [&str; 66] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
    "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
    "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
    "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH",
    "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD",
    "REV",
];

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).unwrap())
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut src = Document::new();
    src.load_file(filename)?;
    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);
    for book in src.select_nodes(src.root(), "//book")? {
        let id = src.attr(book, "id").unwrap_or_default();
        if id == "FRT" || id == "BAK" {
            continue;
        }
        // The original's "not found" check never fired and wrote bnumber 0.
        let Some(pos) = BOOK_NAMES.iter().position(|&n| n == id) else {
            continue;
        };
        let bk = d.create_element("BIBLEBOOK");
        d.set_attribute(bk, "bnumber", &(pos + 1).to_string());
        d.append_child(root, bk);
        let _ = d.set_inner_xml(bk, &format_book_xml(&src.inner_xml(book)));
    }
    b.doc = d;
    Ok(())
}

fn format_book_xml(text: &str) -> String {
    static V: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Regex> = OnceLock::new();
    static HEAD: OnceLock<Regex> = OnceLock::new();
    let t = re(&V, "(<)(v )([^<]*)(id=)([^<]*)(/>)").replace_all(text, "<VERS vnumber=${5}>");
    let t = re(&C, "(<)(c )([^<]*)(id=)([^<]*)(/>)").replace_all(&t, "<CHAPTER cnumber=${5}>");
    let t = re(&TAGS, "</?(?!((/?)CHAPTER|(/?)VERS))[^>]*>").replace_all(&t, "");
    let t = t
        .replace("<VERS", "</VERS><VERS")
        .replace("<CHAPTER", "</VERS></CHAPTER><CHAPTER");
    const CLOSE: &str = "</VERS></CHAPTER>";
    // C#: Substring(IndexOf(CLOSE) + CLOSE.Length), with IndexOf -1 if absent.
    let start = t.find(CLOSE).map_or(CLOSE.len() - 1, |i| i + CLOSE.len());
    let t = t.get(start..).unwrap_or_default();
    let t = re(&HEAD, "<CHAPTER([^>]*)>([^>]*)</VERS>").replace_all(t, "<CHAPTER${1}>");
    format!("{t}{CLOSE}")
}

pub fn export(b: &mut Bible, _meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    let filename = if filename.ends_with(".xml") {
        filename.to_string()
    } else {
        format!("{filename}.xml")
    };
    let mut o = Document::new();
    let r = o.root();
    let dec = o.create_xml_declaration("1.0", None, None);
    o.append_child(r, dec);
    let root = o.create_element_ns("usfx", USFX_NS);
    o.append_child(r, root);
    let index = super::osis::verse_index(&b.doc);
    for book_id in 1..=66 {
        let book = o.create_element("book");
        o.set_attribute(book, "id", BOOK_NAMES[book_id as usize - 1]);
        o.append_child(root, book);
        let mut chap_present = false;
        for chap in 1..=fu::get_chapter_count(book_id) {
            let c = o.create_element("c");
            o.set_attribute(c, "id", &chap.to_string());
            o.append_child(book, c);
            let mut verse_present = false;
            for vers in 1..=fu::get_verse_count(book_id, chap) {
                let key = (book_id.to_string(), chap.to_string(), vers.to_string());
                let Some(&src) = index.get(&key) else {
                    continue;
                };
                let v = o.create_element("v");
                o.set_attribute(v, "id", &vers.to_string());
                o.append_child(book, v);
                let t = o.create_text_node(&b.doc.inner_text(src));
                o.append_child(book, t);
                verse_present = true;
            }
            if verse_present {
                chap_present = true;
            } else {
                o.remove_child(book, c);
            }
        }
        if !chap_present {
            o.remove_child(root, book);
        }
    }
    let nl = global_memory::get().newline.clone();
    let out = Document::parse(&o.outer_xml(r))
        .map(|d| d.to_indented_string(d.root(), &nl))
        .unwrap_or_default();
    crate::io::write_utf8_bom(&filename, &out)?;
    Ok(())
}
