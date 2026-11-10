//! XML parser mirroring the observable behaviour of `XmlDocument.Load`
//! (no end-of-line or attribute-value normalization, whitespace-only text
//! dropped unless preserved).

use crate::{attr_ns_for, Document, Kind, NodeId};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl ParseError {
    pub(crate) fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        ParseError {
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(
                f,
                "{} Line {}, position {}.",
                self.message, self.line, self.column
            )
        }
    }
}

impl std::error::Error for ParseError {}

/// Decodes raw file bytes as `XmlDocument.Load` would: BOM first, then the
/// `encoding` pseudo-attribute of the XML declaration, else UTF-8.
pub fn decode(bytes: &[u8]) -> Result<String, ParseError> {
    use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, UTF_8};
    let (enc, body): (&'static Encoding, &[u8]) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (UTF_8, &bytes[3..])
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (UTF_16LE, &bytes[2..])
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (UTF_16BE, &bytes[2..])
    } else if bytes.starts_with(&[0x3C, 0x00]) {
        (UTF_16LE, bytes)
    } else if bytes.starts_with(&[0x00, 0x3C]) {
        (UTF_16BE, bytes)
    } else {
        let head = &bytes[..bytes.len().min(200)];
        let head = String::from_utf8_lossy(head);
        let mut enc = UTF_8;
        if head.starts_with("<?xml") {
            if let Some(end) = head.find("?>") {
                let decl = &head[..end];
                if let Some(p) = decl.find("encoding") {
                    let rest = decl[p + 8..]
                        .trim_start()
                        .trim_start_matches('=')
                        .trim_start();
                    let q = rest.chars().next().unwrap_or('"');
                    if let Some(label) = rest.get(1..).and_then(|r| r.split(q).next()) {
                        match Encoding::for_label(label.trim().as_bytes()) {
                            Some(e) if e == UTF_16LE || e == UTF_16BE => {}
                            Some(e) => enc = e,
                            None => {
                                return Err(ParseError::new(
                                    format!("System does not support '{label}' encoding."),
                                    1,
                                    p + 1,
                                ))
                            }
                        }
                    }
                }
            }
        }
        (enc, bytes)
    };
    match enc.decode_without_bom_handling_and_without_replacement(body) {
        Some(s) => Ok(s.into_owned()),
        None => Err(ParseError::new(
            "Invalid character in the given encoding.",
            0,
            0,
        )),
    }
}

fn is_ws(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

fn is_valid_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}')
}

fn is_name_char(c: char) -> bool {
    !(is_ws(c)
        || matches!(
            c,
            '<' | '>'
                | '/'
                | '='
                | '"'
                | '\''
                | '&'
                | ';'
                | '?'
                | '!'
                | '['
                | ']'
                | '('
                | ')'
                | '%'
                | ','
                | '|'
                | '*'
                | '+'
                | '{'
                | '}'
                | '`'
                | '^'
                | '~'
                | '$'
                | '#'
                | '@'
                | '\\'
        ))
        && is_valid_char(c)
}

fn is_name_start(c: char) -> bool {
    is_name_char(c) && !c.is_ascii_digit() && !matches!(c, '-' | '.')
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    /// Offset of `src[0]` for error positions inside a fragment.
    fragment: bool,
}

struct Frame {
    node: NodeId,
    preserve: bool,
}

