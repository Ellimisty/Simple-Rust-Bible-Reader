//! Port of the reading view: `BibleFormat.getChapterHtml`, `getRawChapter`,
//! `getBookName`, `populateBooks` and the CSS lookup in `Themes.cs`.

use crate::bible_format::{Bible, Error, Result};
use crate::{chapter_heading, format_util as fu, global_memory, localization};
use std::collections::BTreeMap;

/// `BibleFormat.rtf_header`, prefixed to e-Sword verses before RTF→text.
const RTF_HEADER: &str = r"{\rtf1\ansi\ansicpg1252\deff0\deflang1033{\fonttbl{\f1\fnil\fprq2\fcharset161 TITUS cyberbit Basic;}{\f2\fnil\fprq2\fcharset177 TITUS Cyberbit Basic;}{\f3\fnil\fprq2\fcharset0 TITUS Cyberbit Basic;}{\f4\fnil\fprq2\fcharset136 PMingLiU;}{\f5\fnil\fprq2\fcharset204 Georgia;}{\f6\fnil\fprq2\fcharset238 Georgia;}{\f7\fnil\fprq2\fcharset134 PMingLiU;}{\f8\fnil\fprq2\fcharset222 Cordia New;}{\f9\fnil\fprq2\fcharset129 Gulim;}{\f10\fnil\fprq2\fcharset178 TITUS Cyberbit Basic;}{\f11\fnil\fprq2\fcharset162 Georgia;}{\f12\fnil\fprq2\fcharset163 Times New Roman;}{\f13\fnil\fprq2\fcharset128 MS Mincho;}}{\colortbl ;\red0\green0\blue0;\red0\green0\blue255;\red0\green255\blue255;\red0\green255\blue0;\red255\green0\blue255;\red255\green0\blue0;\red255\green255\blue0;\red255\green255\blue255;\red0\green0\blue128;\red0\green128\blue128;\red0\green128\blue0;\red128\green0\blue128;\red128\green0\blue0;\red128\green128\blue0;\red128\green128\blue128;\red192\green192\blue192;}{\viewkind4\uc1\pard\plain\cf0\fs\par}";

type ThemeFiles = &'static [(&'static str, &'static str)];

const THEMES: &[(&str, ThemeFiles)] = &[
    (
        "Dark",
        &[
            (
                "Commentary.css",
                include_str!("../resources/themes/Dark/Commentary.css"),
            ),
            (
                "Dictionary.css",
                include_str!("../resources/themes/Dark/Dictionary.css"),
            ),
            (
                "Main.css",
                include_str!("../resources/themes/Dark/Main.css"),
            ),
            (
                "Search.css",
                include_str!("../resources/themes/Dark/Search.css"),
            ),
            ("UI.css", include_str!("../resources/themes/Dark/UI.css")),
        ],
    ),
    (
        "Default",
        &[
            (
                "Commentary.css",
                include_str!("../resources/themes/Default/Commentary.css"),
            ),
            (
                "Dictionary.css",
                include_str!("../resources/themes/Default/Dictionary.css"),
            ),
            (
                "Main.css",
                include_str!("../resources/themes/Default/Main.css"),
            ),
            (
                "Search.css",
                include_str!("../resources/themes/Default/Search.css"),
            ),
            ("UI.css", include_str!("../resources/themes/Default/UI.css")),
        ],
    ),
];

#[derive(Debug, Clone, Copy)]
pub enum ThemeCss {
    Ui,
    Main,
    Commentary,
    Dictionary,
    Search,
}

/// `Themes.getThemes`.
pub fn themes() -> Vec<&'static str> {
    THEMES.iter().map(|(n, _)| *n).collect()
}

/// `Themes.getCSS`.
pub fn get_css(theme: &str, css: ThemeCss) -> String {
    let file = match css {
        ThemeCss::Ui => "UI.css",
        ThemeCss::Main => "Main.css",
        ThemeCss::Commentary => "Commentary.css",
        ThemeCss::Dictionary => "Dictionary.css",
        ThemeCss::Search => "Search.css",
    };
    let mut s = String::new();
    if let Some((_, files)) = THEMES.iter().find(|(n, _)| *n == theme) {
        if let Some((_, c)) = files.iter().find(|(f, _)| *f == file) {
            let g = global_memory::get();
            s = c
                .replace("@FontFamilyBIB@", &g.font_family_bib)
                .replace("@FontFamilyCMT@", &g.font_family_cmt)
                .replace("@FontFamilyDICT@", &g.font_family_cmt);
        }
    }
    format!("<style>{s}</style>")
}

/// `Themes.CSS_*` for the selected theme.
pub fn css(kind: ThemeCss) -> String {
    let theme = global_memory::get().selected_theme.clone();
    get_css(&theme, kind)
}

/// `BibleFormat.getBookName`.
pub fn book_name(b: &Bible, book: i32) -> String {
    let x = format!("/XMLBIBLE/BIBLEBOOK[@bnumber='{book}']/@bname");
    match b.doc.select_single_node(b.doc.root(), &x).ok().flatten() {
        Some(n) => b.doc.inner_text(n),
        None => fu::book_names()[fu::zefania_book_no_to_index(book)].clone(),
    }
}

