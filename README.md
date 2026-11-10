# Simple Rust Bible Reader

A Rust port of [Simple Bible Reader](https://github.com/yesudas/simple-bible-reader) 5.6.2
(C#/.NET 4.8, WinForms + WebView2), a reader and converter for many Bible, commentary,
dictionary and book module formats.

The port follows the published 5.6.2 source file by file so that conversions produce the
same output as the original program. See [docs/PORTING.md](docs/PORTING.md) for the
architecture, the mapping from C# sources to Rust crates, and how outputs are verified
against the original executable.

## Workspace

| crate | purpose |
|---|---|
| `sbr-xml` | `System.Xml.XmlDocument`-compatible DOM: parser, `OuterXml`/`Save` serialization, XPath 1.0 |
| `sbr-core` | shared model, versification, Bible/commentary/dictionary/book formats, chapter renderer and themes |
| `sbr-rtf` | RtfPipe port (RTF to HTML/text) |
| `sbr-sword` | SWORD module engine (replaces `libsword.dll`/`CSSword.dll`) |
| `sbr-cli` | `sbr convert`: headless converter |
| `sbr-gui` | `simple-bible-reader`: desktop reader on `wry`/`tao` |

## Building

```sh
cargo build --release
cargo test
```

Run the reader (optionally with a Bible file to open):

```sh
cargo run --release -p sbr-gui -- path/to/bible.xml
```

On Linux the GUI needs the WebKitGTK 4.1 and GTK 3 development packages
(`libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev` on Debian/Ubuntu). Windows uses
WebView2 and macOS uses WKWebView.

## License

MIT. The original Simple Bible Reader is MIT-licensed by yesudas; see [NOTICE](NOTICE).
