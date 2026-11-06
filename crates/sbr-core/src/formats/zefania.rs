//! Port of `ZefaniaBibleFormat.cs`.

use crate::bible_format::{Bible, ExportMeta, Result};

pub fn load(b: &mut Bible, filename: &str) -> Result<()> {
    b.doc.load_file(filename)?;
    let d = &mut b.doc;
    let r = d.root();
    if let Some(n) = d.select_single_node(r, "/XMLBIBLE/INFORMATION/identifier")? {
        b.abbreviation = d.inner_text(n);
    }
    if let Some(n) = d.select_single_node(r, "/XMLBIBLE/INFORMATION/description")? {
        b.description = d.inner_text(n);
    }
    renumber(d, |n| (n > 900).then_some(n - 811))
}

pub fn export(b: &mut Bible, _meta: &ExportMeta, filename: &str) -> Result<()> {
    let filename = if filename.ends_with(".xml") {
        filename.to_string()
    } else {
        format!("{filename}.xml")
    };
    renumber(&mut b.doc, |n| (n > 89).then_some(n + 811))?;
    let r = b.doc.root();
    crate::io::write_utf8_bom(filename, &b.doc.outer_xml(r))?;
    Ok(())
}

fn renumber(d: &mut sbr_xml::Document, f: impl Fn(i32) -> Option<i32>) -> Result<()> {
    let r = d.root();
    for book in d.select_nodes(r, "/XMLBIBLE/BIBLEBOOK")? {
        if let Some(a) = d.attribute_node(book, "bnumber") {
            if let Some(n) = d
                .value(a)
                .and_then(|v| crate::dotnet::int_parse(&v))
                .and_then(&f)
            {
                d.set_value(a, &n.to_string());
            }
        }
    }
    Ok(())
}
