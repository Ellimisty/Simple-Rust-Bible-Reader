//! Port of `BBLXBibleFormat.cs` (`.bblx`). e-Sword 9+ bible module:
//! SQLite db with `Bible(Book, Chapter, Verse, Scripture)` and a
//! `Details` metadata table. Scripture may be TEXT or a BLOB —
//! zlib-compressed, or TwoFish-encrypted (unimplemented: such modules
//! read as empty verses). Book ids pass through
//! eSword2Zefania_Apocrypha_Map; all books are kept.
//! Verse text is stored verbatim (InnerText after postProcessContent);
//! markup like <W430> is NOT expanded to <gr> on load.

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::dotnet::int_parse;
use crate::format_util as fu;
use crate::global_memory;
use rusqlite::types::ValueRef;
use sbr_xml::{Document, NodeId};
use std::collections::HashMap;
use std::io::Read;
use std::sync::OnceLock;

/// e-Sword book number -> Zefania bnumber (apocrypha).
fn esword_to_zefania(b: i32) -> i32 {
    static M: OnceLock<HashMap<i32, i32>> = OnceLock::new();
    let m = M.get_or_init(|| {
        HashMap::from([(67, 69), (68, 67), (69, 68), (74, 80), (75, 81),
                       (78, 76), (76, 77), (77, 78)])
    });
    *m.get(&b).unwrap_or(&b)
}

/// Zefania bnumber -> e-Sword book number (apocrypha).
fn zefania_to_esword(b: i32) -> i32 {
    static M: OnceLock<HashMap<i32, i32>> = OnceLock::new();
    let m = M.get_or_init(|| {
        HashMap::from([(69, 67), (67, 68), (68, 69), (80, 74), (81, 75),
                       (76, 78), (77, 76), (78, 77)])
    });
    *m.get(&b).unwrap_or(&b)
}