/// A book name with its chapter numbers.
pub type BookEntry = (String, Vec<i32>);

/// The book tree built by `populateBooks`: (name, chapter numbers) per group.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct BookTree {
    pub old_testament: Vec<BookEntry>,
    pub new_testament: Vec<BookEntry>,
    pub apocrypha: Vec<BookEntry>,
}

impl BookTree {
    /// Group captions as shown in the tree view, for non-empty groups.
    pub fn groups(&self) -> Vec<(String, &[BookEntry])> {
        let lang = global_memory::get().locale.clone();
        [
            ("Old_Testament", &self.old_testament),
            ("New_Testament", &self.new_testament),
            ("Apocrypha", &self.apocrypha),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (localization::get(&lang, k), v.as_slice()))
        .collect()
    }
}

/// `BibleFormat.populateBooks`: adds missing `bname` attributes and lists
/// the books by testament in book-number order.
pub fn populate_books(b: &mut Bible) -> Result<BookTree> {
    let r = b.doc.root();
    let mut groups: [BTreeMap<i32, String>; 3] = Default::default();
    let mut chapters: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    for node in b.doc.select_nodes(r, "/XMLBIBLE/BIBLEBOOK")? {
        let Some(book) = b
            .doc
            .attr(node, "bnumber")
            .and_then(crate::dotnet::int_parse)
        else {
            continue;
        };
        let g = match book {
            1..=39 => 0,
            40..=66 => 1,
            67.. => 2,
            _ => continue,
        };
        if !b.doc.has_attribute(node, "bname") {
            let name = fu::book_names()[fu::zefania_book_no_to_index(book)].clone();
            b.doc.set_attribute(node, "bname", &name);
        }
        let name = b.doc.get_attribute(node, "bname");
        groups[g].insert(book, name.clone());
        if let std::collections::btree_map::Entry::Vacant(e) = chapters.entry(name) {
            let mut list = Vec::new();
            for ch in b.doc.select_nodes(node, "CHAPTER")? {
                let c = b.doc.get_attribute(ch, "cnumber");
                list.push(
                    crate::dotnet::int_parse(&c)
                        .ok_or_else(|| Error::Other(format!("bad cnumber {c:?}")))?,
                );
            }
            e.insert(list);
        }
    }
    let list = |m: &BTreeMap<i32, String>| {
        m.values()
            .filter(|n| !n.is_empty())
            .map(|n| (n.clone(), chapters[n].clone()))
            .collect::<Vec<_>>()
    };
    Ok(BookTree {
        old_testament: list(&groups[0]),
        new_testament: list(&groups[1]),
        apocrypha: list(&groups[2]),
    })
}

fn replace(s: &str, pat: &str, rep: &str) -> String {
    fancy_regex::Regex::new(pat)
        .expect("valid pattern")
        .replace_all(s, rep)
        .into_owned()
}

/// `BibleFormat.getRawChapter`.
pub fn raw_chapter_html(b: &Bible, book_name: &str, chapter: &str) -> Result<String> {
    let mut html = format!("<html dir='ltr'><head>{}</head><body>", css(ThemeCss::Main));
    let x = format!("/XMLBIBLE/BIBLEBOOK[@bname='{book_name}']/CHAPTER[@cnumber={chapter}]/VERS");
    for v in b.doc.select_nodes(b.doc.root(), &x)? {
        html += &format!(
            "<i>{}</i><br/><textarea readonly='readonly'>{}</textarea><br/>",
            b.doc.get_attribute(v, "vnumber"),
            fu::unescape_xml(&fu::unescape_xml(&b.doc.inner_xml(v)))
        );
    }
    html += "</body></html>";
    Ok(html)
}

