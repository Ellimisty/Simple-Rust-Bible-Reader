//! Port of the format-independent parts of `FormatUtil.cs`.

use crate::localization::{self, SHORT_BOOK_NAMES};
use sbr_xml::Document;
use std::collections::HashMap;
use std::sync::OnceLock;

pub const LOGOS_BOOK_NAMES: [&str; 66] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Rth", "1Sa", "2Sa", "1Ki", "2Ki", "1Ch",
    "2Ch", "Ezr", "Neh", "Esth", "Jb", "Psa", "Prov", "Ecc", "Song", "Isa", "Jer", "Lam", "Ezek",
    "Dan", "Hos", "Joe", "Am", "Obad", "Jon", "Mic", "Nah", "Hab", "Zeph", "Hag", "Zec", "Mal",
    "Matt", "Mrk", "Luk", "Joh", "Act", "Rom", "1Co", "2Co", "Gal", "Eph", "Phil", "Col", "1Th",
    "2Th", "1Ti", "2Ti", "Tit", "Phm", "Heb", "Jas", "1Pe", "2Pe", "1Jo", "2Jo", "3Jo", "Jud",
    "Rev",
];
pub const OLB_BOOK_NAMES: [&str; 66] = [
    "Ge", "Ex", "Le", "Nu", "De", "Jos", "Jud", "Ru", "1Sa", "2Sa", "1Ki", "2Ki", "1Ch", "2Ch",
    "Ezr", "Ne", "Es", "Job", "Ps", "Pr", "Ec", "So", "Isa", "Jer", "La", "Eze", "Da", "Ho", "Joe",
    "Am", "Ob", "Jon", "Mic", "Na", "Hab", "Zep", "Hag", "Zec", "Mal", "Mt", "Mr", "Lu", "Joh",
    "Ac", "Ro", "1Co", "2Co", "Ga", "Eph", "Php", "Col", "1Th", "2Th", "1Ti", "2Ti", "Tit", "Phm",
    "Heb", "Jas", "1Pe", "2Pe", "1Jo", "2Jo", "3Jo", "Jude", "Re",
];
/// Index matches Zefania book numbers (book 90+ are Zefania 901+).
pub const SWORD_BOOK_ABBR_IN_ZEFANIA_INDEX: [&str; 93] = [
    "Gen", "Exod", "Lev", "Num", "Deut", "Josh", "Judg", "Ruth", "1Sam", "2Sam", "1Kgs", "2Kgs",
    "1Chr", "2Chr", "Ezra", "Neh", "Esth", "Job", "Ps", "Prov", "Eccl", "Song", "Isa", "Jer",
    "Lam", "Ezek", "Dan", "Hos", "Joel", "Amos", "Obad", "Jonah", "Mic", "Nah", "Hab", "Zeph",
    "Hag", "Zech", "Mal", "Matt", "Mark", "Luke", "John", "Acts", "Rom", "1Cor", "2Cor", "Gal",
    "Eph", "Phil", "Col", "1Thess", "2Thess", "1Tim", "2Tim", "Titus", "Phlm", "Heb", "Jas",
    "1Pet", "2Pet", "1John", "2John", "3John", "Jude", "Rev", "Jdt", "Wis", "Tob", "Sir", "Bar",
    "1Macc", "2Macc", "AddDan", "AddEsth", "PrMan", "3Macc", "4Macc", "EpJer", "1Esd", "2Esd", "",
    "", "EpLao", "1En", "kGen", "Sus", "Bel", "AddPs", "PrAzar", "EsthGr", "DanGr", "Jub",
];
pub const PDB_BOOK_NUMBERS: [i32; 84] = [
    10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 190, 220, 230, 240, 250,
    260, 290, 300, 310, 330, 340, 350, 360, 370, 380, 390, 400, 410, 420, 430, 440, 450, 460, 470,
    480, 490, 500, 510, 520, 530, 540, 550, 560, 570, 580, 590, 600, 610, 620, 630, 640, 650, 660,
    670, 680, 690, 700, 710, 720, 730, 145, 146, 170, 180, 200, 210, 215, 216, 231, 235, 270, 280,
    285, 315, 320, 335, 345, 346,
];
pub const ESWORD_TO_ZEFANIA_APOCRYPHA: [(i32, i32); 8] = [
    (67, 69),
    (68, 67),
    (69, 68),
    (74, 80),
    (75, 81),
    (78, 76),
    (76, 77),
    (77, 78),
];
pub const ZEFANIA_TO_ESWORD_APOCRYPHA: [(i32, i32); 8] = [
    (69, 67),
    (67, 68),
    (68, 69),
    (80, 74),
    (81, 75),
    (76, 78),
    (77, 76),
    (78, 77),
];

