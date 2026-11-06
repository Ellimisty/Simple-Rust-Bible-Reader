//! A small DOM modelled on `System.Xml.XmlDocument`, which the original
//! Simple Bible Reader uses as its in-memory interchange format.
//!
//! The behaviour that matters for byte-identical exports is reproduced:
//! whitespace-only text nodes are dropped on load unless
//! `preserve_whitespace` is set, fragments assigned through
//! [`Document::set_inner_xml`] keep their whitespace, line endings are not
//! normalized, and elements that once had children serialize as `<E></E>`
//! while never-filled elements serialize as `<E />`.

mod parse;
mod write;
pub mod xpath;

use std::collections::HashMap;

pub use parse::ParseError;
pub use xpath::XPathError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeType {
    Document,
    Element,
    Attribute,
    Text,
    CData,
    Comment,
    Whitespace,
    SignificantWhitespace,
    ProcessingInstruction,
    XmlDeclaration,
    DocumentType,
}

#[derive(Clone, Debug)]
pub(crate) enum Kind {
    Document,
    Element {
        name: String,
        ns: String,
        attrs: Vec<NodeId>,
        /// `true` once the element must be written with an end tag even
        /// when it has no children (mirrors `XmlElement.IsEmpty == false`).
        full_end: bool,
    },
    Attribute {
        name: String,
        ns: String,
        value: String,
    },
    Text(String),
    CData(String),
    Comment(String),
    Whitespace(String),
    SignificantWhitespace(String),
    Pi {
        target: String,
        data: String,
    },
    XmlDecl {
        version: String,
        encoding: String,
        standalone: String,
    },
    DocType {
        name: String,
        public_id: Option<String>,
        system_id: Option<String>,
        subset: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) kind: Kind,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,
}

pub const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

