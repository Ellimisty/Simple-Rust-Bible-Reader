//! Outputs of the original Simple Bible Reader 5.6.2 (run headless under
//! Mono, so `Environment.NewLine` is `\n`) for a two-book KJV+ excerpt.

use sbr_core::{bible_format, global_memory};
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// Exports under the reference file's name: formats without stored metadata
/// derive the description/abbreviation from the output file name.
fn convert(input: &str, from: i32, to: i32, name: &str) -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("sbr_test_{}_{from}_{to}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join(name).to_string_lossy().into_owned();
    let mut b = bible_format::load(input, from).unwrap();
    bible_format::export(&mut b, &out, to).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    bytes
}

#[test]
fn matches_original() {
    global_memory::get_mut().newline = "\n".into();
    let src = fixture("kjvplus_ruth_jude.xml");
    let vpl = fixture("expected/out_12.vpl");
    let ub2 = fixture("expected/out_2.txt");
    let ub3 = fixture("expected/out_3.txt");
    let cases = [
        (src.as_str(), 1, 1, "xml", "expected/out_1.xml"),
        (src.as_str(), 1, 2, "txt", "expected/out_2.txt"),
        (src.as_str(), 1, 3, "txt", "expected/out_3.txt"),
        (ub2.as_str(), 2, 1, "xml", "expected/out_2_1.xml"),
        (ub3.as_str(), 3, 1, "xml", "expected/out_3_1.xml"),
        (ub2.as_str(), 0, 2, "txt", "expected/out_2_2.txt"),
        (ub3.as_str(), 3, 3, "txt", "expected/out_3_3.txt"),
        (src.as_str(), 0, 12, "vpl", "expected/out_12.vpl"),
        (vpl.as_str(), 12, 12, "vpl", "expected/out_12_12.vpl"),
        (vpl.as_str(), 0, 1, "xml", "expected/out_12_1.xml"),
    ];
    for (input, from, to, _ext, expected) in cases {
        let name = std::path::Path::new(expected).file_name().unwrap();
        let got = convert(input, from, to, name.to_str().unwrap());
        let want = std::fs::read(fixture(expected)).unwrap();
        assert!(
            got == want,
            "{input} ({from}) -> {to} differs from {expected}"
        );
    }
}

fn dump(path: &str) -> Vec<String> {
    let c = rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let mut out = Vec::new();
    for q in [
        "SELECT sql FROM sqlite_master ORDER BY name",
        "SELECT * FROM Details",
        "SELECT * FROM Bible",
    ] {
        let mut st = c.prepare(q).unwrap();
        let n = st.column_count();
        let mut rows = st.query([]).unwrap();
        while let Some(r) = rows.next().unwrap() {
            out.push(
                (0..n)
                    .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
                    .collect::<Vec<_>>()
                    .join("|"),
            );
        }
    }
    out
}

#[test]
fn mysword_matches_original() {
    let dir = std::env::temp_dir().join(format!("sbr_mysword_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir
        .join("out_18.bbl.mybible.gz")
        .to_string_lossy()
        .into_owned();
    let mut b = bible_format::load(&fixture("kjvplus_ruth_jude.xml"), 1).unwrap();
    bible_format::export(&mut b, &out, 18).unwrap();
    let db = &out[..out.len() - 3];
    assert_eq!(dump(db), dump(&fixture("expected/out_18.bbl.mybible")));

    let want = std::fs::read(fixture("expected/out_18_1.xml")).unwrap();
    let gz = dir.join("in.bbl.mybible.gz");
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(
        &mut enc,
        &std::fs::read(fixture("expected/out_18.bbl.mybible")).unwrap(),
    )
    .unwrap();
    std::fs::write(&gz, enc.finish().unwrap()).unwrap();
    for input in [
        fixture("expected/out_18.bbl.mybible"),
        gz.to_string_lossy().into_owned(),
    ] {
        assert!(
            convert(&input, 0, 1, "out_18_1.xml") == want,
            "{input} -> Zefania differs"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