impl<'a> Parser<'a> {
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        self.err_at(self.pos, msg)
    }

    fn err_at<T>(&self, pos: usize, msg: impl Into<String>) -> Result<T, ParseError> {
        let before = &self.src[..pos.min(self.src.len())];
        let line = before.matches('\n').count() + 1;
        let col = before
            .rsplit('\n')
            .next()
            .map(|l| l.chars().count())
            .unwrap_or(0)
            + 1;
        Err(ParseError::new(msg, line, col))
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn starts(&self, s: &str) -> bool {
        self.rest().starts_with(s)
    }

    fn bump(&mut self, n: usize) {
        self.pos += n;
    }

    fn skip_ws(&mut self) -> bool {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_ws(c) {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.pos > start
    }

    fn expect(&mut self, s: &str) -> Result<(), ParseError> {
        if self.starts(s) {
            self.bump(s.len());
            Ok(())
        } else {
            match self.peek() {
                Some(c) => self.err(format!(
                    "'{c}' is an unexpected token. The expected token is '{s}'."
                )),
                None => self.err("Unexpected end of file has occurred."),
            }
        }
    }

    fn name(&mut self) -> Result<&'a str, ParseError> {
        let start = self.pos;
        match self.peek() {
            Some(c) if is_name_start(c) => self.bump(c.len_utf8()),
            Some(c) => {
                return self.err(format!(
                    "Name cannot begin with the '{c}' character, hexadecimal value 0x{:02X}.",
                    c as u32
                ))
            }
            None => return self.err("Unexpected end of file has occurred."),
        }
        while let Some(c) = self.peek() {
            if is_name_char(c) {
                self.bump(c.len_utf8());
            } else {
                break;
            }
        }
        Ok(&self.src[start..self.pos])
    }

    /// Parses `&...;` at the cursor and appends its expansion.
    /// In fragment mode, `&name;` naming an entity that is neither predefined
    /// nor declared: returns the name and the length of the reference.
    fn undeclared_entity(&self, doc: &Document) -> Option<(String, usize)> {
        if !self.fragment {
            return None;
        }
        let rest = self.src[self.pos..].strip_prefix('&')?;
        let end = rest.find(';')?;
        let name = &rest[..end];
        let mut chars = name.chars();
        let first = chars.next()?;
        if !(first.is_alphabetic() || first == '_' || first == ':')
            || !chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | ':' | '-' | '.' | '\u{B7}'))
        {
            return None;
        }
        if ["lt", "gt", "amp", "quot", "apos"].contains(&name) || doc.entities.contains_key(name) {
            return None;
        }
        Some((name.to_string(), end + 2))
    }

    fn reference(&mut self, doc: &Document, out: &mut String) -> Result<(), ParseError> {
        let start = self.pos;
        self.bump(1);
        if self.starts("#") {
            self.bump(1);
            let hex = self.starts("x");
            if hex {
                self.bump(1);
            }
            let ds = self.pos;
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() {
                    self.bump(1);
                } else {
                    break;
                }
            }
            let digits = &self.src[ds..self.pos];
            if !self.starts(";") {
                return self.err("An error occurred while parsing EntityName.");
            }
            self.bump(1);
            let v = if hex {
                u32::from_str_radix(digits, 16)
            } else {
                digits.parse::<u32>()
            };
            match v.ok().and_then(char::from_u32) {
                Some(c) if is_valid_char(c) => out.push(c),
                _ => {
                    return self.err_at(
                        start,
                        format!(
                            "Invalid character reference '&#{}{}'.",
                            if hex { "x" } else { "" },
                            digits
                        ),
                    )
                }
            }
            return Ok(());
        }
        let name = self.name()?;
        if !self.starts(";") {
            return self.err("An error occurred while parsing EntityName.");
        }
        self.bump(1);
        match name {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            _ => match doc.entities.get(name) {
                Some(v) => out.push_str(v),
                None => {
                    return self.err_at(
                        start + 1,
                        format!("Reference to undeclared entity '{name}'."),
                    )
                }
            },
        }
        Ok(())
    }

    fn attr_value(&mut self, doc: &Document) -> Result<String, ParseError> {
        let q = match self.peek() {
            Some(c @ ('"' | '\'')) => c,
            _ => {
                return self.err(
                    "There is no Unicode byte order mark... or quote: '\"' or ''' is expected.",
                )
            }
        };
        self.bump(1);
        let mut v = String::new();
        loop {
            match self.peek() {
                None => return self.err("Unexpected end of file has occurred."),
                Some(c) if c == q => {
                    self.bump(1);
                    return Ok(v);
                }
                Some('&') => self.reference(doc, &mut v)?,
                Some('<') => {
                    return self
                        .err("'<', hexadecimal value 0x3C, is an invalid attribute character.")
                }
                Some(c) if !is_valid_char(c) => {
                    return self.err(format!(
                        "'{c}', hexadecimal value 0x{:02X}, is an invalid character.",
                        c as u32
                    ))
                }
                Some(c) => {
                    v.push(c);
                    self.bump(c.len_utf8());
                }
            }
        }
    }

    fn until(&mut self, end: &str, what: &str) -> Result<&'a str, ParseError> {
        match self.rest().find(end) {
            Some(i) => {
                let s = &self.src[self.pos..self.pos + i];
                if let Some(c) = s.chars().find(|&c| !is_valid_char(c)) {
                    return self.err(format!(
                        "'{c}', hexadecimal value 0x{:02X}, is an invalid character.",
                        c as u32
                    ));
                }
                self.bump(i + end.len());
                Ok(s)
            }
            None => self.err(format!(
                "Unexpected end of file while parsing {what} has occurred."
            )),
        }
    }

    fn doctype(&mut self, doc: &mut Document) -> Result<Kind, ParseError> {
        self.skip_ws();
        let name = self.name()?.to_string();
        self.skip_ws();
        let mut public_id = None;
        let mut system_id = None;
        let quoted = |p: &mut Parser<'a>| -> Result<String, ParseError> {
            let q = p.peek().unwrap_or(' ');
            if q != '"' && q != '\'' {
                return p.err("Expecting a quoted literal.");
            }
            p.bump(1);
            let s = p.until(&q.to_string(), "DTD")?.to_string();
            Ok(s)
        };
        if self.starts("PUBLIC") {
            self.bump(6);
            self.skip_ws();
            public_id = Some(quoted(self)?);
            self.skip_ws();
            system_id = Some(quoted(self)?);
        } else if self.starts("SYSTEM") {
            self.bump(6);
            self.skip_ws();
            system_id = Some(quoted(self)?);
        }
        self.skip_ws();
        let mut subset = None;
        if self.starts("[") {
            self.bump(1);
            let start = self.pos;
            let mut depth_quote: Option<char> = None;
            loop {
                let Some(c) = self.peek() else {
                    return self.err("Unexpected end of file while parsing DTD has occurred.");
                };
                match depth_quote {
                    Some(q) if c == q => depth_quote = None,
                    Some(_) => {}
                    None if c == '"' || c == '\'' => depth_quote = Some(c),
                    None if c == ']' => break,
                    None => {}
                }
                self.bump(c.len_utf8());
            }
            let s = self.src[start..self.pos].to_string();
            self.bump(1);
            parse_entities(&s, doc);
            subset = Some(s);
            self.skip_ws();
        }
        self.expect(">")?;
        Ok(Kind::DocType {
            name,
            public_id,
            system_id,
            subset,
        })
    }
}

