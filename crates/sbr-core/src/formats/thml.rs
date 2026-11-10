//! Port of `ThMLBibleFormat.cs` (Theological Markup Language).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use fancy_regex::Regex;
use sbr_xml::{Document, NodeId};
use std::sync::OnceLock;

const THML_BOOK_IDS: [&str; 66] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "iSam", "iiSam", "iKgs", "iiKgs",
    "iChr", "iiChr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer",
    "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph",
    "Hag", "Zech", "Mal", "Matt", "Mark", "Luke", "John", "Acts", "Rom", "iCor", "iiCor", "Gal",
    "Eph", "Phil", "Col", "iThess", "iiThess", "iTim", "iiTim", "Titus", "Phlm", "Heb", "Jas",
    "iPet", "iiPet", "iJohn", "iiJohn", "iiiJohn", "Jude", "Rev",
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

    for book in src.select_nodes(src.root(), "//div2")? {
        let Some(id) = src.attr(book, "id") else {
            continue;
        };
        let Some(pos) = THML_BOOK_IDS
            .iter()
            .position(|b| b.eq_ignore_ascii_case(id))
        else {
            continue;
        };
        let bk = d.create_element("BIBLEBOOK");
        d.set_attribute(bk, "bnumber", &(pos + 1).to_string());
        d.set_attribute(bk, "bname", src.attr(book, "title").unwrap_or_default());
        d.append_child(root, bk);
        for (n, chap) in src.select_nodes(book, "div3")?.into_iter().enumerate() {
            let ch = d.create_element("CHAPTER");
            d.set_attribute(ch, "cnumber", &(n + 1).to_string());
            add_verses(&mut d, ch, &src, chap)?;
            d.append_child(bk, ch);
        }
    }
    b.doc = d;
    Ok(())
}

/// `FormatXML`: verses are the text between `<scripture/>` markers.
fn add_verses(d: &mut Document, ch: NodeId, src: &Document, chap: NodeId) -> Result<()> {
    static TAGS: OnceLock<Regex> = OnceLock::new();
    static MARK: OnceLock<Regex> = OnceLock::new();
    let xml = src
        .inner_xml(chap)
        .replace("<sup>", "<!--")
        .replace("</sup>", "-->");
    let xml = re(&TAGS, "</?(?!((/?)scripture))[^>]*>").replace_all(&xml, "");
    let xml = re(&MARK, "<scripture([^>]*)/>").replace_all(&xml, "</VERS><VERS>");
    let tmp = Document::parse(&format!("<XML><VERS>{xml}</VERS></XML>"))?;
    let verses = tmp.select_nodes(tmp.root(), "/XML/VERS")?;
    let g = global_memory::get();
    // The original stopped one short (`i < Count - 1`), dropping each
    // chapter's last verse.
    for (i, &v) in verses.iter().enumerate().skip(1) {
        let e = d.create_element("VERS");
        d.set_attribute(e, "vnumber", &i.to_string());
        let mut text = tmp.inner_text(v);
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
    Ok(())
}

pub fn export(b: &mut Bible, _meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    let mut o = Document::new();
    let r = o.root();
    let dec = o.create_xml_declaration("1.0", None, None);
    o.append_child(r, dec);
    let thml = o.create_element("ThML");
    o.append_child(r, thml);
    let body = o.create_element("ThML.body");
    o.append_child(thml, body);
    let div1 = o.create_element("div1");
    o.append_child(body, div1);

    let index = super::osis::verse_index(&b.doc);
    let br = b.doc.root();
    for book_id in 1..=66 {
        let src_book = b
            .doc
            .select_single_node(br, &format!("/XMLBIBLE/BIBLEBOOK[@bnumber='{book_id}']"))?;
        let title = src_book
            .and_then(|n| b.doc.attr(n, "id"))
            .map(str::to_string)
            .unwrap_or_else(|| fu::book_names()[book_id as usize - 1].clone());
        let book = o.create_element("div2");
        o.set_attribute(book, "title", &title);
        o.set_attribute(book, "id", THML_BOOK_IDS[book_id as usize - 1]);
        o.append_child(div1, book);
        let mut chapters_present = false;
        for chap in 1..=fu::get_chapter_count(book_id) {
            let ch = o.create_element("div3");
            o.set_attribute(ch, "title", &format!("Chapter {chap}"));
            o.append_child(book, ch);
            let p = o.create_element("p");
            o.append_child(ch, p);
            let mut verse_present = false;
            for vers in 1..=fu::get_verse_count(book_id, chap) {
                let key = (book_id.to_string(), chap.to_string(), vers.to_string());
                let Some(&src) = index.get(&key) else {
                    continue;
                };
                let s = o.create_element("scripture");
                o.append_child(p, s);
                let t = o.create_text_node(&b.doc.inner_text(src));
                o.append_child(p, t);
                verse_present = true;
            }
            if verse_present {
                chapters_present = true;
            } else {
                o.remove_child(book, ch);
            }
        }
        if !chapters_present {
            o.remove_child(div1, book);
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
