//! Port of `OpenSongBibleFormat.cs` (`.xmm`).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use sbr_xml::Document;

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut src = Document::new();
    src.load_file(filename)?;
    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);
    let g = global_memory::get();
    let n = |node| src.attr(node, "n").unwrap_or_default();
    for book in src.select_nodes(src.root(), "/bible/b")? {
        let bk = d.create_element("BIBLEBOOK");
        d.set_attribute(bk, "bnumber", &fu::get_book_no(n(book)).to_string());
        d.append_child(root, bk);
        for chap in src.select_nodes(book, "c")? {
            let ch = d.create_element("CHAPTER");
            d.set_attribute(ch, "cnumber", n(chap));
            d.append_child(bk, ch);
            for v in src.select_nodes(chap, "v")? {
                let e = d.create_element("VERS");
                d.set_attribute(e, "vnumber", n(v));
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

pub fn export(b: &mut Bible, _meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    let mut o = Document::new();
    let r = o.root();
    let dec = o.create_xml_declaration("1.0", None, None);
    o.append_child(r, dec);
    let root = o.create_element("bible");
    o.append_child(r, root);
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
    let filename = if filename.ends_with(".xmm") {
        filename.to_string()
    } else {
        format!("{filename}.xmm")
    };
    let nl = global_memory::get().newline.clone();
    let out = Document::parse(&o.outer_xml(r))
        .map(|d| d.to_indented_string(d.root(), &nl))
        .unwrap_or_default();
    crate::io::write_utf8_bom(&filename, &out)?;
    Ok(())
}
