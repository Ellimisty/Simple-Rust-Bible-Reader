//! Loads an XML file and prints `OuterXml` (for differential checks against .NET).
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: roundtrip <file.xml>");
    let mut d = sbr_xml::Document::new();
    let t = std::time::Instant::now();
    d.load_file(&path).unwrap_or_else(|e| panic!("{e}"));
    eprintln!("loaded in {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let n = d.select_nodes(d.root(), "//VERS").unwrap().len();
    eprintln!("//VERS = {n} in {:?}", t.elapsed());
    print!("{}", d.outer_xml(d.root()));
}
