//! .NET text I/O conventions.

use std::path::Path;

/// `File.WriteAllText(path, s, Encoding.UTF8)` / `new StreamWriter(path, false, Encoding.UTF8)`: UTF-8 with BOM.
pub fn write_utf8_bom(path: impl AsRef<Path>, s: &str) -> std::io::Result<()> {
    let mut v = Vec::with_capacity(s.len() + 3);
    v.extend_from_slice(b"\xEF\xBB\xBF");
    v.extend_from_slice(s.as_bytes());
    std::fs::write(path, v)
}

/// `StreamReader(path, Encoding.Default)`: honours a BOM, otherwise UTF-8,
/// falling back to Windows-1252 (the usual Windows ANSI code page).
pub fn read_text_default(path: impl AsRef<Path>) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    if let Some((enc, bom)) = encoding_rs::Encoding::for_bom(&bytes) {
        return Ok(enc
            .decode_without_bom_handling(&bytes[bom..])
            .0
            .into_owned());
    }
    Ok(match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => encoding_rs::WINDOWS_1252
            .decode(e.as_bytes())
            .0
            .into_owned(),
    })
}

/// `StreamReader.ReadLine` splitting (`\n`, `\r\n` or `\r`).
pub fn read_lines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        if b[i] == b'\n' || b[i] == b'\r' {
            out.push(&s[start..i]);
            if b[i] == b'\r' && i + 1 < b.len() && b[i + 1] == b'\n' {
                i += 1;
            }
            start = i + 1;
        }
        i += 1;
    }
    if start < b.len() {
        out.push(&s[start..]);
    }
    out
}
