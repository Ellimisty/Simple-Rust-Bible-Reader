//! Serialization matching `XmlNode.OuterXml` / `XmlDocument.Save`.

use crate::{Document, Kind, NodeId, XMLNS_NS, XML_NS};

fn char_ref(c: char, out: &mut String) {
    out.push_str(&format!("&#x{:X};", c as u32));
}

pub(crate) fn escape_text_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => char_ref(c, out),
            c => out.push(c),
        }
    }
}

pub(crate) fn escape_attr_into(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push('\t'),
            '\n' | '\r' => char_ref(c, out),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => char_ref(c, out),
            c => out.push(c),
        }
    }
}

pub(crate) fn decl_value(k: &Kind) -> String {
    let Kind::XmlDecl {
        version,
        encoding,
        standalone,
    } = k
    else {
        return String::new();
    };
    let mut s = format!("version=\"{version}\"");
    if !encoding.is_empty() {
        s.push_str(&format!(" encoding=\"{encoding}\""));
    }
    if !standalone.is_empty() {
        s.push_str(&format!(" standalone=\"{standalone}\""));
    }
    s
}

/// In-scope namespace bindings while writing (`prefix`, `uri`).
type Scope = Vec<(String, String)>;

fn in_scope<'s>(scope: &'s Scope, prefix: &str) -> Option<&'s str> {
    scope
        .iter()
        .rev()
        .find(|(p, _)| p == prefix)
        .map(|(_, u)| u.as_str())
}

/// Writes the start tag; returns the number of bindings pushed on `scope`.
fn start_tag(doc: &Document, n: NodeId, scope: &mut Scope, out: &mut String) -> usize {
    let Kind::Element {
        name, ns, attrs, ..
    } = doc.kind(n)
    else {
        unreachable!()
    };
    let before = scope.len();
    out.push('<');
    out.push_str(name);
    for &a in attrs {
        let Kind::Attribute { name: an, .. } = doc.kind(a) else {
            continue;
        };
        if an == "xmlns" {
            scope.push((String::new(), doc.value(a).unwrap_or_default()));
        } else if let Some(p) = an.strip_prefix("xmlns:") {
            scope.push((p.to_string(), doc.value(a).unwrap_or_default()));
        }
    }
    for &a in attrs {
        let Kind::Attribute {
            name: an, value, ..
        } = doc.kind(a)
        else {
            continue;
        };
        out.push(' ');
        out.push_str(an);
        out.push_str("=\"");
        escape_attr_into(value, out);
        out.push('"');
    }
    let mut extra: Vec<(String, String)> = Vec::new();
    let prefix = doc.prefix(n).to_string();
    if in_scope(scope, &prefix).unwrap_or("") != ns.as_str() {
        extra.push((prefix, ns.clone()));
    }
    for &a in attrs {
        let ap = doc.prefix(a);
        let ans = doc.namespace_uri(a);
        if ap.is_empty() || ap == "xmlns" || ans == XML_NS || ans == XMLNS_NS {
            continue;
        }
        if in_scope(scope, ap) != Some(ans) && !extra.iter().any(|(p, _)| p == ap) {
            extra.push((ap.to_string(), ans.to_string()));
        }
    }
    for (p, u) in extra {
        if p.is_empty() {
            out.push_str(" xmlns=\"");
        } else {
            out.push_str(" xmlns:");
            out.push_str(&p);
            out.push_str("=\"");
        }
        escape_attr_into(&u, out);
        out.push('"');
        scope.push((p, u));
    }
    scope.len() - before
}

fn leaf(doc: &Document, n: NodeId, out: &mut String) {
    match doc.kind(n) {
        Kind::Text(s) => escape_text_into(s, out),
        Kind::EntityRef(name) => {
            out.push('&');
            out.push_str(name);
            out.push(';');
        }
        Kind::CData(s) => {
            out.push_str("<![CDATA[");
            out.push_str(s);
            out.push_str("]]>");
        }
        Kind::Comment(s) => {
            out.push_str("<!--");
            out.push_str(s);
            out.push_str("-->");
        }
        Kind::Whitespace(s) | Kind::SignificantWhitespace(s) => out.push_str(s),
        Kind::Pi { target, data } => {
            out.push_str("<?");
            out.push_str(target);
            if !data.is_empty() {
                out.push(' ');
                out.push_str(data);
            }
            out.push_str("?>");
        }
        k @ Kind::XmlDecl { .. } => {
            out.push_str("<?xml ");
            out.push_str(&decl_value(k));
            out.push_str("?>");
        }
        Kind::DocType {
            name,
            public_id,
            system_id,
            subset,
        } => {
            out.push_str("<!DOCTYPE ");
            out.push_str(name);
            if let Some(p) = public_id {
                out.push_str(&format!(
                    " PUBLIC \"{p}\" \"{}\"",
                    system_id.as_deref().unwrap_or("")
                ));
            } else if let Some(s) = system_id {
                out.push_str(&format!(" SYSTEM \"{s}\""));
            }
            if let Some(s) = subset {
                out.push('[');
                out.push_str(s);
                out.push(']');
            }
            out.push('>');
        }
        Kind::Attribute { value, .. } => escape_attr_into(value, out),
        Kind::Document | Kind::Element { .. } => unreachable!(),
    }
}

