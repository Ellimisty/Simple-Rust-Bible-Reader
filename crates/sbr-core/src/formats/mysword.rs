//! Port of `MySword4AndroidBibleFormat.cs` (`.bbl.mybible`, `.bbl.mybible.gz`).

use crate::bible_format::{Bible, ExportMeta, Result};
use crate::dotnet::int_parse;
use crate::format_util as fu;
use crate::global_memory;
use fancy_regex::Regex;
use rusqlite::types::ValueRef;
use sbr_xml::{Document, NodeId};
use std::io::Read;
use std::sync::OnceLock;

/// Export settings from `getProperties()` with their original defaults.
pub struct Properties {
    pub version: String,
    pub font: String,
    pub ot: String,
    pub nt: String,
    pub apocrypha: String,
    pub strong: String,
}

impl Default for Properties {
    fn default() -> Self {
        Self {
            version: "1".into(),
            font: "DEFAULT".into(),
            ot: "1".into(),
            nt: "1".into(),
            apocrypha: "0".into(),
            strong: "0".into(),
        }
    }
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).unwrap())
}

/// `DataRow[i].ToString()` for the SQLite value kinds.
fn cell_string(v: ValueRef) -> String {
    match v {
        ValueRef::Null => String::new(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) | ValueRef::Blob(t) => String::from_utf8_lossy(t).into_owned(),
    }
}

