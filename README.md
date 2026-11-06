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

More crates (`sbr-core`, `sbr-cli`, `sbr-gui`) are added as the port progresses.

## Building

```sh
cargo build --release
cargo test
```

## License

MIT. The original Simple Bible Reader is MIT-licensed by yesudas; see [NOTICE](NOTICE).