/// Scripture cell -> text: plain string, zlib'd blob, or (unsupported)
/// TwoFish-encrypted blob -> empty string.
fn cell_text(v: ValueRef) -> String {
    match v {
        ValueRef::Null => String::new(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => {
            let zi = fu::find_zlib_header_idx(b, 0xB);
            match zi {
                Some(i) => {
                    let mut out = String::new();
                    flate2::read::ZlibDecoder::new(&b[i..])
                        .read_to_string(&mut out)
                        .map(|_| out)
                        .unwrap_or_default()
                }
                // TwoFish-encrypted modules are not supported yet.
                None => String::new(),
            }
        }
    }
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);

    // Errors end the load silently, keeping what was read, like the original.
    let rows = (|| -> rusqlite::Result<Vec<[String; 4]>> {
        let cnn = rusqlite::Connection::open_with_flags(
            filename,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut st = cnn.prepare(
            "SELECT Book, Chapter, Verse, Scripture FROM Bible ORDER BY Book, Chapter, Verse",
        )?;
        let mut rows = st.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push([
                row.get_ref(0).map(|v| match v {
                    ValueRef::Integer(i) => i.to_string(),
                    x => cell_text(x),
                }).unwrap_or_default(),
                row.get_ref(1).map(|v| match v {
                    ValueRef::Integer(i) => i.to_string(),
                    x => cell_text(x),
                }).unwrap_or_default(),
                row.get_ref(2).map(|v| match v {
                    ValueRef::Integer(i) => i.to_string(),
                    x => cell_text(x),
                }).unwrap_or_default(),
                row.get_ref(3).map(cell_text).unwrap_or_default(),
            ]);
        }
        Ok(out)
    })()
    .unwrap_or_default();

    let mut st = State {
        root,
        book: None,
        chap: None,
        p_book: -1,
        p_chap: -1,
        p_verse: -1,
    };
    for [bk, ch, vs, text] in rows {
        let (Some(bno), Some(c), Some(v)) =
            (int_parse(&bk), int_parse(&ch), int_parse(&vs))
        else {
            break;
        };
        process_line(&mut d, &mut st, esword_to_zefania(bno), c, v, &text);
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

fn process_line(d: &mut Document, st: &mut State, bn: i32, cn: i32, vn: i32, text: &str) {
    let restart = cn == 1 && vn == 1 && st.p_verse >= vn;
    if bn != st.p_book || restart {
        let r = d.root();
        st.book = d
            .select_single_node(r, &format!("/XMLBIBLE/BIBLEBOOK[@bnumber='{bn}']"))
            .ok()
            .flatten();
        if st.book.is_none() {
            let e = d.create_element("BIBLEBOOK");
            d.set_attribute(e, "bnumber", &bn.to_string());
            d.append_child(st.root, e);
            st.book = Some(e);
        }
    }
    if cn != st.p_chap || bn != st.p_book || restart {
        let bk = st.book.expect("book");
        st.chap = d
            .select_single_node(bk, &format!("CHAPTER[@cnumber='{cn}']"))
            .ok()
            .flatten();
        if st.chap.is_none() {
            let e = d.create_element("CHAPTER");
            d.set_attribute(e, "cnumber", &cn.to_string());
            d.append_child(bk, e);
            st.chap = Some(e);
        }
    }
    if vn != st.p_verse || cn != st.p_chap || bn != st.p_book {
        let e = d.create_element("VERS");
        let g = global_memory::get();
        if g.parse_bible {
            // original: ConvertRtfToHtmlBible side-flag, then
            // postProcessContent + InnerText (text kept verbatim)
            let t = if !g.parse_bible_bkgnd_cleaner {
                crate::post_process::post_process_content(text)
            } else {
                text.to_string()
            };
            d.set_inner_text(e, &t);
        } else {
            let _ = d.set_inner_xml(e, &fu::escape_xml(text));
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

/// Original `processExportVerse`: markup -> RTF control words.
fn process_export_verse(d: &Document, n: NodeId, b: i32) -> String {
    if !global_memory::get().parse_bible {
        return d.inner_text(n);
    }
    let mut t = d.inner_xml(n);
    if t.contains("<STYLE") {
        t = t
            .replace("<STYLE css=\"color:#ff0000\">", "{\\cf6 ")
            .replace("</STYLE>", "}");
    }
    if t.contains("<i>") {
        t = t.replace("<i>", "{\\cf15\\i ").replace("</i>", "}");
    }
    if t.contains("<gr") {
        static GR: OnceLock<fancy_regex::Regex> = OnceLock::new();
        let prefix = if b < 40 { "H" } else { "G" };
        let gr = GR.get_or_init(|| fancy_regex::Regex::new(
            r#"(<gr )([^s]*)(str=['"]*)([^<'"]*)['"]*(>)(((?!</gr).)*)</gr>"#,
        ).unwrap());
        t = gr
            .replace_all(&t, format!("${{6}}{{\\cf11\\super {prefix}${{4}}}}"))
            .into_owned();
    }
    t
}

pub fn export(b: &mut Bible, meta: &ExportMeta, filename: &str, _filter_idx: i32) -> Result<()> {
    if std::path::Path::new(filename).exists() {
        std::fs::remove_file(filename)?;
    }
    let sql = |e: rusqlite::Error| crate::bible_format::Error::Other(format!("sqlite: {e}"));
    let mut cnn = rusqlite::Connection::open(filename).map_err(sql)?;
    cnn.execute_batch(
        "CREATE TABLE 'Bible' (Book INT, Chapter INT, Verse INT, Scripture TEXT);\
         CREATE TABLE 'Details' (Description NVARCHAR(250), Abbreviation NVARCHAR(50), Comments TEXT, Version INT, Font TEXT, RightToLeft BOOL, OT BOOL, NT BOOL, Apocrypha BOOL, Strong BOOL);\
         CREATE INDEX BookChapterVerseIndex ON Bible (Book, Chapter, Verse);",
    )
    .map_err(sql)?;
    let dir = global_memory::get().direction;
    cnn.execute_batch(&format!(
        "INSERT INTO Details VALUES('{}','{}','{}',1,'DEFAULT',{dir},1,1,0,0)",
        meta.description, meta.abbreviation, meta.comments
    ))
    .map_err(sql)?;
    let tx = cnn.transaction().map_err(sql)?;
    {
        let mut ins = tx
            .prepare("INSERT INTO Bible (Book, Chapter, Verse, Scripture) VALUES (?1, ?2, ?3, ?4)")
            .map_err(sql)?;
        // original iterates the book nodes present in the doc
        let sel = dsel_books(&b.doc);
        for b_ in sel {
            let book_no = zefania_to_esword(b_);
            for c in 1..=fu::get_chapter_count(b_) {
                for v in 1..=fu::get_verse_count(b_, c) {
                    if let Some(n) = b.verse_node(b_, c, v) {
                        let t = fu::get_rtf_unicode_escaped_string(
                            &process_export_verse(&b.doc, n, b_));
                        ins.execute(rusqlite::params![
                            book_no.to_string(),
                            c.to_string(),
                            v.to_string(),
                            t
                        ])
                        .map_err(sql)?;
                    }
                }
            }
        }
    }
    tx.commit().map_err(sql)?;
    Ok(())
}

/// bnumber of each /XMLBIBLE/BIBLEBOOK node, in document order.
fn dsel_books(d: &Document) -> Vec<i32> {
    d.select_nodes(d.root(), "/XMLBIBLE/BIBLEBOOK")
        .unwrap_or_default()
        .into_iter()
        .filter_map(|n| int_parse(&d.get_attribute(n, "bnumber")))
        .collect()
}
