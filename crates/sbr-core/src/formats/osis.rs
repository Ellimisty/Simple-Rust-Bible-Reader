//! Port of `OSISBibleFormat.cs` (Open Scripture Information Standard).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::format_util as fu;
use crate::global_memory;
use fancy_regex::Regex;
use sbr_xml::xpath::Namespaces;
use sbr_xml::{Document, NodeId};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

const OSIS_NS: &str = "http://www.bibletechnologies.net/2003/OSIS/namespace";
const SUBJECT: &str = "Holy Bible";
const RIGHTS: &str = "Scripture is a fact. Facts remain as facts even after translation. Facts cannot be copyrighted. Hence, all scriptures are on Public Domain.";

/// `/books/book/osisID` of `bookid.xml`, in book order.
fn osis_book_names() -> &'static [String] {
    static N: OnceLock<Vec<String>> = OnceLock::new();
    N.get_or_init(|| {
        let d = &fu::tables().book_doc;
        d.select_nodes(d.root(), "/books/book/osisID")
            .unwrap()
            .into_iter()
            .map(|n| d.inner_text(n))
            .collect()
    })
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).unwrap())
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut strxml = crate::io::read_text_default(filename)?;
    // The original first requires the file to be well-formed XML.
    Document::parse(&strxml)?;

    // Then it flattens milestone verses and drops every non-verse tag.
    static SID: OnceLock<Regex> = OnceLock::new();
    static EID: OnceLock<Regex> = OnceLock::new();
    static TAGS: OnceLock<Regex> = OnceLock::new();
    strxml = re(&SID, "<verse([^>]*)sID([^>]*)/>")
        .replace_all(&strxml, "<verse${1}sID${2}>")
        .into_owned();
    strxml = re(&EID, "<verse([^>]*)eID([^>]*)/>")
        .replace_all(&strxml, "</verse>")
        .into_owned();
    strxml = re(&TAGS, "</?(?!((/?)verse))[^>]*>")
        .replace_all(&strxml, "")
        .into_owned();
    let src = Document::parse(&format!("<xml>{strxml}</xml>"))?;

    let mut verses = src.select_nodes(src.root(), "//verse")?;
    if verses.is_empty() {
        let mut ns = Namespaces::new();
        ns.add("o", OSIS_NS);
        verses = src.select_nodes_ns(src.root(), "//o:verse", &ns)?;
    }

    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);
    let mut st = State {
        root,
        book: None,
        chap: None,
        p_book: -1,
        p_chap: -1,
        p_verse: -1,
    };
    for v in verses {
        process_line(&mut d, &mut st, &src, v);
    }
    b.doc = d;
    Ok(())
}

struct State {
    root: NodeId,
    book: Option<NodeId>,
    chap: Option<NodeId>,
    p_book: i32,
    p_chap: i32,
    p_verse: i32,
}

fn process_line(d: &mut Document, st: &mut State, src: &Document, verse: NodeId) {
    let Some(id) = src.attr(verse, "osisID") else {
        return;
    };
    // The original throws on osisIDs that are not `Book.Chapter.Verse`.
    let parts: Vec<&str> = id.split('.').collect();
    let (Some(cn), Some(vn)) = (
        parts.get(1).and_then(|s| crate::dotnet::int_parse(s)),
        parts.get(2).and_then(|s| crate::dotnet::int_parse(s)),
    ) else {
        return;
    };
    let bn = osis_book_names()
        .iter()
        .position(|n| n == parts[0])
        .map_or(0, |i| i as i32 + 1);
    let mut text = src.inner_text(verse);

    let restart = cn == 1 && vn == 1 && st.p_verse >= vn;
    if bn != st.p_book || restart {
        let e = d.create_element("BIBLEBOOK");
        d.set_attribute(e, "bnumber", &bn.to_string());
        d.append_child(st.root, e);
        st.book = Some(e);
    }
    if cn != st.p_chap || restart {
        let e = d.create_element("CHAPTER");
        d.set_attribute(e, "cnumber", &cn.to_string());
        if let Some(bk) = st.book {
            d.append_child(bk, e);
        }
        st.chap = Some(e);
    }
    if vn != st.p_verse || cn != st.p_chap || bn != st.p_book {
        let e = d.create_element("VERS");
        let g = global_memory::get();
        if g.parse_bible {
            if !g.parse_bible_bkgnd_cleaner {
                text = crate::post_process::post_process_content(&text);
            }
            d.set_inner_text(e, &text);
        } else {
            let _ = d.set_inner_xml(e, &fu::escape_xml(&text));
        }
        d.set_attribute(e, "vnumber", &vn.to_string());
        if let Some(ch) = st.chap {
            d.append_child(ch, e);
        }
    }
    st.p_book = bn;
    st.p_chap = cn;
    st.p_verse = vn;
}