/// Data built by `FormatUtil.Init()` from `bookid.xml`.
pub struct BookTables {
    pub chapter_count: Vec<i32>,
    pub verse_count: Vec<HashMap<i32, i32>>,
    pub possible_abbr: Vec<Vec<String>>,
    pub book_doc: Document,
}

pub fn tables() -> &'static BookTables {
    static T: OnceLock<BookTables> = OnceLock::new();
    T.get_or_init(|| {
        let doc = Document::parse(include_str!("../resources/bookid.xml")).expect("bookid.xml");
        let r = doc.root();
        let one = |x: String| {
            doc.select_single_node(r, &x)
                .unwrap()
                .map(|n| doc.inner_text(n))
                .unwrap_or_default()
        };
        let mut chapter_count = Vec::new();
        let mut verse_count = Vec::new();
        let mut possible_abbr = Vec::new();
        for i in 1..=66 {
            let cc: i32 = one(format!("/books/book[position()={i}]/chapters"))
                .trim()
                .parse()
                .unwrap_or(0);
            chapter_count.push(cc);
            let mut vc = HashMap::new();
            for j in 1..=cc {
                if let Ok(v) = one(format!(
                    "/books/book[position()={i}]/chapter[@n='{j}']/@verses"
                ))
                .trim()
                .parse()
                {
                    vc.insert(j, v);
                }
            }
            verse_count.push(vc);
            let mut abbr = vec![
                i.to_string(),
                one(format!("/books/book[position()={i}]/@name")),
            ];
            for n in doc
                .select_nodes(r, &format!("/books/book[position()={i}]/abbrev"))
                .unwrap()
            {
                let t = doc.inner_text(n);
                abbr.push(t.to_lowercase());
                abbr.push(t.to_uppercase());
                abbr.insert(abbr.len() - 2, t);
            }
            for lang in localization::LOCALES.iter().filter(|&&l| l != "en") {
                abbr.push(localization::book_names(lang)[i - 1].clone());
            }
            possible_abbr.push(abbr);
        }
        BookTables {
            chapter_count,
            verse_count,
            possible_abbr,
            book_doc: doc,
        }
    })
}

/// Active UI locale (`GlobalMemory.Locale`); only affects book names.
pub fn book_names() -> &'static [String] {
    localization::book_names(&crate::global_locale())
}

pub fn get_book_name(book_no: i32) -> String {
    book_names()[zefania_book_no_to_index(book_no)].clone()
}

pub fn get_book_name_short(book_no: i32) -> &'static str {
    SHORT_BOOK_NAMES[(book_no - 1) as usize]
}

/// `FormatUtil.getBookNo`: -1 when unknown.
pub fn get_book_no(book_name: &str) -> i32 {
    let want = book_name.to_lowercase();
    let t = tables();
    for (i, abbr) in t.possible_abbr.iter().enumerate() {
        if abbr.iter().any(|p| p.to_lowercase() == want) {
            return i as i32 + 1;
        }
    }
    let names = book_names();
    for book_no in 1..=names.len() as i32 {
        let idx = zefania_book_no_to_index(book_no);
        if names.get(idx).is_some_and(|n| n.to_lowercase() == want)
            || SHORT_BOOK_NAMES[(book_no - 1) as usize].to_lowercase() == want
        {
            return book_no;
        }
    }
    for book_no in 1..=names.len() as i32 {
        if names
            .get(zefania_book_no_to_index(book_no))
            .is_some_and(|n| want.contains(&n.to_lowercase()))
        {
            return book_no;
        }
    }
    -1
}