fn parse_entities(subset: &str, doc: &mut Document) {
    let mut rest = subset;
    while let Some(i) = rest.find("<!ENTITY") {
        rest = &rest[i + 8..];
        let t = rest.trim_start();
        if t.starts_with('%') {
            continue;
        }
        let Some(name_end) = t.find(|c: char| is_ws(c)) else {
            break;
        };
        let name = &t[..name_end];
        let after = t[name_end..].trim_start();
        let Some(q) = after.chars().next() else { break };
        if q != '"' && q != '\'' {
            continue;
        }
        if let Some(end) = after[1..].find(q) {
            let raw = &after[1..1 + end];
            let mut v = String::new();
            let mut p = Parser {
                src: raw,
                pos: 0,
                fragment: true,
            };
            while let Some(c) = p.peek() {
                if c == '&'
                    && (raw[p.pos..].starts_with("&#")
                        || ["&lt;", "&gt;", "&amp;", "&quot;", "&apos;"]
                            .iter()
                            .any(|e| raw[p.pos..].starts_with(e)))
                {
                    if p.reference(doc, &mut v).is_err() {
                        break;
                    }
                } else if let Some(inner) = c
                    .eq(&'&')
                    .then(|| raw[p.pos + 1..].split_once(';'))
                    .flatten()
                    .and_then(|(n, _)| doc.entities.get(n).map(|v| (n.len(), v.clone())))
                {
                    // References to entities declared earlier are expanded.
                    v.push_str(&inner.1);
                    p.bump(inner.0 + 2);
                } else {
                    v.push(c);
                    p.bump(c.len_utf8());
                }
            }
            doc.entities.entry(name.to_string()).or_insert(v);
        }
    }
}

