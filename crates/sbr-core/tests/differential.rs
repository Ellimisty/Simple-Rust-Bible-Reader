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

fn convert(input: &str, from: i32, to: i32, ext: &str) -> Vec<u8> {
    let out =
        std::env::temp_dir().join(format!("sbr_test_{}_{from}_{to}.{ext}", std::process::id()));
    let out = out.to_string_lossy().into_owned();
    let mut b = bible_format::load(input, from).unwrap();
    bible_format::export(&mut b, &out, to).unwrap();
    let bytes = std::fs::read(&out).unwrap();
    let _ = std::fs::remove_file(&out);
    bytes
}

#[test]
fn matches_original() {
    global_memory::get_mut().newline = "\n".into();
    let src = fixture("kjvplus_ruth_jude.xml");
    let vpl = fixture("expected/out_12.vpl");
    let cases = [
        (src.as_str(), 1, 1, "xml", "expected/out_1.xml"),
        (src.as_str(), 0, 12, "vpl", "expected/out_12.vpl"),
        (vpl.as_str(), 12, 12, "vpl", "expected/out_12_12.vpl"),
        (vpl.as_str(), 0, 1, "xml", "expected/out_12_1.xml"),
    ];
    for (input, from, to, ext, expected) in cases {
        let got = convert(input, from, to, ext);
        let want = std::fs::read(fixture(expected)).unwrap();
        assert!(
            got == want,
            "{input} ({from}) -> {to} differs from {expected}"
        );
    }
}
