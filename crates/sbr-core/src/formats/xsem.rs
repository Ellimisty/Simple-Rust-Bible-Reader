//! Port of `XSEMBibleFormat.cs` (XML Scripture Encoding Model).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use fancy_regex::Regex;
use sbr_xml::{Document, NodeId};
use std::collections::HashMap;
use std::sync::OnceLock;

const BOOK_NAMES: [&str; 66] = [
    "GEN", "EXO", "LEV", "NUM", "DEU", "JOS", "JDG", "RUT", "1SA", "2SA", "1KI", "2KI", "1CH",
    "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM", "EZK",
    "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL",
    "MAT", "MRK", "LUK", "JHN", "ACT", "ROM", "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH",
    "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE", "2PE", "1JN", "2JN", "3JN", "JUD",
    "REV",
];

const SCRIPTURE_HEADER: &str = include_str!("../../resources/xsem_scripture_header.xml");

fn regexes() -> &'static [(Regex, &'static str); 7] {
    static R: OnceLock<[(Regex, &'static str); 7]> = OnceLock::new();
    R.get_or_init(|| {
        let r = |p: &str| Regex::new(p).unwrap();
        [
            (
                r("(<)(book )([^<]*)(value=)([^<]*)(>)"),
                "<BIBLEBOOK bnumber=${5}>",
            ),
            (r("<verseEnd[^<]*/>"), "</VERS>"),
            (
                r("(<)(verse )([^<]*)(value=)([^<]*)(/>)"),
                "<VERS vnumber=${5}>",
            ),
            (r("<chapterEnd[^<]*/>"), "</CHAPTER>"),
            (
                r("(<)(chapter )([^<]*)(value=)([^<]*)(/>)"),
                "<CHAPTER cnumber=${5}>",
            ),
            (r("</?(?!((/?)BIBLEBOOK|(/?)CHAPTER|(/?)VERS))[^>]*>"), ""),
            (
                r("(<)(VERS[^<]*)(>)([^(</)]*)(<VERS)"),
                "${1}${2}${3}${4}</VERS>${5}",
            ),
        ]
    })
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut text = crate::io::read_text_default(filename)?.replace("</book>", "</BIBLEBOOK>");
    let (rx, fix) = regexes().split_at(6);
    for (re, rep) in rx {
        text = re.replace_all(&text, *rep).into_owned();
    }
    text = format!("<XMLBIBLE>{text}</XMLBIBLE>");
    for (i, name) in BOOK_NAMES.iter().enumerate() {
        text = text.replace(&format!("\"{name}\""), &format!("\"{}\"", i + 1));
    }
    let mut d = Document::new();
    if d.load_xml(&text).is_err() {
        // The original retries once after closing verses left open.
        let text = fix[0].0.replace_all(&text, fix[0].1).into_owned();
        d = Document::new();
        d.load_xml(&text)?;
    }
    b.doc = d;
    Ok(())
}

fn verse_index(doc: &Document) -> HashMap<(String, String, String), NodeId> {
    let mut m = HashMap::new();
    let Some(top) = doc
        .document_element()
        .filter(|&t| doc.name(t) == "XMLBIBLE")
    else {
        return m;
    };
    let kids =
        |n: NodeId, name: &'static str| doc.child_elements(n).filter(move |&c| doc.name(c) == name);
    for bk in kids(top, "BIBLEBOOK") {
        let Some(bn) = doc.attr(bk, "bnumber") else {
            continue;
        };
        for ch in kids(bk, "CHAPTER") {
            let Some(cn) = doc.attr(ch, "cnumber") else {
                continue;
            };
            for v in kids(ch, "VERS") {
                let Some(vn) = doc.attr(v, "vnumber") else {
                    continue;
                };
                m.entry((bn.to_string(), cn.to_string(), vn.to_string()))
                    .or_insert(v);
            }
        }
    }
    m
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
    let root = o.create_element("scripture");
    o.append_child(r, root);
    let hdr = o.create_element("scriptureHeader");
    o.set_inner_xml(hdr, SCRIPTURE_HEADER.trim_start_matches('\u{feff}'))?;
    o.append_child(root, hdr);

    let index = verse_index(&b.doc);
    for book_id in 1..=66 {
        // Like the original, every book opens a new canon element.
        let id = if book_id < 40 { "OT" } else { "NT" };
        let canon = {
            let c = o.create_element("canon");
            o.set_attribute(c, "id", id);
            o.set_attribute(c, "value", id);
            o.append_child(root, c);
            c
        };
        let book = o.create_element("book");
        o.set_attribute(book, "value", BOOK_NAMES[book_id as usize - 1]);
        o.append_child(canon, book);
        let mut chap_present = false;
        for chap in 1..=fu::get_chapter_count(book_id) {
            let ch = o.create_element("chapter");
            o.set_attribute(ch, "value", &chap.to_string());
            o.append_child(book, ch);
            let mut verse_present = false;
            for vers in 1..=fu::get_verse_count(book_id, chap) {
                let key = (book_id.to_string(), chap.to_string(), vers.to_string());
                let Some(&src) = index.get(&key) else {
                    continue;
                };
                let v = o.create_element("verse");
                o.set_attribute(v, "value", &vers.to_string());
                o.append_child(book, v);
                let t = o.create_text_node(&b.doc.inner_text(src));
                o.append_child(book, t);
                let end = o.create_element("verseEnd");
                o.append_child(book, end);
                verse_present = true;
            }
            if verse_present {
                let end = o.create_element("chapterEnd");
                o.append_child(book, end);
                chap_present = true;
            } else {
                o.remove_child(book, ch);
            }
        }
        if !chap_present {
            o.remove_child(canon, book);
        }
    }

    let nl = global_memory::get().newline.clone();
    let out = Document::parse(&o.outer_xml(r))
        .map(|d| d.to_indented_string(d.root(), &nl))
        .unwrap_or_default();
    crate::io::write_utf8_bom(&filename, &out)?;
    Ok(())
}