/// Parses `xml` and appends the resulting nodes under `parent`.
///
/// `fragment` selects `InnerXml` semantics (whitespace kept, no root-level
/// restrictions); otherwise document rules apply.
pub(crate) fn parse_into(
    doc: &mut Document,
    parent: NodeId,
    xml: &str,
    fragment: bool,
    _document_mode: bool,
) -> Result<(), ParseError> {
    let mut p = Parser {
        src: xml,
        pos: 0,
        fragment,
    };
    let preserve_all = fragment || doc.preserve_whitespace;
    let inherited_preserve = {
        let mut cur = Some(parent);
        let mut v = false;
        while let Some(c) = cur {
            if let Some(s) = doc.attr(c, "xml:space") {
                v = s == "preserve";
                break;
            }
            cur = doc.nodes[c.0].parent;
        }
        v
    };
    let mut stack: Vec<Frame> = vec![Frame {
        node: parent,
        preserve: inherited_preserve,
    }];
    let mut seen_root = false;
    let mut text = String::new();
    let mut text_start = 0usize;
    let mut text_is_ws = true;

    macro_rules! flush_text {
        () => {
            if !text.is_empty() {
                let top = stack.last().unwrap();
                let at_root = !p.fragment && stack.len() == 1;
                if text_is_ws {
                    if preserve_all {
                        let n = doc.alloc(Kind::Whitespace(std::mem::take(&mut text)));
                        doc.append_child(top.node, n);
                    } else if top.preserve && !at_root {
                        let n = doc.alloc(Kind::SignificantWhitespace(std::mem::take(&mut text)));
                        doc.append_child(top.node, n);
                    } else {
                        text.clear();
                    }
                } else {
                    if at_root {
                        return p.err_at(text_start, "Data at the root level is invalid.");
                    }
                    let n = doc.alloc(Kind::Text(std::mem::take(&mut text)));
                    doc.append_child(top.node, n);
                }
                text_is_ws = true;
            }
        };
    }

    while p.pos < xml.len() {
        let c = p.peek().unwrap();
        if c != '<' {
            if text.is_empty() {
                text_start = p.pos;
            }
            if c == '&' {
                if !p.fragment && stack.len() == 1 {
                    return p.err("Data at the root level is invalid.");
                }
                if let Some((name, len)) = p.undeclared_entity(doc) {
                    // Fragments (`InnerXml`) keep undeclared entities as
                    // `XmlEntityReference` nodes instead of failing.
                    flush_text!();
                    let n = doc.alloc(Kind::EntityRef(name));
                    doc.append_child(stack.last().unwrap().node, n);
                    p.bump(len);
                    continue;
                }
                p.reference(doc, &mut text)?;
                text_is_ws = false;
            } else if !is_valid_char(c) {
                return p.err(format!(
                    "'{c}', hexadecimal value 0x{:02X}, is an invalid character.",
                    c as u32
                ));
            } else {
                if !is_ws(c) {
                    text_is_ws = false;
                }
                if c == '>' && text.ends_with("]]") {
                    return p.err("']]>' is not allowed in character data.");
                }
                text.push(c);
                p.bump(c.len_utf8());
            }
            continue;
        }
        flush_text!();
        if p.starts("<!--") {
            p.bump(4);
            let s = p.until("-->", "comment")?.to_string();
            let n = doc.alloc(Kind::Comment(s));
            doc.append_child(stack.last().unwrap().node, n);
        } else if p.starts("<![CDATA[") {
            if !p.fragment && stack.len() == 1 {
                return p.err("Data at the root level is invalid.");
            }
            p.bump(9);
            let s = p.until("]]>", "CDATA")?.to_string();
            let n = doc.alloc(Kind::CData(s));
            doc.append_child(stack.last().unwrap().node, n);
        } else if p.starts("<!DOCTYPE") {
            if p.fragment || stack.len() != 1 || seen_root {
                return p.err("Unexpected DTD declaration.");
            }
            p.bump(9);
            let k = p.doctype(doc)?;
            let n = doc.alloc(k);
            doc.append_child(parent, n);
        } else if p.starts("<?") {
            let start = p.pos;
            p.bump(2);
            let target = p.name()?.to_string();
            let has_ws = p.skip_ws();
            let data = p.until("?>", "processing instruction")?.to_string();
            if target.eq_ignore_ascii_case("xml") {
                if p.fragment || start != 0 || target != "xml" {
                    return p.err_at(start, "Unexpected XML declaration. The XML declaration must be the first node in the document, and no white space characters are allowed to appear before it.");
                }
                let n = doc.alloc(parse_decl(&data));
                doc.append_child(parent, n);
            } else {
                if !has_ws && !data.is_empty() {
                    return p.err_at(start, "Invalid processing instruction.");
                }
                let n = doc.alloc(Kind::Pi { target, data });
                doc.append_child(stack.last().unwrap().node, n);
            }
        } else if p.starts("</") {
            let start = p.pos;
            p.bump(2);
            let name = p.name()?;
            p.skip_ws();
            p.expect(">")?;
            if stack.len() == 1 {
                return p.err_at(start, "Unexpected end tag.");
            }
            let top = stack.pop().unwrap();
            let open = doc.name(top.node);
            if open != name {
                return p.err_at(
                    start + 2,
                    format!("The '{open}' start tag does not match the end tag of '{name}'."),
                );
            }
            if let Kind::Element { full_end, .. } = &mut doc.nodes[top.node.0].kind {
                *full_end = true;
            }
        } else {
            let start = p.pos;
            p.bump(1);
            if !p.fragment && stack.len() == 1 {
                if seen_root {
                    return p.err_at(start, "There are multiple root elements.");
                }
                seen_root = true;
            }
            let name = p.name()?.to_string();
            let el = doc.alloc(Kind::Element {
                name: name.clone(),
                ns: String::new(),
                attrs: Vec::new(),
                full_end: false,
            });
            let mut preserve = stack.last().unwrap().preserve;
            loop {
                let had_ws = p.skip_ws();
                if p.starts("/>") || p.starts(">") {
                    break;
                }
                if !had_ws {
                    return p.err("Expected whitespace between attributes.");
                }
                let an = p.name()?.to_string();
                p.skip_ws();
                p.expect("=")?;
                p.skip_ws();
                let av = p.attr_value(doc)?;
                if doc.attribute_node(el, &an).is_some() {
                    return p.err(format!("'{an}' is a duplicate attribute name."));
                }
                if an == "xml:space" {
                    preserve = av == "preserve";
                }
                let a = doc.alloc(Kind::Attribute {
                    ns: attr_ns_for(&an),
                    name: an,
                    value: av,
                });
                doc.append_attribute(el, a);
            }
            doc.append_child(stack.last().unwrap().node, el);
            resolve_ns(doc, el, &p, start)?;
            if p.starts("/>") {
                p.bump(2);
            } else {
                p.bump(1);
                stack.push(Frame { node: el, preserve });
            }
        }
    }
    #[allow(unused_assignments)]
    {
        flush_text!();
    }
    if stack.len() > 1 {
        let n = doc.name(stack.last().unwrap().node).to_string();
        return p.err(format!(
            "Unexpected end of file has occurred. The following elements are not closed: {n}."
        ));
    }
    if !p.fragment && !seen_root {
        return p.err("Root element is missing.");
    }
    Ok(())
}