pub fn get_verse_count(book_no: i32, chapter_no: i32) -> i32 {
    tables().verse_count[(book_no - 1) as usize][&chapter_no]
}

pub fn get_chapter_count(book_no: i32) -> i32 {
    tables().chapter_count[(book_no - 1) as usize]
}

pub fn convert_pdb_bk_no_to_normal_bk_no(pdb: i32) -> Option<i32> {
    PDB_BOOK_NUMBERS
        .iter()
        .position(|&x| x == pdb)
        .map(|i| i as i32 + 1)
}

pub fn sword_book_abbr_to_zefania_book_no(abbr: &str) -> i32 {
    let n = SWORD_BOOK_ABBR_IN_ZEFANIA_INDEX
        .iter()
        .position(|&x| x == abbr)
        .map(|i| i as i32 + 1)
        .unwrap_or(0);
    if n > 89 {
        n + 811
    } else {
        n
    }
}

pub fn zefania_book_no_to_index(book_no: i32) -> usize {
    (if book_no > 900 {
        book_no - 811 - 1
    } else {
        book_no - 1
    }) as usize
}

pub fn zefania_index_to_book_no(idx: i32) -> i32 {
    if idx + 1 > 89 {
        idx + 811 + 1
    } else {
        idx + 1
    }
}

/// `FormatUtil.EscapeXML`. The replacement order is the original's, so `&`
/// produced by earlier steps is escaped again (`'` → `&amp;apos;`).
pub fn escape_xml(s: &str) -> String {
    s.replace('\'', "&apos;")
        .replace('"', "&quot;")
        .replace('>', "&gt;")
        .replace('<', "&lt;")
        .replace('&', "&amp;")
}

pub fn unescape_xml(s: &str) -> String {
    s.replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
}

pub fn get_rtf_unicode_escaped_string(s: &str) -> String {
    let mut out = String::new();
    for u in s.encode_utf16() {
        if u <= 0x7f {
            out.push(u as u8 as char);
        } else {
            out.push_str(&format!("\\u{u}?"));
        }
    }
    out
}

/// Index of the first common zlib header (`78 9C` / `78 DA`) before `till`.
pub fn find_zlib_header_idx(bytes: &[u8], till: usize) -> Option<usize> {
    (0..till.min(bytes.len().saturating_sub(1)))
        .find(|&i| bytes[i] == 0x78 && (bytes[i + 1] == 0x9C || bytes[i + 1] == 0xDA))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_order_matches_original() {
        assert_eq!(escape_xml("a'b<c&"), "a&amp;apos;b&amp;lt;c&amp;");
        assert_eq!(unescape_xml("&amp;lt;"), "&lt;");
    }

    #[test]
    fn book_lookup() {
        assert_eq!(get_chapter_count(1), 50);
        assert_eq!(get_verse_count(1, 1), 31);
        assert_eq!(get_book_no("Gen"), 1);
        assert_eq!(get_book_no("REV"), 66);
        assert_eq!(get_book_no("1 Samuel"), 9);
        assert_eq!(sword_book_abbr_to_zefania_book_no("Jub"), 904);
        assert_eq!(zefania_book_no_to_index(904), 92);
    }
}

const RTF_HEADER: &str = "{\\rtf1\\ansi\n";

