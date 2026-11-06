# Porting notes

## Source of truth

The port is made from `SimpleBibleReader_src_5_6_2.zip` in the upstream repository. That
repository's `installers/Simple_Bible_Reader_5_6_2.exe` (SHA-256
`29325a6155a90504b8ecced55ca898fae6097fb59d6315972dc6bc4b9b9808d2`) is a 7-Zip SFX
containing the .NET build of that source, and it is used as the reference for behaviour.

## Architecture of the original

Every Bible format converts to and from one in-memory Zefania-style `XmlDocument`
(`XMLBIBLE/BIBLEBOOK[@bnumber]/CHAPTER[@cnumber]/VERS[@vnumber]`) held in
`BibleFormat.BibleXmlDocument`. Commentaries, dictionaries and books use the same pattern
with their own documents. The GUI renders verses as HTML in WebView2 using the CSS in
`Themes/`.

Byte-identical exports therefore depend on reproducing `System.Xml` exactly, which is why
`sbr-xml` exists instead of using a generic XML crate. Behaviour it reproduces:

- `Load`/`LoadXml` drop whitespace-only text nodes unless `PreserveWhitespace` (or
  `xml:space="preserve"`); `InnerXml` assignments keep them.
- No end-of-line or attribute-value normalization on load (`XmlTextReader.Normalization`
  is false).
- `OuterXml` escaping rules for text and attributes, `<E />` vs `<E></E>` (an element that
  ever had children keeps a full end tag), namespace declarations added on output.
- `Save` indentation as `XmlTextWriter` with `Formatting.Indented`.

## Crate mapping

| C# | Rust |
|---|---|
| `System.Xml` | `sbr-xml` |
| `BibleFormat`, `CommentaryFormat`, `DictionaryFormat`, `BookFormat`, `FormatUtil`, `GlobalMemory`, `*Formats/` | `sbr-core` |
| `Sword/`, `SwordVersification.cs` | `sbr-core::sword` |
| `RtfPipe/` | ported module |
| `zip2/`, `zlib/`, `Ionic.Zlib/` | `zip`, `flate2` crates |
| `Twofish.cs` | `twofish` crate |
| `Unidecode/` | ported tables |
| `System.Data.SQLite` | `rusqlite` (bundled SQLite) |
| WinForms + WebView2 UI | `sbr-gui` (`tao` + `wry`, reusing the HTML/CSS themes) |

## Verification

The original assembly runs headless under Mono on Linux. A small C# harness calls
`FormatUtil.Init()`, `BibleFormat.BibleFactory(in, idx).Load()` and
`BibleFormat.doExportBible(out, idx).ExportBible(out, idx)`; Rust output is compared to it
byte for byte. Known Mono limitations: `System.Speech` needs a stub assembly, OleDb
(Access `.bbl`/EasySlides) is not implemented, and Mono writes `\n` where .NET on Windows
writes `\r\n` in a few places.

The `sbr-xml` round trip of the 16 MB KJV+ Zefania file is byte-identical to the original's
Zefania export.

## Intentional deviations

- VPL, Crosswire style `Book.Chapter.Verse text`: the original appended the
  dot-separated fields after the verse number to the verse text a second
  time; the port keeps only the text after the first space.

## Running the CLI

```sh
cargo run --release -p sbr-cli -- convert [--newline lf|crlf] <input> <input-format|0> <output> <output-format>
```

Format numbers are the original filter indices (`1` Zefania, `12` VPL, ...);
`0` autodetects the input. `crates/sbr-core/tests/differential.rs` checks the
port against outputs captured from the original under Mono (`--newline lf`).

## Reference outputs from the original program

`tools/original-harness/setup.sh [dir]` downloads the upstream 5.6.2 installer
(SHA-256 checked), unpacks it and builds `harness.exe`, which runs the
original `BibleFactory` → `Load` → `doExportBible` → `ExportBible` pipeline
headless under Mono:

```sh
tools/original-harness/setup.sh ~/sbr-original
mono ~/sbr-original/harness.exe x <input> <input-format|0> <output> <output-format>
```

Save its outputs under `crates/sbr-core/tests/fixtures/expected/` and add a
case to `tests/differential.rs`. Mono differences: `Environment.NewLine` is
`\n`, `Encoding.Default` is UTF-8 (Windows-1252 on .NET Framework), and
OleDb (Access `.mdb`) is unavailable.

## Layout for parallel porting

Every Bible format id is wired in `bible_format.rs` to `formats/<name>.rs`;
unported modules return `Error::Unsupported`. Commentary, dictionary and book
formats live under `src/commentary/`, `src/dictionary/` and `src/book/`;
RtfPipe is `crates/sbr-rtf` and the SWORD engine `crates/sbr-sword`. Per-family
notes (deviations, original bugs, Mono limitations) go in
`docs/porting/<family>.md`.