struct TempFile(std::path::PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    let mut d = Document::new();
    let r = d.root();
    let dec = d.create_xml_declaration("1.0", None, None);
    d.append_child(r, dec);
    let root = d.create_element("XMLBIBLE");
    d.append_child(r, root);

    let mut _tmp = None;
    let mut db_path = std::path::PathBuf::from(filename);
    if filename.to_lowercase().ends_with(".gz") {
        let mut raw = Vec::new();
        flate2::read::GzDecoder::new(std::fs::File::open(filename)?).read_to_end(&mut raw)?;
        db_path = std::env::temp_dir().join(format!(
            "sbr_mysword_{}_{:p}.bbl.mybible",
            std::process::id(),
            &raw
        ));
        std::fs::write(&db_path, &raw)?;
        _tmp = Some(TempFile(db_path.clone()));
    }

    // Errors end the load silently, keeping what was read, like the original.
    let rows = (|| -> rusqlite::Result<Vec<[String; 4]>> {
        let cnn = rusqlite::Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut st = cnn.prepare(
            "SELECT Book, Chapter, Verse, Scripture FROM Bible ORDER BY Book, Chapter, Verse",
        )?;
        let mut rows = st.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push([0, 1, 2, 3].map(|i| row.get_ref(i).map(cell_string).unwrap_or_default()));
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
        let Some(bno) = int_parse(&bk) else { break };
        if bno > 66 {
            break;
        }
        let (Some(c), Some(v)) = (int_parse(&ch), int_parse(&vs)) else {
            break;
        };
        process_line(&mut d, &mut st, bno, c, v, &text);
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
    if cn != st.p_chap || restart {
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
        if global_memory::get().parse_bible {
            if set_verse(d, e, text).is_err() {
                d.set_inner_text(e, text);
            }
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

/// The private `processLine(XmlElement, string)`: GBF-style tags to Zefania.
fn set_verse(
    d: &mut Document,
    e: NodeId,
    line: &str,
) -> std::result::Result<(), sbr_xml::ParseError> {
    static RF: OnceLock<Regex> = OnceLock::new();
    static PAIR: OnceLock<Regex> = OnceLock::new();
    static TAG2: OnceLock<Regex> = OnceLock::new();
    static W1: OnceLock<Regex> = OnceLock::new();
    static W2: OnceLock<Regex> = OnceLock::new();
    static ANY: OnceLock<Regex> = OnceLock::new();
    let mut line = line.to_string();
    let g = global_memory::get().clone();
    if g.parse_bible {
        if line.contains("<RF") {
            line = re(&RF, r"\<RF\>[^<]+\<Rf\>")
                .replace_all(&line, "")
                .into_owned();
        }
        if line.contains("<FR>") {
            line = line
                .replace("<FR>", "<STYLE css=\"color:#ff0000\">")
                .replace("<Fr>", "</STYLE>");
        }
        line = re(&PAIR, "<[A-Z][A-Z]>([^<]*)<[A-Z][a-z]>")
            .replace_all(&line, "${1}")
            .into_owned();
        line = re(&TAG2, "<[A-Z]([A-Z]|[a-z])>")
            .replace_all(&line, "")
            .into_owned();
        if line.contains("<W") {
            line = re(&W1, "(?i)( [^<]*)(<W[H|G])([0-9.xs*]*)(>)")
                .replace_all(&line, "<gr str=\"${3}\">${1}</gr>")
                .into_owned();
            line = re(&W2, "(?i)(<W[H|G])([0-9.xs*]*)(>)")
                .replace_all(&line, "<gr str=\"${2}\">&nbsp;</gr>")
                .into_owned();
        }
        if line.contains("<X") || line.contains("<x") {
            line = line.replace("<X>", "").replace("<x>", "");
        }
        line = re(&ANY, r"<[A-Za-z0-9\.]+>")
            .replace_all(&line, "")
            .into_owned();
        if !g.parse_bible_bkgnd_cleaner {
            line = crate::post_process::post_process_content(&line);
        }
        d.set_inner_xml(e, &line)
    } else {
        d.set_inner_text(e, &fu::escape_xml(&line));
        Ok(())
    }
}

fn process_export_verse(d: &Document, n: NodeId, b: i32) -> String {
    static GR: OnceLock<Regex> = OnceLock::new();
    if !global_memory::get().parse_bible {
        return d.inner_text(n);
    }
    let mut t = d.inner_xml(n);
    if t.contains("<STYLE") {
        t = t
            .replace("<STYLE css=\"color:#ff0000\">", "<FR>")
            .replace("</STYLE>", "<Fr>");
    }
    if t.contains("<gr") {
        let prefix = if b < 40 { "H" } else { "G" };
        let gr = re(
            &GR,
            r#"(<gr )([^s]*)(str=['"]*)([^<'"]*)['"]*(>)(((?!</gr).)*)</gr>"#,
        );
        t = gr
            .replace_all(&t, format!("${{6}}<W{prefix}${{4}}>").as_str())
            .into_owned();
    }
    t
}

pub fn export(b: &mut Bible, meta: &ExportMeta, filename: &str) -> Result<()> {
    export_with(b, meta, &Properties::default(), filename)
}

/// Like the original, the database is written to the name without `.gz`
/// and is left uncompressed.
pub fn export_with(b: &mut Bible, meta: &ExportMeta, p: &Properties, filename: &str) -> Result<()> {
    let filename = if filename.ends_with(".bbl.mybible.gz") {
        filename.to_string()
    } else {
        format!("{filename}.bbl.mybible.gz")
    };
    let tmp = &filename[..filename.len() - 3];
    if std::path::Path::new(tmp).exists() {
        std::fs::remove_file(tmp)?;
    }
    let sql = |e: rusqlite::Error| crate::bible_format::Error::Other(format!("sqlite: {e}"));
    let mut cnn = rusqlite::Connection::open(tmp).map_err(sql)?;
    cnn.execute_batch(
        "CREATE TABLE 'Bible' (Book INT, Chapter INT, Verse INT, Scripture TEXT);\
         CREATE TABLE 'Details' (Description NVARCHAR(250), Abbreviation NVARCHAR(50), Comments TEXT, Version INT, Font TEXT, RightToLeft BOOL, OT BOOL, NT BOOL, Apocrypha BOOL, Strong BOOL);\
         CREATE INDEX BookChapterVerseIndex ON Bible (Book, Chapter, Verse);",
    )
    .map_err(sql)?;
    let dir = global_memory::get().direction;
    cnn.execute_batch(&format!(
        "INSERT INTO Details VALUES('{}','{}','{}',{},'{}',{dir},{},{},{},{})",
        meta.description,
        meta.abbreviation,
        meta.comments,
        p.version,
        p.font,
        p.ot,
        p.nt,
        p.apocrypha,
        p.strong
    ))
    .map_err(sql)?;
    let tx = cnn.transaction().map_err(sql)?;
    {
        let mut ins = tx
            .prepare("INSERT INTO Bible (Book, Chapter, Verse, Scripture) VALUES (?1, ?2, ?3, ?4)")
            .map_err(sql)?;
        for bk in 1..=66 {
            for c in 1..=fu::get_chapter_count(bk) {
                for v in 1..=fu::get_verse_count(bk, c) {
                    if let Some(n) = b.verse_node(bk, c, v) {
                        let t = fu::unescape_xml(&process_export_verse(&b.doc, n, bk));
                        ins.execute(rusqlite::params![
                            bk.to_string(),
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