fn compact(doc: &Document, n: NodeId, scope: &mut Scope, out: &mut String) {
    match doc.kind(n) {
        Kind::Document => {
            for &c in doc.children(n) {
                compact(doc, c, scope, out);
            }
        }
        Kind::Element { name, full_end, .. } => {
            let pushed = start_tag(doc, n, scope, out);
            let ch = doc.children(n);
            if ch.is_empty() && !full_end {
                out.push_str(" />");
            } else {
                out.push('>');
                for &c in ch {
                    compact(doc, c, scope, out);
                }
                out.push_str("</");
                out.push_str(name);
                out.push('>');
            }
            scope.truncate(scope.len() - pushed);
        }
        _ => leaf(doc, n, out),
    }
}

pub(crate) fn write_node(doc: &Document, n: NodeId, out: &mut String) {
    let mut scope = Scope::new();
    compact(doc, n, &mut scope, out);
}

/// Emulates `XmlTextWriter` with `Formatting.Indented`: children are put on
/// their own lines until text content makes an element "mixed".
struct Indenter<'a> {
    doc: &'a Document,
    indent: &'a str,
    newline: &'a str,
    out: String,
    scope: Scope,
    /// mixed-content flag per open element; index 0 is the document level
    mixed: Vec<bool>,
    wrote_any: bool,
}

impl Indenter<'_> {
    fn line(&mut self) {
        let depth = self.mixed.len() - 1;
        if *self.mixed.last().unwrap() {
            return;
        }
        if self.wrote_any {
            self.out.push_str(self.newline);
        }
        for _ in 0..depth {
            self.out.push_str(self.indent);
        }
    }

    fn node(&mut self, n: NodeId) {
        let doc = self.doc;
        match doc.kind(n) {
            Kind::Document => {
                for &c in doc.children(n) {
                    self.node(c);
                }
            }
            Kind::Element { name, full_end, .. } => {
                self.line();
                self.wrote_any = true;
                let pushed = start_tag(doc, n, &mut self.scope, &mut self.out);
                let ch = doc.children(n);
                if ch.is_empty() && !full_end {
                    self.out.push_str(" />");
                } else {
                    self.out.push('>');
                    let parent_mixed = *self.mixed.last().unwrap();
                    self.mixed.push(parent_mixed);
                    for &c in ch {
                        self.node(c);
                    }
                    let had_children = !ch.is_empty();
                    if had_children && !*self.mixed.last().unwrap() {
                        self.mixed.pop();
                        self.mixed.push(false);
                        let depth = self.mixed.len() - 2;
                        self.out.push_str(self.newline);
                        for _ in 0..depth {
                            self.out.push_str(self.indent);
                        }
                    }
                    self.mixed.pop();
                    self.out.push_str("</");
                    self.out.push_str(name);
                    self.out.push('>');
                }
                self.scope.truncate(self.scope.len() - pushed);
            }
            Kind::Text(_)
            | Kind::EntityRef(_)
            | Kind::CData(_)
            | Kind::Whitespace(_)
            | Kind::SignificantWhitespace(_) => {
                if self.mixed.len() > 1 {
                    *self.mixed.last_mut().unwrap() = true;
                }
                self.wrote_any = true;
                leaf(doc, n, &mut self.out);
            }
            _ => {
                self.line();
                self.wrote_any = true;
                leaf(doc, n, &mut self.out);
            }
        }
    }
}

pub(crate) fn indented(doc: &Document, n: NodeId, indent: &str, newline: &str) -> String {
    let mut w = Indenter {
        doc,
        indent,
        newline,
        out: String::new(),
        scope: Scope::new(),
        mixed: vec![false],
        wrote_any: false,
    };
    w.node(n);
    w.out
}
