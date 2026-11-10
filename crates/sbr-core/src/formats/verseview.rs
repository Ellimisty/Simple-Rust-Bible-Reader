//! Port of `VerseVIEWBibleFormat.cs`.

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use sbr_xml::Document;
use std::path::Path;

const REVISION: &str = "1";
const FONT: &str = "Arial,Helvetica,sans-serif,Calibri";
const COPYRIGHT: &str = "Public Domain, God, for Eternity";
const SIZE_FACTOR: &str = "1";

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut src = Document::new();
    src.load_file(filename)?;
    let sr = src.root();
    if let Some(t) = src.select_single_node(sr, "/bible/title")? {
        b.description = src.inner_text(t);
    }
    let book_names: Option<Vec<String>> = src
        .select_single_node(sr, "/bible/booknames")?
        .map(|n| src.inner_text(n).split(',').map(str::to_string).collect());

    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);
    let g = global_memory::get();
    let n = |node| src.attr(node, "n").unwrap_or_default();
    for (i, book) in src.select_nodes(sr, "/bible/b")?.into_iter().enumerate() {
        let name = match &book_names {
            Some(names) => names.get(i).map(String::as_str).unwrap_or_default(),
            None => n(book),
        };
        let bk = d.create_element("BIBLEBOOK");
        d.set_attribute(bk, "bnumber", &fu::get_book_no(name).to_string());
        d.append_child(root, bk);
        for chap in src.select_nodes(book, "c")? {
            let ch = d.create_element("CHAPTER");
            d.set_attribute(ch, "cnumber", n(chap));
            d.append_child(bk, ch);
            for v in src.select_nodes(chap, "v")? {
                let e = d.create_element("VERS");
                d.set_attribute(e, "vnumber", n(v));
                // The original read the text only when cleaning, otherwise
                // reusing the previous verse's (initially empty) text.
                let mut text = src.inner_text(v);
                if g.parse_bible {
                    if !g.parse_bible_bkgnd_cleaner {
                        text = crate::post_process::post_process_content(&text);
                    }
                    d.set_inner_text(e, &text);
                } else {
                    d.set_inner_xml(e, &fu::escape_xml(&text))?;
                }
                d.append_child(ch, e);
            }
        }
    }
    b.doc = d;
    Ok(())
}

pub fn export(b: &mut Bible, meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    let mut o = Document::new();
    let r = o.root();
    let dec = o.create_xml_declaration("1.0", None, None);
    o.append_child(r, dec);
    let root = o.create_element("bible");
    o.append_child(r, root);
    let fname = Path::new(filename)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    for (tag, text) in [
        ("fname", fname.as_str()),
        ("revision", REVISION),
        ("title", meta.abbreviation.as_str()),
        ("font", FONT),
        ("copyright", COPYRIGHT),
        ("sizefactor", SIZE_FACTOR),
    ] {
        let e = o.create_element(tag);
        o.set_inner_text(e, text);
        o.append_child(root, e);
    }
    let index = super::osis::verse_index(&b.doc);
    for book_id in 1..=66 {
        let book = o.create_element("b");
        o.set_attribute(book, "n", &fu::book_names()[book_id as usize - 1]);
        o.append_child(root, book);
        for chap in 1..=fu::get_chapter_count(book_id) {
            let ch = o.create_element("c");
            o.set_attribute(ch, "n", &chap.to_string());
            o.append_child(book, ch);
            for vers in 1..=fu::get_verse_count(book_id, chap) {
                let v = o.create_element("v");
                o.set_attribute(v, "n", &vers.to_string());
                let key = (book_id.to_string(), chap.to_string(), vers.to_string());
                let text = index
                    .get(&key)
                    .map(|&n| b.doc.inner_text(n))
                    .unwrap_or_default();
                o.set_inner_text(v, &text);
                o.append_child(ch, v);
            }
        }
    }
    let filename = if filename.ends_with(".xml") {
        filename.to_string()
    } else {
        format!("{filename}.xml")
    };
    let nl = global_memory::get().newline.clone();
    let out = Document::parse(&o.outer_xml(r))
        .map(|d| d.to_indented_string(d.root(), &nl))
        .unwrap_or_default();
    crate::io::write_utf8_bom(&filename, &out)?;
    Ok(())
}
