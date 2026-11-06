//! Port of `VPLBibleFormat.cs` (verse-per-line text).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::dotnet::int_parse;
use crate::format_util as fu;
use crate::global_memory;
use sbr_xml::{Document, NodeId};
use std::sync::OnceLock;

struct State {
    root: NodeId,
    book: Option<NodeId>,
    chap: Option<NodeId>,
    p_book: i32,
    p_chap: i32,
    p_verse: i32,
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let text = crate::io::read_text_default(filename)?;
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
    for line in crate::io::read_lines(&text) {
        process_line(&mut d, &mut st, line);
    }
    b.doc = d;
    Ok(())
}

fn parse_line(line: &str) -> Option<(i32, i32, i32, String)> {
    let colon = line.find(':');
    if colon.is_some_and(|i| line[..i].chars().count() < 9) {
        let data: Vec<&str> = line.split(':').collect();
        let head: Vec<&str> = data[0].split(' ').collect();
        let book = fu::get_book_no(head[0]);
        if book == -1 {
            return None;
        }
        let chap = int_parse(head.get(1)?.trim())?;
        let verse = int_parse(data[1].split(' ').next()?.trim())?;
        let sp = data[1].find(' ')?;
        let mut text = data[1][sp..].trim().to_string();
        for d in &data[2..] {
            text.push(':');
            text.push_str(d);
        }
        Some((book, chap, verse, text))
    } else {
        // Crosswire style "Book.Chapter.Verse text". The original appended
        // the dot-separated fields after the verse number to the text again;
        // the port keeps the text after the first space only.
        let data: Vec<&str> = line.split('.').collect();
        if data.len() < 3 {
            return None;
        }
        let book = fu::get_book_no(data[0]);
        if book == -1 {
            return None;
        }
        let chap = int_parse(data[1])?;
        let verse = int_parse(data[2].split(' ').next()?)?;
        let sp = line.find(' ')?;
        Some((book, chap, verse, line[sp..].trim().to_string()))
    }
}

fn process_line(d: &mut Document, st: &mut State, line: &str) {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return;
    }
    let Some((bn, cn, vn, text)) = parse_line(line) else {
        return;
    };
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
            let text = if g.parse_bible_bkgnd_cleaner {
                text
            } else {
                crate::post_process::post_process_content(&text)
            };
            d.set_inner_text(e, &text);
        } else {
            // Like the original, a malformed fragment leaves the verse empty.
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

pub fn export(b: &mut Bible, _meta: &ExportMeta, filename: &str) -> Result<()> {
    let filename = if filename.ends_with(".vpl") {
        filename.to_string()
    } else {
        format!("{filename}.vpl")
    };
    let nl = global_memory::get().newline.clone();
    let mut out = String::new();
    for bk in 1..=66 {
        for c in 1..=fu::get_chapter_count(bk) {
            for v in 1..=fu::get_verse_count(bk, c) {
                if let Some(n) = b.verse_node(bk, c, v) {
                    let t = process_strong_verse(&b.doc, n, bk);
                    out.push_str(&format!(
                        "{} {c}:{v} {t}{nl}",
                        crate::localization::SHORT_BOOK_NAMES[(bk - 1) as usize]
                    ));
                }
            }
        }
    }
    crate::io::write_utf8_bom(filename, &out)?;
    Ok(())
}

fn process_strong_verse(d: &Document, n: NodeId, b: i32) -> String {
    static RE: OnceLock<fancy_regex::Regex> = OnceLock::new();
    if !global_memory::get().parse_bible {
        return d.inner_text(n);
    }
    let t = d.inner_xml(n);
    if !t.contains("<gr") {
        return t;
    }
    let re = RE.get_or_init(|| {
        fancy_regex::Regex::new(
            r#"(?i)(<gr )([^s]*)(str=['"]*)([^<'"]*)['"]*(>)(((?!</gr).)*)</gr>"#,
        )
        .unwrap()
    });
    let prefix = if b < 40 { "H" } else { "G" };
    re.replace_all(&t, format!("${{6}} <{prefix}${{4}}>").as_str())
        .into_owned()
}