fn resolve_ns(doc: &mut Document, el: NodeId, p: &Parser<'_>, at: usize) -> Result<(), ParseError> {
    let prefix = doc.prefix(el).to_string();
    let ns = match doc.lookup_namespace(el, &prefix) {
        Some(ns) => ns,
        None => return p.err_at(at, format!("'{prefix}' is an undeclared prefix.")),
    };
    if let Kind::Element { ns: n, .. } = &mut doc.nodes[el.0].kind {
        *n = ns;
    }
    let attrs: Vec<NodeId> = doc.attributes(el).to_vec();
    for a in attrs {
        let ap = doc.prefix(a).to_string();
        if ap.is_empty() || ap == "xmlns" || ap == "xml" {
            continue;
        }
        let ns = match doc.lookup_namespace(el, &ap) {
            Some(ns) => ns,
            None => return p.err_at(at, format!("'{ap}' is an undeclared prefix.")),
        };
        if let Kind::Attribute { ns: n, .. } = &mut doc.nodes[a.0].kind {
            *n = ns;
        }
    }
    Ok(())
}

fn parse_decl(data: &str) -> Kind {
    let get = |key: &str| -> String {
        let Some(i) = data.find(key) else {
            return String::new();
        };
        let rest = data[i + key.len()..]
            .trim_start()
            .trim_start_matches('=')
            .trim_start();
        let Some(q) = rest.chars().next() else {
            return String::new();
        };
        rest[1..].split(q).next().unwrap_or("").to_string()
    };
    Kind::XmlDecl {
        version: get("version"),
        encoding: get("encoding"),
        standalone: get("standalone"),
    }
}