fn text_el(d: &mut Document, parent: NodeId, name: &str, text: Option<&str>) -> NodeId {
    let e = d.create_element(name);
    if let Some(t) = text {
        d.set_inner_text(e, t);
    }
    d.append_child(parent, e);
    e
}

/// Verses of the Zefania document keyed by their attribute strings; the
/// first match wins, like the original's `SelectSingleNode`.
fn verse_index(doc: &Document) -> HashMap<(String, String, String), NodeId> {
    let mut m = HashMap::new();
    let Some(top) = doc.document_element() else {
        return m;
    };
    if doc.name(top) != "XMLBIBLE" {
        return m;
    }
    for bk in doc
        .child_elements(top)
        .filter(|&n| doc.name(n) == "BIBLEBOOK")
    {
        let Some(bn) = doc.attr(bk, "bnumber") else {
            continue;
        };
        for ch in doc.child_elements(bk).filter(|&n| doc.name(n) == "CHAPTER") {
            let Some(cn) = doc.attr(ch, "cnumber") else {
                continue;
            };
            for v in doc.child_elements(ch).filter(|&n| doc.name(n) == "VERS") {
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

pub fn export(b: &mut Bible, meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    let filename = if filename.ends_with(".xml") {
        filename.to_string()
    } else {
        format!("{filename}.xml")
    };
    let stem = Path::new(&filename)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let mut o = Document::new();
    let r = o.root();
    let dec = o.create_xml_declaration("1.0", None, None);
    o.append_child(r, dec);
    let osis = o.create_element_ns("osis", OSIS_NS);
    o.append_child(r, osis);
    let text = o.create_element("osisText");
    o.set_attribute(text, "osisRefWork", "Bible");
    o.set_attribute(text, "osisIDWork", &stem);
    o.append_child(osis, text);

    let header = o.create_element("header");
    let rev = text_el(&mut o, header, "revisionDesc", None);
    text_el(&mut o, rev, "date", Some(&today));
    text_el(&mut o, rev, "p", Some(&meta.comments));
    let work = text_el(&mut o, header, "work", None);
    text_el(&mut o, work, "title", Some(&meta.abbreviation));
    text_el(&mut o, work, "contributor", None);
    text_el(&mut o, work, "creator", None);
    text_el(&mut o, work, "subject", Some(SUBJECT));
    text_el(&mut o, work, "date", Some(&today));
    text_el(&mut o, work, "description", Some(&meta.description));
    text_el(&mut o, work, "publisher", None);
    let ty = text_el(&mut o, work, "type", Some("Bible"));
    o.set_attribute(ty, "type", "OSIS");
    let ident = text_el(&mut o, work, "identifier", Some(&stem));
    o.set_attribute(ident, "type", "OSIS");
    text_el(&mut o, work, "rights", Some(RIGHTS));
    text_el(&mut o, work, "refSystem", Some("Bible"));
    o.append_child(text, header);

    let names = osis_book_names();
    let index = verse_index(&b.doc);
    for book in 1..=66 {
        let osis_id = &names[book as usize - 1];
        let div = o.create_element("div");
        o.set_attribute(div, "type", "book");
        o.set_attribute(div, "osisID", osis_id);
        o.append_child(text, div);
        for chap in 1..=fu::get_chapter_count(book) {
            let ch = o.create_element("chapter");
            o.set_attribute(ch, "osisID", &format!("{osis_id}.{chap}"));
            o.append_child(div, ch);
            for vers in 1..=fu::get_verse_count(book, chap) {
                let key = (book.to_string(), chap.to_string(), vers.to_string());
                let Some(&src) = index.get(&key) else {
                    continue;
                };
                let v = o.create_element("verse");
                o.set_attribute(v, "osisID", &format!("{osis_id}.{chap}.{vers}"));
                let inner = b.doc.inner_xml(src);
                let red = inner
                    .replace(
                        "<STYLE css=\"color:#ff0000\">",
                        "<q who=\"Jesus\" marker=\"\">",
                    )
                    .replace("</STYLE>", "</q>");
                // The original aborts the export when other STYLE spans leave
                // the replaced markup unbalanced; keep such verses unchanged.
                if o.set_inner_xml(v, &red).is_err() {
                    o.set_inner_xml(v, &inner)?;
                }
                o.append_child(ch, v);
            }
            if o.children(ch).is_empty() {
                o.remove_child(div, ch);
            }
        }
        if o.children(div).is_empty() {
            o.remove_child(text, div);
        }
    }

    // `PrintXML`: re-parse and indent; a parse failure yields "".
    let nl = global_memory::get().newline.clone();
    let out = Document::parse(&o.outer_xml(r))
        .map(|d| d.to_indented_string(d.root(), &nl))
        .unwrap_or_default();
    crate::io::write_utf8_bom(&filename, &out)?;
    Ok(())
}