/// `BibleFormat.getChapterHtml` with the WebView2 renderer. `has_commentary`
/// stands in for `cmt.getCommentaryForVerse(..) != ""` when a commentary is open.
pub fn chapter_html(
    b: &mut Bible,
    book_name: &str,
    chapter: &str,
    para_mode: bool,
    has_commentary: Option<&dyn Fn(i32, i32, i32) -> bool>,
) -> Result<String> {
    if global_memory::get().display_bib_raw {
        return raw_chapter_html(b, book_name, chapter);
    }
    let (parse_bible, commentary_icon) = {
        let g = global_memory::get();
        (g.parse_bible, g.commentary_icon)
    };
    let mut html = format!(
        "<html dir='ltr'><head><title>{}</title>{}</head><body>",
        b.description,
        css(ThemeCss::Main)
    );
    let r = b.doc.root();
    let verses = b.doc.select_nodes(
        r,
        &format!("/XMLBIBLE/BIBLEBOOK[@bname='{book_name}']/CHAPTER[@cnumber={chapter}]/VERS"),
    )?;
    let book_node = b
        .doc
        .select_single_node(r, &format!("/XMLBIBLE/BIBLEBOOK[@bname='{book_name}']"))?
        .ok_or_else(|| Error::Other(format!("no book named {book_name:?}")))?;
    let book_no = b.doc.get_attribute(book_node, "bnumber");
    let mchap = crate::dotnet::int_parse(chapter).unwrap_or(0);
    let mbook = crate::dotnet::int_parse(&book_no).unwrap_or(0);
    html += &format!(
        "<h3>{}</h3>",
        chapter_heading::chapter_title(mbook, mchap).unwrap_or("")
    );
    if !para_mode {
        html += "<ul>";
    }
    let para_style = if para_mode { "para" } else { "linear" };
    for verse in verses {
        let verse_no = b.doc.get_attribute(verse, "vnumber");
        let book = b
            .doc
            .parent(verse)
            .and_then(|c| b.doc.parent(c))
            .and_then(|bk| b.doc.attr(bk, "bnumber").and_then(crate::dotnet::int_parse))
            .unwrap_or(mbook);
        let prefix = if book >= 40 { "G" } else { "H" };
        let mut text = if b.cleaning && parse_bible {
            let rtf = format!("{RTF_HEADER}{}", b.doc.inner_text(verse));
            sbr_rtf::to_text(&rtf).unwrap_or_else(|_| b.doc.inner_text(verse))
        } else if parse_bible {
            let mut t = b.doc.inner_xml(verse);
            if t.to_lowercase().contains("style") {
                t = replace(
                    &t,
                    "(?i)(<)(STYLE )([^<]*)(css=)([^<]*)(>)",
                    "<span style=${5}>",
                );
                t = replace(&t, "(?i)</STYLE>", "</span>");
            }
            if t.to_lowercase().contains("gr") {
                t = replace(
                    &t,
                    r#"(?i)(<gr )([^s]*)(str=['"]*)([^<'"]*)['"]*(>)(((?!</gr).)*)</gr>"#,
                    &format!(
                        "${{6}}<sup><span onclick=\"chrome.webview.postMessage('L{prefix}${{4}}');\" class='lexicon'>{prefix}${{4}}</span></sup>"
                    ),
                );
            }
            t
        } else {
            b.doc.inner_text(verse)
        };
        text = replace(&text, "<NOTE>[^<]+</NOTE>", "");
        let id = format!("{book_no}-{chapter}-{verse_no}");
        let mut v = format!(
            "<a class=\"{para_style}\" id='{verse_no}' OnClick=\"chrome.webview.postMessage('{id}');return false;\" OnMouseDown=\"if(event.button == 2)chrome.webview.postMessage('{id}');\"><sup class='versenumber'>{verse_no}</sup>{text}</a> "
        );
        if !para_mode {
            v = format!("<li>{v}");
        }
        if let Some(has) = has_commentary {
            let c = |s: &str| crate::dotnet::int_parse(s).unwrap_or(0);
            if has(c(&book_no), c(chapter), c(&verse_no)) && commentary_icon {
                v += " <img id='cmtimage' border='0'/>";
            }
        }
        v += "</a>";
        // Old-style COMMENT siblings are converted to NOTE children.
        if let Some(sib) = b.doc.next_sibling(verse) {
            if b.doc.name(sib) == "COMMENT" {
                let note = b.doc.create_element("NOTE");
                let t = b.doc.inner_text(sib);
                b.doc.set_inner_text(note, &t);
                b.doc.remove_all(sib);
                b.doc.append_child(verse, note);
            }
        }
        if b.doc.has_child_nodes(verse) {
            if let Some(note) = b.doc.select_single_node(verse, "NOTE")? {
                v += &format!("<BLOCKQUOTE>{}</BLOCKQUOTE>", b.doc.inner_text(note));
            }
        }
        if !para_mode {
            v += "</li>";
        }
        html += &v;
    }
    if !para_mode {
        html += "</ul>";
    }
    html += "</body></html>";
    Ok(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_chapter() {
        let mut b = Bible::new(1);
        b.doc
            .load_xml(r#"<XMLBIBLE><BIBLEBOOK bnumber="8"><CHAPTER cnumber="1"><VERS vnumber="1">In the <gr str="3117">days</gr> <STYLE css="color:#ff0000">x</STYLE><NOTE>n</NOTE></VERS></CHAPTER></BIBLEBOOK><BIBLEBOOK bnumber="65"><CHAPTER cnumber="1"/></BIBLEBOOK></XMLBIBLE>"#)
            .unwrap();
        let tree = populate_books(&mut b).unwrap();
        assert_eq!(tree.old_testament, vec![("Ruth".to_string(), vec![1])]);
        assert_eq!(tree.new_testament, vec![("Jude".to_string(), vec![1])]);
        let html = chapter_html(&mut b, "Ruth", "1", false, None).unwrap();
        assert!(html.contains("<h3>"), "{html}");
        assert!(html.contains("<li><a class=\"linear\" id='1' OnClick=\"chrome.webview.postMessage('8-1-1');return false;\""), "{html}");
        assert!(html.contains("In the days<sup><span onclick=\"chrome.webview.postMessage('LH3117');\" class='lexicon'>H3117</span></sup><span style=\"color:#ff0000\">x</span></a> </a><BLOCKQUOTE>n</BLOCKQUOTE></li></ul></body></html>"), "{html}");
    }
}