#[derive(Clone, Debug)]
pub struct Document {
    pub(crate) nodes: Vec<Node>,
    pub preserve_whitespace: bool,
    pub(crate) entities: HashMap<String, String>,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Document {
            nodes: vec![Node {
                kind: Kind::Document,
                parent: None,
                children: Vec::new(),
            }],
            preserve_whitespace: false,
            entities: HashMap::new(),
        }
    }

    /// The document node itself.
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    pub(crate) fn alloc(&mut self, kind: Kind) -> NodeId {
        self.nodes.push(Node {
            kind,
            parent: None,
            children: Vec::new(),
        });
        NodeId(self.nodes.len() - 1)
    }

    pub(crate) fn kind(&self, n: NodeId) -> &Kind {
        &self.nodes[n.0].kind
    }

    // ----- loading -------------------------------------------------------

    /// `XmlDocument.LoadXml`: replaces the content of this document.
    pub fn load_xml(&mut self, xml: &str) -> Result<(), ParseError> {
        let preserve = self.preserve_whitespace;
        *self = Document::new();
        self.preserve_whitespace = preserve;
        let root = self.root();
        parse::parse_into(self, root, xml, false, true)
    }

    /// `XmlDocument.Load(stream)`: decodes using BOM / encoding declaration.
    pub fn load_bytes(&mut self, bytes: &[u8]) -> Result<(), ParseError> {
        let text = parse::decode(bytes)?;
        self.load_xml(&text)
    }

    pub fn load_file(&mut self, path: impl AsRef<std::path::Path>) -> Result<(), ParseError> {
        let bytes = std::fs::read(path).map_err(|e| ParseError::new(e.to_string(), 0, 0))?;
        self.load_bytes(&bytes)
    }

    pub fn parse(xml: &str) -> Result<Document, ParseError> {
        let mut d = Document::new();
        d.load_xml(xml)?;
        Ok(d)
    }

    // ----- creation ------------------------------------------------------

    pub fn create_element(&mut self, name: &str) -> NodeId {
        self.create_element_ns(name, "")
    }

    pub fn create_element_ns(&mut self, qname: &str, ns: &str) -> NodeId {
        self.alloc(Kind::Element {
            name: qname.to_string(),
            ns: ns.to_string(),
            attrs: Vec::new(),
            full_end: false,
        })
    }

    pub fn create_text_node(&mut self, text: &str) -> NodeId {
        self.alloc(Kind::Text(text.to_string()))
    }

    pub fn create_cdata_section(&mut self, text: &str) -> NodeId {
        self.alloc(Kind::CData(text.to_string()))
    }

    pub fn create_comment(&mut self, text: &str) -> NodeId {
        self.alloc(Kind::Comment(text.to_string()))
    }

    pub fn create_whitespace(&mut self, text: &str) -> NodeId {
        self.alloc(Kind::Whitespace(text.to_string()))
    }

    pub fn create_processing_instruction(&mut self, target: &str, data: &str) -> NodeId {
        self.alloc(Kind::Pi {
            target: target.to_string(),
            data: data.to_string(),
        })
    }

    /// `XmlDocument.CreateXmlDeclaration(version, encoding, standalone)`;
    /// `None` and `""` are equivalent, as in .NET.
    pub fn create_xml_declaration(
        &mut self,
        version: &str,
        encoding: Option<&str>,
        standalone: Option<&str>,
    ) -> NodeId {
        self.alloc(Kind::XmlDecl {
            version: version.to_string(),
            encoding: encoding.unwrap_or("").to_string(),
            standalone: standalone.unwrap_or("").to_string(),
        })
    }

    pub fn create_attribute(&mut self, name: &str, value: &str) -> NodeId {
        let ns = attr_ns_for(name);
        self.alloc(Kind::Attribute {
            name: name.to_string(),
            ns,
            value: value.to_string(),
        })
    }

    // ----- tree navigation ----------------------------------------------

    pub fn node_type(&self, n: NodeId) -> NodeType {
        match self.kind(n) {
            Kind::Document => NodeType::Document,
            Kind::Element { .. } => NodeType::Element,
            Kind::Attribute { .. } => NodeType::Attribute,
            Kind::Text(_) => NodeType::Text,
            Kind::CData(_) => NodeType::CData,
            Kind::Comment(_) => NodeType::Comment,
            Kind::Whitespace(_) => NodeType::Whitespace,
            Kind::SignificantWhitespace(_) => NodeType::SignificantWhitespace,
            Kind::Pi { .. } => NodeType::ProcessingInstruction,
            Kind::XmlDecl { .. } => NodeType::XmlDeclaration,
            Kind::DocType { .. } => NodeType::DocumentType,
        }
    }

    pub fn is_element(&self, n: NodeId) -> bool {
        matches!(self.kind(n), Kind::Element { .. })
    }

    pub fn document_element(&self) -> Option<NodeId> {
        self.nodes[0]
            .children
            .iter()
            .copied()
            .find(|&c| self.is_element(c))
    }

    pub fn parent(&self, n: NodeId) -> Option<NodeId> {
        match self.kind(n) {
            Kind::Attribute { .. } => None,
            _ => self.nodes[n.0].parent,
        }
    }

    /// The element owning an attribute node.
    pub fn owner_element(&self, attr: NodeId) -> Option<NodeId> {
        match self.kind(attr) {
            Kind::Attribute { .. } => self.nodes[attr.0].parent,
            _ => None,
        }
    }

    pub fn children(&self, n: NodeId) -> &[NodeId] {
        &self.nodes[n.0].children
    }

    pub fn child_elements(&self, n: NodeId) -> impl Iterator<Item = NodeId> + '_ {
        self.children(n)
            .iter()
            .copied()
            .filter(move |&c| self.is_element(c))
    }

    pub fn has_child_nodes(&self, n: NodeId) -> bool {
        !self.nodes[n.0].children.is_empty()
    }

    pub fn first_child(&self, n: NodeId) -> Option<NodeId> {
        self.children(n).first().copied()
    }

    pub fn last_child(&self, n: NodeId) -> Option<NodeId> {
        self.children(n).last().copied()
    }

    pub(crate) fn index_in_parent(&self, n: NodeId) -> Option<(NodeId, usize)> {
        let p = self.parent(n)?;
        let i = self.nodes[p.0].children.iter().position(|&c| c == n)?;
        Some((p, i))
    }

    pub fn next_sibling(&self, n: NodeId) -> Option<NodeId> {
        let (p, i) = self.index_in_parent(n)?;
        self.nodes[p.0].children.get(i + 1).copied()
    }

    pub fn previous_sibling(&self, n: NodeId) -> Option<NodeId> {
        let (p, i) = self.index_in_parent(n)?;
        if i == 0 {
            None
        } else {
            Some(self.nodes[p.0].children[i - 1])
        }
    }

    /// `XmlNode.Name`.
    pub fn name(&self, n: NodeId) -> &str {
        match self.kind(n) {
            Kind::Document => "#document",
            Kind::Element { name, .. } | Kind::Attribute { name, .. } => name,
            Kind::Text(_) => "#text",
            Kind::CData(_) => "#cdata-section",
            Kind::Comment(_) => "#comment",
            Kind::Whitespace(_) => "#whitespace",
            Kind::SignificantWhitespace(_) => "#significant-whitespace",
            Kind::Pi { target, .. } => target,
            Kind::XmlDecl { .. } => "xml",
            Kind::DocType { name, .. } => name,
        }
    }

    pub fn local_name(&self, n: NodeId) -> &str {
        let name = self.name(n);
        match self.kind(n) {
            Kind::Element { .. } | Kind::Attribute { .. } => {
                name.rsplit_once(':').map(|(_, l)| l).unwrap_or(name)
            }
            _ => name,
        }
    }

    pub fn prefix(&self, n: NodeId) -> &str {
        match self.kind(n) {
            Kind::Element { name, .. } | Kind::Attribute { name, .. } => {
                name.split_once(':').map(|(p, _)| p).unwrap_or("")
            }
            _ => "",
        }
    }

    pub fn namespace_uri(&self, n: NodeId) -> &str {
        match self.kind(n) {
            Kind::Element { ns, .. } | Kind::Attribute { ns, .. } => ns,
            _ => "",
        }
    }

    /// `XmlNode.Value` (None for elements and the document).
    pub fn value(&self, n: NodeId) -> Option<String> {
        match self.kind(n) {
            Kind::Attribute { value, .. } => Some(value.clone()),
            Kind::Text(s)
            | Kind::CData(s)
            | Kind::Comment(s)
            | Kind::Whitespace(s)
            | Kind::SignificantWhitespace(s) => Some(s.clone()),
            Kind::Pi { data, .. } => Some(data.clone()),
            Kind::XmlDecl { .. } => Some(write::decl_value(self.kind(n))),
            _ => None,
        }
    }

    pub fn set_value(&mut self, n: NodeId, v: &str) {
        match &mut self.nodes[n.0].kind {
            Kind::Attribute { value, .. } => *value = v.to_string(),
            Kind::Text(s)
            | Kind::CData(s)
            | Kind::Comment(s)
            | Kind::Whitespace(s)
            | Kind::SignificantWhitespace(s) => *s = v.to_string(),
            Kind::Pi { data, .. } => *data = v.to_string(),
            _ => {}
        }
    }

    // ----- attributes ----------------------------------------------------

    pub fn attributes(&self, el: NodeId) -> &[NodeId] {
        match self.kind(el) {
            Kind::Element { attrs, .. } => attrs,
            _ => &[],
        }
    }

    pub fn attribute_node(&self, el: NodeId, name: &str) -> Option<NodeId> {
        self.attributes(el)
            .iter()
            .copied()
            .find(|&a| self.name(a) == name)
    }

    pub fn has_attribute(&self, el: NodeId, name: &str) -> bool {
        self.attribute_node(el, name).is_some()
    }

    /// `XmlElement.GetAttribute`: `""` when absent.
    pub fn get_attribute(&self, el: NodeId, name: &str) -> String {
        self.attr(el, name).unwrap_or_default().to_string()
    }

    pub fn attr(&self, el: NodeId, name: &str) -> Option<&str> {
        self.attribute_node(el, name).map(|a| match self.kind(a) {
            Kind::Attribute { value, .. } => value.as_str(),
            _ => unreachable!(),
        })
    }

    /// `XmlElement.SetAttribute(name, value)`.
    pub fn set_attribute(&mut self, el: NodeId, name: &str, value: &str) {
        if let Some(a) = self.attribute_node(el, name) {
            self.set_value(a, value);
            return;
        }
        let a = self.create_attribute(name, value);
        self.append_attribute(el, a);
    }

    /// `XmlAttributeCollection.Append`: replaces an attribute of the same name in place
    /// is not .NET behaviour; .NET removes the old one and appends the new one.
    pub fn append_attribute(&mut self, el: NodeId, attr: NodeId) {
        let name = self.name(attr).to_string();
        if let Some(old) = self.attribute_node(el, &name) {
            self.remove_attribute_node(el, old);
        }
        if let Some(old_owner) = self.nodes[attr.0].parent {
            self.remove_attribute_node(old_owner, attr);
        }
        self.nodes[attr.0].parent = Some(el);
        if let Kind::Element { attrs, .. } = &mut self.nodes[el.0].kind {
            attrs.push(attr);
        }
    }

    pub fn remove_attribute(&mut self, el: NodeId, name: &str) {
        if let Some(a) = self.attribute_node(el, name) {
            self.remove_attribute_node(el, a);
        }
    }

    pub fn remove_attribute_node(&mut self, el: NodeId, attr: NodeId) {
        if let Kind::Element { attrs, .. } = &mut self.nodes[el.0].kind {
            attrs.retain(|&a| a != attr);
        }
        self.nodes[attr.0].parent = None;
    }

    pub fn remove_all_attributes(&mut self, el: NodeId) {
        let attrs: Vec<NodeId> = self.attributes(el).to_vec();
        for a in attrs {
            self.remove_attribute_node(el, a);
        }
    }

    // ----- mutation ------------------------------------------------------

    fn detach(&mut self, n: NodeId) {
        if let Some(p) = self.nodes[n.0].parent.take() {
            if matches!(self.kind(n), Kind::Attribute { .. }) {
                if let Kind::Element { attrs, .. } = &mut self.nodes[p.0].kind {
                    attrs.retain(|&a| a != n);
                }
                return;
            }
            let ch = &mut self.nodes[p.0].children;
            ch.retain(|&c| c != n);
            if ch.is_empty() {
                self.mark_full_end(p);
            }
        }
    }

    fn mark_full_end(&mut self, el: NodeId) {
        if let Kind::Element { full_end, .. } = &mut self.nodes[el.0].kind {
            *full_end = true;
        }
    }

    /// `XmlNode.AppendChild`. Appending a document fragment is not modelled.
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) -> NodeId {
        self.detach(child);
        self.nodes[child.0].parent = Some(parent);
        self.nodes[parent.0].children.push(child);
        child
    }

    pub fn prepend_child(&mut self, parent: NodeId, child: NodeId) -> NodeId {
        self.detach(child);
        self.nodes[child.0].parent = Some(parent);
        self.nodes[parent.0].children.insert(0, child);
        child
    }

    pub fn insert_before(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> NodeId {
        let Some(r) = reference else {
            return self.append_child(parent, child);
        };
        self.detach(child);
        let i = self.nodes[parent.0]
            .children
            .iter()
            .position(|&c| c == r)
            .unwrap_or(self.nodes[parent.0].children.len());
        self.nodes[child.0].parent = Some(parent);
        self.nodes[parent.0].children.insert(i, child);
        child
    }

    pub fn insert_after(
        &mut self,
        parent: NodeId,
        child: NodeId,
        reference: Option<NodeId>,
    ) -> NodeId {
        let Some(r) = reference else {
            return self.prepend_child(parent, child);
        };
        self.detach(child);
        let i = self.nodes[parent.0]
            .children
            .iter()
            .position(|&c| c == r)
            .map(|i| i + 1)
            .unwrap_or(self.nodes[parent.0].children.len());
        self.nodes[child.0].parent = Some(parent);
        self.nodes[parent.0].children.insert(i, child);
        child
    }

    pub fn remove_child(&mut self, parent: NodeId, child: NodeId) -> NodeId {
        if self.nodes[child.0].parent == Some(parent) {
            self.detach(child);
        }
        child
    }

    pub fn replace_child(
        &mut self,
        parent: NodeId,
        new_child: NodeId,
        old_child: NodeId,
    ) -> NodeId {
        self.insert_before(parent, new_child, Some(old_child));
        self.remove_child(parent, old_child)
    }

    /// Removes all children (and, for elements, attributes): `XmlNode.RemoveAll`.
    pub fn remove_all(&mut self, n: NodeId) {
        self.remove_all_children(n);
        if self.is_element(n) {
            self.remove_all_attributes(n);
        }
    }

    pub fn remove_all_children(&mut self, n: NodeId) {
        let ch = std::mem::take(&mut self.nodes[n.0].children);
        let had = !ch.is_empty();
        for c in ch {
            self.nodes[c.0].parent = None;
        }
        if had {
            self.mark_full_end(n);
        }
    }

    /// `XmlNode.CloneNode(deep)`; the clone is detached.
    pub fn clone_node(&mut self, n: NodeId, deep: bool) -> NodeId {
        let mut kind = self.kind(n).clone();
        let attrs: Vec<NodeId> = self.attributes(n).to_vec();
        if let Kind::Element {
            attrs: a, full_end, ..
        } = &mut kind
        {
            a.clear();
            if !deep {
                *full_end = false;
            }
        }
        let c = self.alloc(kind);
        for a in attrs {
            let ac = self.clone_node(a, true);
            self.append_attribute(c, ac);
        }
        if deep {
            let ch: Vec<NodeId> = self.children(n).to_vec();
            for k in ch {
                let kc = self.clone_node(k, true);
                self.nodes[kc.0].parent = Some(c);
                self.nodes[c.0].children.push(kc);
            }
        }
        c
    }

    /// Copies a node (deeply) from another document: `XmlDocument.ImportNode`.
    pub fn import_node(&mut self, other: &Document, n: NodeId, deep: bool) -> NodeId {
        let mut kind = other.kind(n).clone();
        if let Kind::Element {
            attrs, full_end, ..
        } = &mut kind
        {
            attrs.clear();
            if !deep {
                *full_end = false;
            }
        }
        let c = self.alloc(kind);
        for &a in other.attributes(n) {
            let ac = self.import_node(other, a, true);
            self.append_attribute(c, ac);
        }
        if deep {
            for &k in other.children(n) {
                let kc = self.import_node(other, k, true);
                self.nodes[kc.0].parent = Some(c);
                self.nodes[c.0].children.push(kc);
            }
        }
        c
    }

    // ----- text ----------------------------------------------------------

    /// `XmlNode.InnerText` getter.
    pub fn inner_text(&self, n: NodeId) -> String {
        match self.kind(n) {
            Kind::Document | Kind::Element { .. } => {
                let mut s = String::new();
                self.collect_text(n, &mut s);
                s
            }
            _ => self.value(n).unwrap_or_default(),
        }
    }

    fn collect_text(&self, n: NodeId, out: &mut String) {
        for &c in self.children(n) {
            match self.kind(c) {
                Kind::Text(s)
                | Kind::CData(s)
                | Kind::Whitespace(s)
                | Kind::SignificantWhitespace(s) => out.push_str(s),
                Kind::Element { .. } => self.collect_text(c, out),
                _ => {}
            }
        }
    }

    /// `XmlElement.InnerText` setter.
    pub fn set_inner_text(&mut self, n: NodeId, text: &str) {
        match self.kind(n) {
            Kind::Element { .. } | Kind::Document => {
                let ch = self.children(n);
                if ch.len() == 1 && matches!(self.kind(ch[0]), Kind::Text(_)) {
                    let t = ch[0];
                    self.set_value(t, text);
                } else {
                    self.remove_all_children(n);
                    let t = self.create_text_node(text);
                    self.append_child(n, t);
                }
            }
            _ => self.set_value(n, text),
        }
    }

    /// `XmlNode.InnerXml` getter.
    pub fn inner_xml(&self, n: NodeId) -> String {
        let mut s = String::new();
        match self.kind(n) {
            Kind::Attribute { .. } => {
                write::escape_attr_into(&self.value(n).unwrap_or_default(), &mut s)
            }
            _ => {
                for &c in self.children(n) {
                    write::write_node(self, c, &mut s);
                }
            }
        }
        s
    }

    /// `XmlElement.InnerXml` setter. Whitespace in the fragment is kept.
    pub fn set_inner_xml(&mut self, n: NodeId, xml: &str) -> Result<(), ParseError> {
        let had = self.has_child_nodes(n);
        let ch = std::mem::take(&mut self.nodes[n.0].children);
        for c in ch {
            self.nodes[c.0].parent = None;
        }
        if had {
            self.mark_full_end(n);
        }
        if xml.is_empty() {
            return Ok(());
        }
        parse::parse_into(self, n, xml, true, false)
    }

    /// `XmlNode.OuterXml`.
    pub fn outer_xml(&self, n: NodeId) -> String {
        let mut s = String::new();
        if n == self.root() {
            for &c in self.children(n) {
                write::write_node(self, c, &mut s);
            }
        } else {
            write::write_node(self, n, &mut s);
        }
        s
    }

    /// Serialization used by `XmlDocument.Save` (indented unless
    /// `preserve_whitespace`), without encoding concerns.
    pub fn to_indented_string(&self, n: NodeId, newline: &str) -> String {
        write::indented(self, n, "  ", newline)
    }

    /// `XmlDocument.Save(path)`: UTF-8 with BOM, CRLF line breaks between
    /// indented nodes, matching .NET Framework on Windows.
    pub fn save_bytes(&self) -> Vec<u8> {
        let body = if self.preserve_whitespace {
            self.outer_xml(self.root())
        } else {
            self.to_indented_string(self.root(), "\r\n")
        };
        let mut out = vec![0xEF, 0xBB, 0xBF];
        out.extend_from_slice(body.as_bytes());
        out
    }

    // ----- namespaces ----------------------------------------------------

    /// Resolves a prefix in scope at `n` (`""` = default namespace).
    pub fn lookup_namespace(&self, mut n: NodeId, prefix: &str) -> Option<String> {
        if prefix == "xml" {
            return Some(XML_NS.to_string());
        }
        if prefix == "xmlns" {
            return Some(XMLNS_NS.to_string());
        }
        if let Kind::Attribute { .. } = self.kind(n) {
            n = self.nodes[n.0].parent?;
        }
        let want = if prefix.is_empty() {
            "xmlns".to_string()
        } else {
            format!("xmlns:{prefix}")
        };
        let mut cur = Some(n);
        while let Some(c) = cur {
            if let Some(v) = self.attr(c, &want) {
                return Some(v.to_string());
            }
            cur = self.nodes[c.0].parent;
        }
        if prefix.is_empty() {
            Some(String::new())
        } else {
            None
        }
    }

    // ----- XPath ---------------------------------------------------------

    pub fn select_nodes(&self, ctx: NodeId, expr: &str) -> Result<Vec<NodeId>, XPathError> {
        xpath::select(self, ctx, expr, &xpath::Namespaces::new())
    }

    pub fn select_single_node(
        &self,
        ctx: NodeId,
        expr: &str,
    ) -> Result<Option<NodeId>, XPathError> {
        Ok(self.select_nodes(ctx, expr)?.into_iter().next())
    }

    pub fn select_nodes_ns(
        &self,
        ctx: NodeId,
        expr: &str,
        ns: &xpath::Namespaces,
    ) -> Result<Vec<NodeId>, XPathError> {
        xpath::select(self, ctx, expr, ns)
    }

    pub fn select_single_node_ns(
        &self,
        ctx: NodeId,
        expr: &str,
        ns: &xpath::Namespaces,
    ) -> Result<Option<NodeId>, XPathError> {
        Ok(self.select_nodes_ns(ctx, expr, ns)?.into_iter().next())
    }

    /// All descendant elements with the given qualified name, in document
    /// order: `XmlNode.GetElementsByTagName` (`"*"` matches all).
    pub fn get_elements_by_tag_name(&self, n: NodeId, name: &str) -> Vec<NodeId> {
        let mut out = Vec::new();
        self.descendants(n, &mut |d, id| {
            if d.is_element(id) && (name == "*" || d.name(id) == name) {
                out.push(id);
            }
        });
        out
    }

    pub(crate) fn descendants(&self, n: NodeId, f: &mut dyn FnMut(&Document, NodeId)) {
        let mut stack: Vec<NodeId> = self.children(n).iter().rev().copied().collect();
        while let Some(c) = stack.pop() {
            f(self, c);
            stack.extend(self.children(c).iter().rev().copied());
        }
    }
}

pub(crate) fn attr_ns_for(name: &str) -> String {
    if name == "xmlns" || name.starts_with("xmlns:") {
        XMLNS_NS.to_string()
    } else if name.starts_with("xml:") {
        XML_NS.to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests;
