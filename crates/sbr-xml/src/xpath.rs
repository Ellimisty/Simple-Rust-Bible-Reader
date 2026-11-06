//! XPath 1.0 evaluator over [`Document`], used where the original code calls
//! `SelectNodes` / `SelectSingleNode`.

use crate::{Document, Kind, NodeId, XMLNS_NS};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum XPathError {
    #[error("'{0}' has an invalid token.")]
    Syntax(String),
    #[error("Namespace Manager or XsltContext needed. This query has a prefix, variable, or user-defined function.")]
    UnknownPrefix(String),
    #[error("Expression must evaluate to a node-set.")]
    NotNodeSet,
    #[error("'{0}' is an unknown XPath function.")]
    UnknownFunction(String),
}

/// `XmlNamespaceManager` equivalent: prefix → namespace URI.
#[derive(Debug, Clone, Default)]
pub struct Namespaces(HashMap<String, String>);

impl Namespaces {
    pub fn new() -> Self {
        Namespaces(HashMap::new())
    }
    pub fn add(&mut self, prefix: &str, uri: &str) {
        self.0.insert(prefix.to_string(), uri.to_string());
    }
    fn get(&self, prefix: &str) -> Option<&str> {
        if prefix == "xml" {
            return Some(crate::XML_NS);
        }
        self.0.get(prefix).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------- lexer

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Slash,
    DSlash,
    LBr,
    RBr,
    LPar,
    RPar,
    At,
    Dot,
    DDot,
    Comma,
    Pipe,
    DColon,
    Op(&'static str),
    Lit(String),
    Num(f64),
    /// name test: (prefix, local) where local may be "*"
    Name(Option<String>, String),
    Star,
    Func(String),
    Axis(String),
    NodeType(String),
}

fn lex(src: &str) -> Result<Vec<Tok>, XPathError> {
    let err = || XPathError::Syntax(src.to_string());
    let cs: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out: Vec<Tok> = Vec::new();
    let is_ncstart = |c: char| c.is_alphabetic() || c == '_';
    let is_nc = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '\u{B7}');
    while i < cs.len() {
        let c = cs[i];
        // whether the preceding token forces operator interpretation
        let op_ctx = match out.last() {
            None => false,
            Some(
                Tok::At
                | Tok::DColon
                | Tok::LPar
                | Tok::LBr
                | Tok::Comma
                | Tok::Op(_)
                | Tok::Slash
                | Tok::DSlash
                | Tok::Pipe,
            ) => false,
            Some(_) => true,
        };
        match c {
            ' ' | '\t' | '\n' | '\r' => i += 1,
            '/' => {
                if cs.get(i + 1) == Some(&'/') {
                    out.push(Tok::DSlash);
                    i += 2;
                } else {
                    out.push(Tok::Slash);
                    i += 1;
                }
            }
            '[' => {
                out.push(Tok::LBr);
                i += 1
            }
            ']' => {
                out.push(Tok::RBr);
                i += 1
            }
            '(' => {
                out.push(Tok::LPar);
                i += 1
            }
            ')' => {
                out.push(Tok::RPar);
                i += 1
            }
            '@' => {
                out.push(Tok::At);
                i += 1
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1
            }
            '|' => {
                out.push(Tok::Pipe);
                i += 1
            }
            ':' if cs.get(i + 1) == Some(&':') => {
                out.push(Tok::DColon);
                i += 2
            }
            '.' if cs.get(i + 1) == Some(&'.') => {
                out.push(Tok::DDot);
                i += 2
            }
            '.' if !cs.get(i + 1).is_some_and(|c| c.is_ascii_digit()) => {
                out.push(Tok::Dot);
                i += 1
            }
            '=' => {
                out.push(Tok::Op("="));
                i += 1
            }
            '!' if cs.get(i + 1) == Some(&'=') => {
                out.push(Tok::Op("!="));
                i += 2
            }
            '<' | '>' => {
                if cs.get(i + 1) == Some(&'=') {
                    out.push(Tok::Op(if c == '<' { "<=" } else { ">=" }));
                    i += 2;
                } else {
                    out.push(Tok::Op(if c == '<' { "<" } else { ">" }));
                    i += 1;
                }
            }
            '+' => {
                out.push(Tok::Op("+"));
                i += 1
            }
            '-' => {
                out.push(Tok::Op("-"));
                i += 1
            }
            '*' => {
                out.push(if op_ctx { Tok::Op("*") } else { Tok::Star });
                i += 1
            }
            '"' | '\'' => {
                let start = i + 1;
                let mut j = start;
                while j < cs.len() && cs[j] != c {
                    j += 1;
                }
                if j >= cs.len() {
                    return Err(err());
                }
                out.push(Tok::Lit(cs[start..j].iter().collect()));
                i = j + 1;
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                    i += 1;
                }
                let s: String = cs[start..i].iter().collect();
                out.push(Tok::Num(s.parse().map_err(|_| err())?));
            }
            c if is_ncstart(c) => {
                let start = i;
                while i < cs.len() && is_nc(cs[i]) {
                    i += 1;
                }
                let first: String = cs[start..i].iter().collect();
                if op_ctx && matches!(first.as_str(), "and" | "or" | "mod" | "div") {
                    out.push(Tok::Op(match first.as_str() {
                        "and" => "and",
                        "or" => "or",
                        "mod" => "mod",
                        _ => "div",
                    }));
                    continue;
                }
                let mut prefix = None;
                let mut local = first;
                if cs.get(i) == Some(&':') && cs.get(i + 1) != Some(&':') {
                    if cs.get(i + 1) == Some(&'*') {
                        prefix = Some(local);
                        local = "*".into();
                        i += 2;
                    } else if cs.get(i + 1).is_some_and(|&c| is_ncstart(c)) {
                        let s2 = i + 1;
                        i += 1;
                        while i < cs.len() && is_nc(cs[i]) {
                            i += 1;
                        }
                        prefix = Some(local);
                        local = cs[s2..i].iter().collect();
                    } else {
                        return Err(err());
                    }
                }
                let mut j = i;
                while j < cs.len() && matches!(cs[j], ' ' | '\t' | '\n' | '\r') {
                    j += 1;
                }
                if prefix.is_none()
                    && local != "*"
                    && cs.get(j) == Some(&':')
                    && cs.get(j + 1) == Some(&':')
                {
                    out.push(Tok::Axis(local));
                } else if cs.get(j) == Some(&'(') && local != "*" {
                    let full = match &prefix {
                        Some(p) => format!("{p}:{local}"),
                        None => local.clone(),
                    };
                    if prefix.is_none()
                        && matches!(
                            local.as_str(),
                            "node" | "text" | "comment" | "processing-instruction"
                        )
                    {
                        out.push(Tok::NodeType(local));
                    } else {
                        out.push(Tok::Func(full));
                    }
                } else {
                    out.push(Tok::Name(prefix, local));
                }
            }
            _ => return Err(err()),
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- AST

#[derive(Debug, Clone, Copy, PartialEq)]
enum Axis {
    Child,
    Descendant,
    DescendantOrSelf,
    Parent,
    Ancestor,
    AncestorOrSelf,
    FollowingSibling,
    PrecedingSibling,
    Following,
    Preceding,
    Attribute,
    Self_,
    Namespace,
}

impl Axis {
    fn reverse(self) -> bool {
        matches!(
            self,
            Axis::Ancestor
                | Axis::AncestorOrSelf
                | Axis::PrecedingSibling
                | Axis::Preceding
                | Axis::Parent
        )
    }
}

#[derive(Debug, Clone)]
enum Test {
    Name(Option<String>, String),
    Node,
    Text,
    Comment,
    Pi(Option<String>),
}

#[derive(Debug, Clone)]
struct Step {
    axis: Axis,
    test: Test,
    preds: Vec<Expr>,
}

#[derive(Debug, Clone)]
enum Expr {
    Bin(&'static str, Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Union(Box<Expr>, Box<Expr>),
    /// `absolute`, steps
    Path(bool, Vec<Step>),
    /// primary, predicates, trailing steps
    Filter(Box<Expr>, Vec<Expr>, Vec<Step>),
    Lit(String),
    Num(f64),
    Func(String, Vec<Expr>),
}

struct P {
    t: Vec<Tok>,
    i: usize,
    src: String,
}

impl P {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.t.get(self.i).cloned();
        self.i += 1;
        t
    }
    fn err<T>(&self) -> Result<T, XPathError> {
        Err(XPathError::Syntax(self.src.clone()))
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, t: &Tok) -> Result<(), XPathError> {
        if self.eat(t) {
            Ok(())
        } else {
            self.err()
        }
    }

    fn binary(&mut self, level: usize) -> Result<Expr, XPathError> {
        const LEVELS: [&[&str]; 5] = [
            &["or"],
            &["and"],
            &["=", "!="],
            &["<", ">", "<=", ">="],
            &["+", "-"],
        ];
        if level == LEVELS.len() {
            return self.multiplicative();
        }
        let mut l = self.binary(level + 1)?;
        while let Some(Tok::Op(op)) = self.peek() {
            let op = *op;
            if !LEVELS[level].contains(&op) {
                break;
            }
            self.i += 1;
            let r = self.binary(level + 1)?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn multiplicative(&mut self) -> Result<Expr, XPathError> {
        let mut l = self.unary()?;
        while let Some(Tok::Op(op @ ("*" | "div" | "mod"))) = self.peek() {
            let op = *op;
            self.i += 1;
            let r = self.unary()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn unary(&mut self) -> Result<Expr, XPathError> {
        if self.eat(&Tok::Op("-")) {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        let mut l = self.path_expr()?;
        while self.eat(&Tok::Pipe) {
            let r = self.path_expr()?;
            l = Expr::Union(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn path_expr(&mut self) -> Result<Expr, XPathError> {
        match self.peek() {
            Some(Tok::Lit(_) | Tok::Num(_) | Tok::LPar | Tok::Func(_)) => {
                let prim = self.primary()?;
                let mut preds = Vec::new();
                while self.peek() == Some(&Tok::LBr) {
                    preds.push(self.predicate()?);
                }
                let mut steps = Vec::new();
                if matches!(self.peek(), Some(Tok::Slash | Tok::DSlash)) {
                    self.rel_path(&mut steps, true)?;
                }
                if preds.is_empty() && steps.is_empty() {
                    Ok(prim)
                } else {
                    Ok(Expr::Filter(Box::new(prim), preds, steps))
                }
            }
            Some(Tok::Slash) => {
                self.i += 1;
                let mut steps = Vec::new();
                if self.starts_step() {
                    self.rel_path(&mut steps, false)?;
                }
                Ok(Expr::Path(true, steps))
            }
            Some(Tok::DSlash) => {
                let mut steps = Vec::new();
                self.rel_path(&mut steps, true)?;
                Ok(Expr::Path(true, steps))
            }
            _ => {
                let mut steps = Vec::new();
                self.rel_path(&mut steps, false)?;
                Ok(Expr::Path(false, steps))
            }
        }
    }

    fn starts_step(&self) -> bool {
        matches!(
            self.peek(),
            Some(
                Tok::Name(..)
                    | Tok::Star
                    | Tok::At
                    | Tok::Dot
                    | Tok::DDot
                    | Tok::Axis(_)
                    | Tok::NodeType(_)
            )
        )
    }

    /// Parses `step (('/'|'//') step)*`; if `lead_sep`, a separator comes first.
    fn rel_path(&mut self, steps: &mut Vec<Step>, lead_sep: bool) -> Result<(), XPathError> {
        if !lead_sep {
            steps.push(self.step()?);
        }
        loop {
            match self.peek() {
                Some(Tok::Slash) => {
                    self.i += 1;
                    steps.push(self.step()?);
                }
                Some(Tok::DSlash) => {
                    self.i += 1;
                    steps.push(Step {
                        axis: Axis::DescendantOrSelf,
                        test: Test::Node,
                        preds: vec![],
                    });
                    steps.push(self.step()?);
                }
                _ => return Ok(()),
            }
        }
    }

    fn step(&mut self) -> Result<Step, XPathError> {
        if self.eat(&Tok::Dot) {
            return Ok(Step {
                axis: Axis::Self_,
                test: Test::Node,
                preds: vec![],
            });
        }
        if self.eat(&Tok::DDot) {
            return Ok(Step {
                axis: Axis::Parent,
                test: Test::Node,
                preds: vec![],
            });
        }
        let mut axis = Axis::Child;
        if self.eat(&Tok::At) {
            axis = Axis::Attribute;
        } else if let Some(Tok::Axis(a)) = self.peek().cloned() {
            self.i += 1;
            self.expect(&Tok::DColon)?;
            axis = match a.as_str() {
                "child" => Axis::Child,
                "descendant" => Axis::Descendant,
                "descendant-or-self" => Axis::DescendantOrSelf,
                "parent" => Axis::Parent,
                "ancestor" => Axis::Ancestor,
                "ancestor-or-self" => Axis::AncestorOrSelf,
                "following-sibling" => Axis::FollowingSibling,
                "preceding-sibling" => Axis::PrecedingSibling,
                "following" => Axis::Following,
                "preceding" => Axis::Preceding,
                "attribute" => Axis::Attribute,
                "self" => Axis::Self_,
                "namespace" => Axis::Namespace,
                _ => return self.err(),
            };
        }
        let test = match self.next() {
            Some(Tok::Star) => Test::Name(None, "*".into()),
            Some(Tok::Name(p, l)) => Test::Name(p, l),
            Some(Tok::NodeType(t)) => {
                self.expect(&Tok::LPar)?;
                let r = match t.as_str() {
                    "node" => Test::Node,
                    "text" => Test::Text,
                    "comment" => Test::Comment,
                    _ => {
                        if let Some(Tok::Lit(s)) = self.peek().cloned() {
                            self.i += 1;
                            Test::Pi(Some(s))
                        } else {
                            Test::Pi(None)
                        }
                    }
                };
                self.expect(&Tok::RPar)?;
                r
            }
            _ => return self.err(),
        };
        let mut preds = Vec::new();
        while self.peek() == Some(&Tok::LBr) {
            preds.push(self.predicate()?);
        }
        Ok(Step { axis, test, preds })
    }

    fn predicate(&mut self) -> Result<Expr, XPathError> {
        self.expect(&Tok::LBr)?;
        let e = self.binary(0)?;
        self.expect(&Tok::RBr)?;
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, XPathError> {
        match self.next() {
            Some(Tok::Lit(s)) => Ok(Expr::Lit(s)),
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::LPar) => {
                let e = self.binary(0)?;
                self.expect(&Tok::RPar)?;
                Ok(e)
            }
            Some(Tok::Func(name)) => {
                self.expect(&Tok::LPar)?;
                let mut args = Vec::new();
                if !self.eat(&Tok::RPar) {
                    loop {
                        args.push(self.binary(0)?);
                        if self.eat(&Tok::RPar) {
                            break;
                        }
                        self.expect(&Tok::Comma)?;
                    }
                }
                Ok(Expr::Func(name, args))
            }
            _ => self.err(),
        }
    }
}

fn parse(src: &str) -> Result<Expr, XPathError> {
    let t = lex(src)?;
    let mut p = P {
        t,
        i: 0,
        src: src.to_string(),
    };
    let e = p.binary(0)?;
    if p.i != p.t.len() {
        return p.err();
    }
    Ok(e)
}

// ---------------------------------------------------------------- values

#[derive(Debug, Clone)]
pub enum Value {
    Nodes(Vec<NodeId>),
    Str(String),
    Num(f64),
    Bool(bool),
}

pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n == n.trunc() && n.abs() < 1e18 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

pub fn string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'));
    let body = t.strip_prefix('-').unwrap_or(t);
    let valid = !body.is_empty()
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.matches('.').count() <= 1
        && body != ".";
    if valid {
        t.parse().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

struct Ctx<'a> {
    doc: &'a Document,
    ns: &'a Namespaces,
}

#[derive(Clone, Copy)]
struct Focus {
    node: NodeId,
    pos: usize,
    size: usize,
}

impl<'a> Ctx<'a> {
    fn string_value(&self, n: NodeId) -> String {
        self.doc.inner_text(n)
    }

    fn to_str(&self, v: &Value) -> String {
        match v {
            Value::Nodes(ns) => ns
                .first()
                .map(|&n| self.string_value(n))
                .unwrap_or_default(),
            Value::Str(s) => s.clone(),
            Value::Num(n) => number_to_string(*n),
            Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        }
    }

    fn to_num(&self, v: &Value) -> f64 {
        match v {
            Value::Num(n) => *n,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            _ => string_to_number(&self.to_str(v)),
        }
    }

    fn to_bool(&self, v: &Value) -> bool {
        match v {
            Value::Nodes(ns) => !ns.is_empty(),
            Value::Str(s) => !s.is_empty(),
            Value::Num(n) => *n != 0.0 && !n.is_nan(),
            Value::Bool(b) => *b,
        }
    }

    fn top(&self, mut n: NodeId) -> NodeId {
        loop {
            match self.doc.nodes[n.0].parent {
                Some(p) => n = p,
                None => return n,
            }
        }
    }

    /// Sort key giving document order.
    fn order_key(&self, n: NodeId) -> Vec<(u8, usize)> {
        let mut key = Vec::new();
        let mut cur = n;
        loop {
            let node = &self.doc.nodes[cur.0];
            let Some(p) = node.parent else { break };
            if matches!(node.kind, Kind::Attribute { .. }) {
                let i = self
                    .doc
                    .attributes(p)
                    .iter()
                    .position(|&a| a == cur)
                    .unwrap_or(0);
                key.push((0, i));
            } else {
                let i = self.doc.nodes[p.0]
                    .children
                    .iter()
                    .position(|&c| c == cur)
                    .unwrap_or(0);
                key.push((1, i));
            }
            cur = p;
        }
        key.push((0, cur.0));
        key.reverse();
        key
    }

    fn sort_dedup(&self, nodes: &mut Vec<NodeId>) {
        if nodes.len() < 2 {
            return;
        }
        let mut keyed: Vec<(Vec<(u8, usize)>, NodeId)> =
            nodes.iter().map(|&n| (self.order_key(n), n)).collect();
        keyed.sort();
        keyed.dedup_by(|a, b| a.1 == b.1);
        *nodes = keyed.into_iter().map(|(_, n)| n).collect();
    }

    fn matches(&self, axis: Axis, test: &Test, n: NodeId) -> Result<bool, XPathError> {
        let k = self.doc.kind(n);
        Ok(match test {
            Test::Node => true,
            Test::Text => matches!(
                k,
                Kind::Text(_)
                    | Kind::CData(_)
                    | Kind::Whitespace(_)
                    | Kind::SignificantWhitespace(_)
            ),
            Test::Comment => matches!(k, Kind::Comment(_)),
            Test::Pi(t) => {
                matches!(k, Kind::Pi { target, .. } if t.as_ref().is_none_or(|t| t == target))
            }
            Test::Name(prefix, local) => {
                let principal = if axis == Axis::Attribute {
                    matches!(k, Kind::Attribute { .. })
                } else {
                    matches!(k, Kind::Element { .. })
                };
                if !principal {
                    return Ok(false);
                }
                let want_ns = match prefix {
                    Some(p) => self
                        .ns
                        .get(p)
                        .ok_or_else(|| XPathError::UnknownPrefix(p.clone()))?,
                    None => "",
                };
                if local == "*" {
                    prefix.is_none() || self.doc.namespace_uri(n) == want_ns
                } else {
                    self.doc.local_name(n) == local && self.doc.namespace_uri(n) == want_ns
                }
            }
        })
    }

    fn axis_nodes(&self, axis: Axis, n: NodeId, out: &mut Vec<NodeId>) {
        let d = self.doc;
        let is_attr = matches!(d.kind(n), Kind::Attribute { .. });
        match axis {
            Axis::Child => {
                if !is_attr {
                    out.extend(d.children(n).iter().copied().filter(|&c| {
                        !matches!(d.kind(c), Kind::XmlDecl { .. } | Kind::DocType { .. })
                    }));
                }
            }
            Axis::Descendant | Axis::DescendantOrSelf => {
                if axis == Axis::DescendantOrSelf {
                    out.push(n);
                }
                if !is_attr {
                    d.descendants(n, &mut |d, c| {
                        if !matches!(d.kind(c), Kind::XmlDecl { .. } | Kind::DocType { .. }) {
                            out.push(c)
                        }
                    });
                }
            }
            Axis::Parent => {
                if let Some(p) = d.nodes[n.0].parent {
                    out.push(p);
                }
            }
            Axis::Ancestor | Axis::AncestorOrSelf => {
                if axis == Axis::AncestorOrSelf {
                    out.push(n);
                }
                let mut cur = d.nodes[n.0].parent;
                while let Some(p) = cur {
                    out.push(p);
                    cur = d.nodes[p.0].parent;
                }
            }
            Axis::FollowingSibling | Axis::PrecedingSibling => {
                if is_attr {
                    return;
                }
                if let Some((p, i)) = d.index_in_parent(n) {
                    let ch = &d.nodes[p.0].children;
                    if axis == Axis::FollowingSibling {
                        out.extend(ch[i + 1..].iter().copied());
                    } else {
                        out.extend(ch[..i].iter().rev().copied());
                    }
                }
            }
            Axis::Following => {
                let mut cur = if is_attr {
                    d.nodes[n.0].parent
                } else {
                    Some(n)
                };
                let mut first = !is_attr;
                if is_attr {
                    if let Some(o) = cur {
                        d.descendants(o, &mut |_, c| out.push(c));
                    }
                }
                while let Some(c) = cur {
                    if first {
                        first = false;
                    }
                    if let Some((p, i)) = d.index_in_parent(c) {
                        for &s in &d.nodes[p.0].children[i + 1..] {
                            out.push(s);
                            d.descendants(s, &mut |_, x| out.push(x));
                        }
                        cur = Some(p);
                    } else {
                        cur = None;
                    }
                }
            }
            Axis::Preceding => {
                let mut all = Vec::new();
                let top = self.top(n);
                d.descendants(top, &mut |_, c| all.push(c));
                let mut anc = Vec::new();
                self.axis_nodes(Axis::Ancestor, n, &mut anc);
                let start = if is_attr {
                    d.nodes[n.0].parent.unwrap()
                } else {
                    n
                };
                if let Some(idx) = all.iter().position(|&c| c == start) {
                    for &c in all[..idx].iter().rev() {
                        if !anc.contains(&c) {
                            out.push(c);
                        }
                    }
                }
            }
            Axis::Attribute => {
                out.extend(
                    d.attributes(n)
                        .iter()
                        .copied()
                        .filter(|&a| d.namespace_uri(a) != XMLNS_NS),
                );
            }
            Axis::Self_ => out.push(n),
            Axis::Namespace => {}
        }
    }

    fn step(
        &self,
        input: &[NodeId],
        ordered_flat: bool,
        step: &Step,
    ) -> Result<(Vec<NodeId>, bool), XPathError> {
        let mut result = Vec::new();
        for &n in input {
            let mut cand = Vec::new();
            self.axis_nodes(step.axis, n, &mut cand);
            let mut sel = Vec::with_capacity(cand.len());
            for c in cand {
                if self.matches(step.axis, &step.test, c)? {
                    sel.push(c);
                }
            }
            for p in &step.preds {
                sel = self.filter(sel, p)?;
            }
            if step.axis.reverse() {
                sel.reverse();
            }
            result.extend(sel);
        }
        let keeps_flat = matches!(step.axis, Axis::Child | Axis::Attribute | Axis::Self_);
        if input.len() <= 1 {
            Ok((result, keeps_flat || matches!(step.axis, Axis::Parent)))
        } else if ordered_flat && keeps_flat {
            Ok((result, true))
        } else {
            self.sort_dedup(&mut result);
            Ok((result, false))
        }
    }

    fn filter(&self, nodes: Vec<NodeId>, pred: &Expr) -> Result<Vec<NodeId>, XPathError> {
        let size = nodes.len();
        let mut out = Vec::new();
        for (i, n) in nodes.into_iter().enumerate() {
            let f = Focus {
                node: n,
                pos: i + 1,
                size,
            };
            let v = self.eval(pred, f)?;
            let keep = match v {
                Value::Num(x) => x == (i + 1) as f64,
                other => self.to_bool(&other),
            };
            if keep {
                out.push(n);
            }
        }
        Ok(out)
    }

    fn steps(
        &self,
        mut nodes: Vec<NodeId>,
        mut flat: bool,
        steps: &[Step],
    ) -> Result<Vec<NodeId>, XPathError> {
        let mut i = 0;
        while i < steps.len() {
            let s = &steps[i];
            // `//name[...]` → descendant::name[...] when predicates are positionless
            if s.axis == Axis::DescendantOrSelf
                && matches!(s.test, Test::Node)
                && s.preds.is_empty()
            {
                if let Some(next) = steps.get(i + 1) {
                    if next.axis == Axis::Child && next.preds.iter().all(positionless) {
                        let merged = Step {
                            axis: Axis::Descendant,
                            test: next.test.clone(),
                            preds: next.preds.clone(),
                        };
                        let (r, f) = self.step(&nodes, flat, &merged)?;
                        nodes = r;
                        flat = f && nodes.len() <= 1;
                        i += 2;
                        continue;
                    }
                }
            }
            let (r, f) = self.step(&nodes, flat, s)?;
            nodes = r;
            flat = f;
            i += 1;
        }
        Ok(nodes)
    }

    fn eval(&self, e: &Expr, f: Focus) -> Result<Value, XPathError> {
        Ok(match e {
            Expr::Lit(s) => Value::Str(s.clone()),
            Expr::Num(n) => Value::Num(*n),
            Expr::Neg(x) => Value::Num(-self.to_num(&self.eval(x, f)?)),
            Expr::Path(abs, steps) => {
                let start = if *abs { self.top(f.node) } else { f.node };
                Value::Nodes(self.steps(vec![start], true, steps)?)
            }
            Expr::Filter(prim, preds, steps) => {
                let Value::Nodes(mut ns) = self.eval(prim, f)? else {
                    return Err(XPathError::NotNodeSet);
                };
                for p in preds {
                    ns = self.filter(ns, p)?;
                }
                Value::Nodes(self.steps(ns, false, steps)?)
            }
            Expr::Union(a, b) => {
                let (Value::Nodes(mut x), Value::Nodes(y)) = (self.eval(a, f)?, self.eval(b, f)?)
                else {
                    return Err(XPathError::NotNodeSet);
                };
                x.extend(y);
                self.sort_dedup(&mut x);
                Value::Nodes(x)
            }
            Expr::Bin(op, a, b) => match *op {
                "or" => {
                    Value::Bool(self.to_bool(&self.eval(a, f)?) || self.to_bool(&self.eval(b, f)?))
                }
                "and" => {
                    Value::Bool(self.to_bool(&self.eval(a, f)?) && self.to_bool(&self.eval(b, f)?))
                }
                "=" | "!=" | "<" | ">" | "<=" | ">=" => {
                    let (x, y) = (self.eval(a, f)?, self.eval(b, f)?);
                    Value::Bool(self.compare(op, &x, &y))
                }
                _ => {
                    let x = self.to_num(&self.eval(a, f)?);
                    let y = self.to_num(&self.eval(b, f)?);
                    Value::Num(match *op {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        "div" => x / y,
                        _ => x % y,
                    })
                }
            },
            Expr::Func(name, args) => self.call(name, args, f)?,
        })
    }

    fn cmp_atoms(&self, op: &str, x: &Value, y: &Value) -> bool {
        if matches!(op, "=" | "!=") {
            let eq = if matches!(x, Value::Bool(_)) || matches!(y, Value::Bool(_)) {
                self.to_bool(x) == self.to_bool(y)
            } else if matches!(x, Value::Num(_)) || matches!(y, Value::Num(_)) {
                self.to_num(x) == self.to_num(y)
            } else {
                self.to_str(x) == self.to_str(y)
            };
            return if op == "=" { eq } else { !eq };
        }
        let (a, b) = (self.to_num(x), self.to_num(y));
        match op {
            "<" => a < b,
            ">" => a > b,
            "<=" => a <= b,
            _ => a >= b,
        }
    }

    fn compare(&self, op: &str, x: &Value, y: &Value) -> bool {
        match (x, y) {
            (Value::Nodes(xs), Value::Nodes(ys)) => {
                let ysv: Vec<Value> = ys
                    .iter()
                    .map(|&n| Value::Str(self.string_value(n)))
                    .collect();
                xs.iter().any(|&a| {
                    let av = Value::Str(self.string_value(a));
                    ysv.iter().any(|bv| self.cmp_atoms(op, &av, bv))
                })
            }
            (Value::Nodes(xs), other) => match other {
                Value::Bool(_) => self.cmp_atoms(op, &Value::Bool(!xs.is_empty()), other),
                _ => xs
                    .iter()
                    .any(|&a| self.cmp_atoms(op, &Value::Str(self.string_value(a)), other)),
            },
            (other, Value::Nodes(ys)) => match other {
                Value::Bool(_) => self.cmp_atoms(op, other, &Value::Bool(!ys.is_empty())),
                _ => ys
                    .iter()
                    .any(|&b| self.cmp_atoms(op, other, &Value::Str(self.string_value(b)))),
            },
            _ => self.cmp_atoms(op, x, y),
        }
    }

    fn call(&self, name: &str, args: &[Expr], f: Focus) -> Result<Value, XPathError> {
        let arg = |i: usize| -> Result<Value, XPathError> { self.eval(&args[i], f) };
        let s_arg = |i: usize| -> Result<String, XPathError> {
            if i < args.len() {
                Ok(self.to_str(&arg(i)?))
            } else {
                Ok(self.string_value(f.node))
            }
        };
        let node_arg = |i: usize| -> Result<Option<NodeId>, XPathError> {
            if i < args.len() {
                match arg(i)? {
                    Value::Nodes(n) => Ok(n.first().copied()),
                    _ => Err(XPathError::NotNodeSet),
                }
            } else {
                Ok(Some(f.node))
            }
        };
        Ok(match name {
            "last" => Value::Num(f.size as f64),
            "position" => Value::Num(f.pos as f64),
            "count" => match arg(0)? {
                Value::Nodes(n) => Value::Num(n.len() as f64),
                _ => return Err(XPathError::NotNodeSet),
            },
            "local-name" => Value::Str(
                node_arg(0)?
                    .map(|n| self.local_or_empty(n))
                    .unwrap_or_default(),
            ),
            "name" => Value::Str(
                node_arg(0)?
                    .map(|n| self.name_or_empty(n))
                    .unwrap_or_default(),
            ),
            "namespace-uri" => Value::Str(
                node_arg(0)?
                    .map(|n| self.doc.namespace_uri(n).to_string())
                    .unwrap_or_default(),
            ),
            "string" => Value::Str(s_arg(0)?),
            "concat" => {
                let mut s = String::new();
                for i in 0..args.len() {
                    s.push_str(&s_arg(i)?);
                }
                Value::Str(s)
            }
            "starts-with" => Value::Bool(s_arg(0)?.starts_with(&s_arg(1)?)),
            "contains" => Value::Bool(s_arg(0)?.contains(&s_arg(1)?)),
            "substring-before" => {
                let (a, b) = (s_arg(0)?, s_arg(1)?);
                Value::Str(a.find(&b).map(|i| a[..i].to_string()).unwrap_or_default())
            }
            "substring-after" => {
                let (a, b) = (s_arg(0)?, s_arg(1)?);
                Value::Str(
                    a.find(&b)
                        .map(|i| a[i + b.len()..].to_string())
                        .unwrap_or_default(),
                )
            }
            "substring" => {
                let s: Vec<char> = s_arg(0)?.chars().collect();
                let start = xp_round(self.to_num(&arg(1)?));
                let end = if args.len() > 2 {
                    start + xp_round(self.to_num(&arg(2)?))
                } else {
                    f64::INFINITY
                };
                let r: String = s
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| {
                        let p = (*i + 1) as f64;
                        p >= start && p < end
                    })
                    .map(|(_, c)| *c)
                    .collect();
                Value::Str(r)
            }
            "string-length" => Value::Num(s_arg(0)?.encode_utf16().count() as f64),
            "normalize-space" => Value::Str(
                s_arg(0)?
                    .split([' ', '\t', '\n', '\r'])
                    .filter(|w| !w.is_empty())
                    .collect::<Vec<_>>()
                    .join(" "),
            ),
            "translate" => {
                let (s, from, to) = (s_arg(0)?, s_arg(1)?, s_arg(2)?);
                let from: Vec<char> = from.chars().collect();
                let to: Vec<char> = to.chars().collect();
                Value::Str(
                    s.chars()
                        .filter_map(|c| match from.iter().position(|&x| x == c) {
                            Some(i) => to.get(i).copied(),
                            None => Some(c),
                        })
                        .collect(),
                )
            }
            "boolean" => Value::Bool(self.to_bool(&arg(0)?)),
            "not" => Value::Bool(!self.to_bool(&arg(0)?)),
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "number" => Value::Num(if args.is_empty() {
                string_to_number(&self.string_value(f.node))
            } else {
                self.to_num(&arg(0)?)
            }),
            "sum" => match arg(0)? {
                Value::Nodes(n) => Value::Num(
                    n.iter()
                        .map(|&x| string_to_number(&self.string_value(x)))
                        .sum(),
                ),
                _ => return Err(XPathError::NotNodeSet),
            },
            "floor" => Value::Num(self.to_num(&arg(0)?).floor()),
            "ceiling" => Value::Num(self.to_num(&arg(0)?).ceil()),
            "round" => Value::Num(xp_round(self.to_num(&arg(0)?))),
            "lang" => {
                let want = s_arg(0)?.to_lowercase();
                let mut cur = Some(f.node);
                let mut found = None;
                while let Some(c) = cur {
                    if let Some(l) = self.doc.attr(c, "xml:lang") {
                        found = Some(l.to_lowercase());
                        break;
                    }
                    cur = self.doc.nodes[c.0].parent;
                }
                Value::Bool(found.is_some_and(|l| l == want || l.starts_with(&format!("{want}-"))))
            }
            _ => return Err(XPathError::UnknownFunction(name.to_string())),
        })
    }

    fn local_or_empty(&self, n: NodeId) -> String {
        match self.doc.kind(n) {
            Kind::Element { .. } | Kind::Attribute { .. } | Kind::Pi { .. } => {
                self.doc.local_name(n).to_string()
            }
            _ => String::new(),
        }
    }

    fn name_or_empty(&self, n: NodeId) -> String {
        match self.doc.kind(n) {
            Kind::Element { .. } | Kind::Attribute { .. } | Kind::Pi { .. } => {
                self.doc.name(n).to_string()
            }
            _ => String::new(),
        }
    }
}

fn xp_round(x: f64) -> f64 {
    if x.is_nan() || x.is_infinite() {
        x
    } else {
        (x + 0.5).floor()
    }
}

/// True if a predicate cannot depend on context position/size.
fn positionless(e: &Expr) -> bool {
    fn walk(e: &Expr) -> bool {
        match e {
            Expr::Func(n, a) => n != "position" && n != "last" && a.iter().all(walk),
            Expr::Bin(_, a, b) | Expr::Union(a, b) => walk(a) && walk(b),
            Expr::Neg(a) => walk(a),
            _ => true,
        }
    }
    let boolish = match e {
        Expr::Bin(op, ..) => matches!(*op, "or" | "and" | "=" | "!=" | "<" | ">" | "<=" | ">="),
        Expr::Path(..) | Expr::Union(..) | Expr::Lit(_) => true,
        Expr::Func(n, _) => matches!(
            n.as_str(),
            "not" | "boolean" | "contains" | "starts-with" | "true" | "false" | "lang"
        ),
        _ => false,
    };
    boolish && walk(e)
}

/// Evaluates `expr` with `ctx` as context node; it must yield a node-set.
pub fn select(
    doc: &Document,
    ctx: NodeId,
    expr: &str,
    ns: &Namespaces,
) -> Result<Vec<NodeId>, XPathError> {
    match evaluate(doc, ctx, expr, ns)? {
        Value::Nodes(n) => Ok(n),
        _ => Err(XPathError::NotNodeSet),
    }
}

/// `XPathNavigator.Evaluate`.
pub fn evaluate(
    doc: &Document,
    ctx: NodeId,
    expr: &str,
    ns: &Namespaces,
) -> Result<Value, XPathError> {
    let e = parse(expr)?;
    let c = Ctx { doc, ns };
    c.eval(
        &e,
        Focus {
            node: ctx,
            pos: 1,
            size: 1,
        },
    )
}