/// `FormatUtil.convertHtmlToRtf` (regex method). As in the original, the
/// hyperlink colour table is never emitted.
pub fn convert_html_to_rtf(html: &str, rtf_header: Option<&str>) -> String {
    const RULES: &[(&str, &str)] = &[
        (
            r"(?i)<(?:hr)(?:\s+[^>]*)?\s*[\/]?>",
            "{\\pard \\brdrb \\brdrs \\brdrw10 \\brsp20 \\par}\n{\\pard\\par}\n",
        ),
        (r"(?i)<(?:br)(?:\s+[^>]*)?\s*[\/]?>", "{\\pard\\par}\n"),
        (
            r"(?i)<(?:p|div|section|article)(?:\s+[^>]*)?\s*[\/]>",
            "{\\pard\\par}\n",
        ),
        (r"<(?:[^>]+)\/>", ""),
        (
            r#"(?i)<a(?:\s+[^>]*)?(?:\s+href=(["'])(?:javascript:void\(0?\);?|#|return false;?|void\(0?\);?|)\1)(?:\s+[^>]*)?>"#,
            "{{{\n",
        ),
        (r"<a(?:\s+[^>]*)?>", "{{{\n"),
        (r"<\/a(?:\s+[^>]*)?>", "\n}}}"),
        (r"<(?:b|strong)(?:\s+[^>]*)?>", "{\\b\n"),
        (r"<(?:i|em)(?:\s+[^>]*)?>", "{\\i\n"),
        (r"<(?:u|ins)(?:\s+[^>]*)?>", "{\\ul\n"),
        (r"<(?:strike|del)(?:\s+[^>]*)?>", "{\\strike\n"),
        (r"<sup(?:\s+[^>]*)?>", "{\\super\n"),
        (r"<sub(?:\s+[^>]*)?>", "{\\sub\n"),
        (r"<(?:p|div|section|article)(?:\s+[^>]*)?>", "{\\pard\n"),
        (r"<h[1-6](?:\s+[^>]*)?>", "{\\b\n"),
        (r"<\/(?:h[1-6])(?:\s+[^>]*)?>", "\n}"),
        (r"<br[\/]>", "\\line\n"),
        (r"<\/(?:p|div|section|article)(?:\s+[^>]*)?>", "\n\\par}\n"),
        (
            r"<\/(?:b|strong|i|em|u|ins|strike|del|sup|sub)(?:\s+[^>]*)?>",
            "\n}",
        ),
        (r"<(?:[^>]+)>", ""),
    ];
    let mut text = html.to_string();
    for (pat, rep) in RULES {
        let re = fancy_regex::Regex::new(pat).expect("valid pattern");
        text = re
            .replace_all(&text, fancy_regex::NoExpand(rep))
            .into_owned();
    }
    format!("{}{}\n}}", rtf_header.unwrap_or(RTF_HEADER), text)
}

/// `FormatUtil.convertRtfToHtml`: RtfPipe, falling back to plain text.
pub fn convert_rtf_to_html(rtf: &str) -> String {
    if rtf.trim().is_empty()
        || !(rtf.contains("{\\") || rtf.starts_with('\\') || rtf.contains("\\cf"))
    {
        return rtf.to_string();
    }
    let full = if rtf.starts_with("{\\rtf1") {
        rtf.to_string()
    } else {
        format!("{RTF_HEADER}{rtf}")
    };
    sbr_rtf::to_html(&full)
        .or_else(|_| sbr_rtf::to_text(&full))
        .unwrap_or_else(|_| rtf.to_string())
}

#[cfg(test)]
mod rtf_tests {
    use super::convert_html_to_rtf;

    #[test]
    fn html_to_rtf_matches_original() {
        // Captured from FormatUtil.convertHtmlToRtf under Mono.
        let cases = [
            ("<b>x</b><br/>y", "{\\rtf1\\ansi\n{\\b\nx\n}{\\pard\\par}\ny\n}"),
            ("<P>a<i>b</I></p><hr>", "{\\rtf1\\ansi\na{\\i\nb\n\\par}\n{\\pard \\brdrb \\brdrs \\brdrw10 \\brsp20 \\par}\n{\\pard\\par}\n\n}"),
            ("<a href=\"#\">l</a> <a href='x'>m</a>", "{\\rtf1\\ansi\n{{{\nl\n}}} {{{\nm\n}}}\n}"),
            ("<h2 class=\"t\">H</h2><sup>1</sup><span>s</span><br/>", "{\\rtf1\\ansi\n{\\b\nH\n}{\\super\n1\n}s{\\pard\\par}\n\n}"),
            ("<div/><img src='a'/>&amp;", "{\\rtf1\\ansi\n{\\pard\\par}\n&amp;\n}"),
        ];
        for (html, want) in cases {
            assert_eq!(convert_html_to_rtf(html, None), want, "{html}");
        }
    }
}
