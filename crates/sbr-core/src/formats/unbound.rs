//! Port of `UnboundBibleFormat.cs` (The Unbound Bible, Unmapped-BCV/BCVS).

use crate::bible_format::{
    Bible, Error, ExportMeta, Result, F03_UNBOUND_BIBLE_BCV, F04_UNBOUND_BIBLE_BCVS,
};
use crate::dotnet::int_parse;
use crate::format_util as fu;
use crate::global_memory;
use sbr_xml::{Document, NodeId};

const DEFAULT_COPYRIGHT: &str = "Eternity, God";

struct State {
    root: NodeId,
    book: Option<NodeId>,
    chap: Option<NodeId>,
    book_no: i32,
    chap_no: i32,
    verse_no: i32,
    text: String,
    p_book: i32,
    p_chap: i32,
    p_verse: i32,
    format: i32,
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
        book_no: 0,
        chap_no: 0,
        verse_no: 0,
        text: String::new(),
        p_book: -1,
        p_chap: -1,
        p_verse: -1,
        format: b.filter_index,
    };
    for line in crate::io::read_lines(&text) {
        process_line(&mut d, &mut st, line)?;
    }
    b.doc = d;
    b.filter_index = st.format;
    Ok(())
}

fn parse(s: &str) -> Result<i32> {
    int_parse(s)
        .ok_or_else(|| Error::Other(format!("Input string was not in a correct format: {s:?}")))
}

fn process_line(d: &mut Document, st: &mut State, line: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(());
    }
    if line.starts_with("filetype") {
        // "BCV" is also a substring of "BCVS", so the original always picks BCV.
        if line.contains("BCV") {
            st.format = F03_UNBOUND_BIBLE_BCV;
        } else if line.contains("BCVS") {
            st.format = F04_UNBOUND_BIBLE_BCVS;
        }
    }
    let data: Vec<&str> = line.split('\t').collect();
    let cols = match data.len() {
        4 => Some((0, 3)),
        5 => Some((0, 4)),
        6 => Some((0, 5)),
        9 => Some((3, 8)),
        _ => None,
    };
    // Lines with another column count (e.g. "filetype") keep the previous
    // values and still go through the element creation below.
    if let Some((first, text)) = cols {
        st.book_no = parse(&data[first].replace(['O', 'N'], ""))?;
        st.chap_no = parse(data[first + 1])?;
        st.verse_no = parse(data[first + 2])?;
        st.text = data[text].to_string();
    }
    let (bn, cn, vn) = (st.book_no, st.chap_no, st.verse_no);
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
                st.text = crate::post_process::post_process_content(&st.text);
            }
            d.set_inner_text(e, &st.text);
        } else {
            let _ = d.set_inner_xml(e, &fu::escape_xml(&st.text));
        }
        d.set_attribute(e, "vnumber", &vn.to_string());
        if let Some(ch) = st.chap {
            d.append_child(ch, e);
        }
    }
    st.p_book = bn;
    st.p_chap = cn;
    st.p_verse = vn;
    Ok(())
}

pub fn export(b: &mut Bible, meta: &ExportMeta, filename: &str, format: i32) -> Result<()> {
    let filename = if filename.ends_with(".txt") {
        filename.to_string()
    } else {
        format!("{filename}.txt")
    };
    let nl = global_memory::get().newline.clone();
    let bcvs = format == F04_UNBOUND_BIBLE_BCVS;
    let mut out = String::new();
    let mut line = |s: &str| {
        out.push_str(s);
        out.push_str(&nl);
    };
    line("#THE UNBOUND BIBLE (www.unboundbible.org)");
    line(&format!("#name\t{}", meta.description));
    line(if bcvs {
        "filetype\tUnmapped-BCVS"
    } else {
        "filetype\tUnmapped-BCV"
    });
    line(&format!("#copyright {DEFAULT_COPYRIGHT}"));
    line(&format!("#abbreviation\t{}", meta.abbreviation));
    line("#language\t");
    line(&format!("#note\t{}", meta.comments));
    line(if bcvs {
        "#columns\torig_book_index\torig_chapter\torig_verse\torig_subverse\torder_by\ttext"
    } else {
        "#columns\torig_book_index\torig_chapter\torig_verse\torder_by\ttext"
    });
    let mut ctr = 1;
    for bk in 1..=66 {
        let testament = if bk < 40 { "O" } else { "N" };
        for c in 1..=fu::get_chapter_count(bk) {
            for v in 1..=fu::get_verse_count(bk, c) {
                let Some(n) = b.verse_node(bk, c, v) else {
                    continue;
                };
                // TODO: RTF-to-text for `CLEANING` sources (e-Sword) once RtfPipe is ported.
                let text = b.doc.inner_text(n);
                let sub = if bcvs { "\t" } else { "" };
                line(&format!(
                    "{bk:02}{testament}\t{c}\t{v}\t{sub}{ctr}0\t{text}"
                ));
                ctr += 1;
            }
        }
    }
    crate::io::write_utf8_bom(filename, &out)?;
    Ok(())
}
