//! Expected strings come from running the same operations through
//! System.Xml under Mono.

use crate::xpath::Namespaces;
use crate::Document;

#[test]
fn empty_element_forms() {
    let mut d = Document::new();
    let r = d.create_element("R");
    d.append_child(d.root(), r);
    let a = d.create_element("A");
    d.set_inner_xml(a, "x <b>1</b> <i>2</i>\n  <u/>").unwrap();
    d.append_child(r, a);
    let b = d.create_element("B");
    d.set_inner_text(b, "");
    d.append_child(r, b);
    let c = d.create_element("C");
    let z = d.create_element("Z");
    d.append_child(c, z);
    d.remove_child(c, z);
    d.append_child(r, c);
    let e = d.create_element("E");
    d.set_inner_xml(e, "").unwrap();
    d.append_child(r, e);
    let f = d.create_element("F");
    d.set_inner_text(f, "a");
    d.set_inner_text(f, "b");
    d.append_child(r, f);
    assert_eq!(d.outer_xml(d.root()), "<R><A>x <b>1</b> <i>2</i>\n  <u /></A><B></B><C></C><E /><F>b</F></R>");
    assert_eq!(d.children(a).len(), 6);
}

#[test]
fn load_drops_insignificant_whitespace() {
    let d = Document::parse("<R a=\"x\ny\tz\">  <V> a </V> <W>p <b>q</b> <i>r</i></W>\n<X xml:space=\"preserve\"> <y/> </X></R>").unwrap();
    assert_eq!(d.outer_xml(d.root()), "<R a=\"x&#xA;y\tz\"><V> a </V><W>p <b>q</b><i>r</i></W><X xml:space=\"preserve\"> <y /> </X></R>");
}

#[test]
fn undeclared_entity_is_error() {
    let e = Document::parse("<R>&nbsp;</R>").unwrap_err();
    assert_eq!(e.message, "Reference to undeclared entity 'nbsp'.");
    assert_eq!((e.line, e.column), (1, 5));
}

#[test]
fn line_endings_and_escaping() {
    let d = Document::parse("<?xml version=\"1.0\"?>\r\n<R>\r\n  <V a=\"1\r\n2\">x\r\ny</V>\r\n  <W>a<!--c-->b<![CDATA[<z>]]>c</W>\r\n</R>").unwrap();
    assert_eq!(d.outer_xml(d.root()), "<?xml version=\"1.0\"?><R><V a=\"1&#xD;&#xA;2\">x\r\ny</V><W>a<!--c-->b<![CDATA[<z>]]>c</W></R>");
    let mut d = Document::new();
    let t = d.create_element("T");
    d.set_inner_text(t, "a\u{1}b\r\nc<&>\"'");
    d.set_attribute(t, "q", "a\u{1}b\r\nc<&>\"'\t");
    assert_eq!(d.outer_xml(t), "<T q=\"a&#x1;b&#xD;&#xA;c&lt;&amp;&gt;&quot;'\t\">a&#x1;b\r\nc&lt;&amp;&gt;\"'</T>");
}

#[test]
fn indented_save() {
    let d = Document::parse("<?xml version=\"1.0\"?>\r\n<R>\r\n  <V a=\"1\r\n2\">x\r\ny</V>\r\n  <W>a<!--c-->b<![CDATA[<z>]]>c</W>\r\n</R>").unwrap();
    assert_eq!(d.to_indented_string(d.root(), "\n"), "<?xml version=\"1.0\"?>\n<R>\n  <V a=\"1&#xD;&#xA;2\">x\r\ny</V>\n  <W>a<!--c-->b<![CDATA[<z>]]>c</W>\n</R>");
    let d = Document::parse("<A><B>t<i>x</i></B><C><D/><!--k--><E>u</E></C></A>").unwrap();
    assert_eq!(d.to_indented_string(d.root(), "\n"), "<A>\n  <B>t<i>x</i></B>\n  <C>\n    <D />\n    <!--k-->\n    <E>u</E>\n  </C>\n</A>");
}

#[test]
fn namespaces() {
    let mut d = Document::new();
    let n = d.create_element_ns("o:verse", "http://www.bibletechnologies.net/2003/OSIS/namespace");
    assert_eq!(d.outer_xml(n), "<o:verse xmlns:o=\"http://www.bibletechnologies.net/2003/OSIS/namespace\" />");
    let mut d = Document::parse("<osis xmlns=\"urn:x\"><verse osisID=\"a\">t</verse></osis>").unwrap();
    let mut m = Namespaces::new();
    m.add("o", "urn:x");
    assert_eq!(d.select_nodes_ns(d.root(), "//o:verse", &m).unwrap().len(), 1);
    assert_eq!(d.select_nodes(d.root(), "//verse").unwrap().len(), 0);
    let v = d.create_element("verse");
    let de = d.document_element().unwrap();
    d.append_child(de, v);
    assert_eq!(d.outer_xml(d.root()), "<osis xmlns=\"urn:x\"><verse osisID=\"a\">t</verse><verse xmlns=\"\" /></osis>");
}

#[test]
fn xpath_basics() {
    let d = Document::parse(
        "<XMLBIBLE><BIBLEBOOK bnumber='1' bname='Genesis'><CHAPTER cnumber='1'><VERS vnumber='1'>In</VERS><VERS vnumber='2'>And</VERS></CHAPTER><CHAPTER cnumber='2'><VERS vnumber='1'>Thus</VERS></CHAPTER></BIBLEBOOK><BIBLEBOOK bnumber='2'><CHAPTER cnumber='1'><VERS vnumber='1'>Now</VERS></CHAPTER></BIBLEBOOK></XMLBIBLE>",
    )
    .unwrap();
    let r = d.root();
    let t = |x: &str| -> Vec<String> { d.select_nodes(r, x).unwrap().into_iter().map(|n| d.inner_text(n)).collect() };
    assert_eq!(t("/XMLBIBLE/BIBLEBOOK[@bnumber='1']/CHAPTER[@cnumber='1']/VERS[@vnumber='2']"), ["And"]);
    assert_eq!(t("//VERS"), ["In", "And", "Thus", "Now"]);
    assert_eq!(t("//VERS[1]"), ["In", "Thus", "Now"]);
    assert_eq!(t("(//VERS)[last()]"), ["Now"]);
    assert_eq!(t("/XMLBIBLE/BIBLEBOOK/@bname"), ["Genesis"]);
    assert_eq!(t("//CHAPTER[count(VERS) > 1]/VERS[position()=2]"), ["And"]);
    assert_eq!(t("//VERS[contains(., 'h')] | //VERS[@vnumber=2]"), ["And", "Thus"]);
    assert_eq!(t("/XMLBIBLE/BIBLEBOOK[2]/CHAPTER/VERS/../../@bnumber"), ["2"]);
    assert_eq!(t("//VERS[@vnumber='2']/preceding-sibling::VERS"), ["In"]);
    assert_eq!(t("//BIBLEBOOK[not(@bname)]//VERS"), ["Now"]);
    let v = crate::xpath::evaluate(&d, r, "count(//VERS) * 2 + 1", &Namespaces::new()).unwrap();
    assert!(matches!(v, crate::xpath::Value::Num(n) if n == 9.0));
}
