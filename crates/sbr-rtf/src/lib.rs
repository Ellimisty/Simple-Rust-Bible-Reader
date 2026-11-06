//! Port of RtfPipe 1.x as bundled with Simple Bible Reader 5.6.2.
//! Not ported yet: every function returns `Error::NotPorted`.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("RTF conversion is not ported yet")]
    NotPorted,
    #[error("{0}")]
    Other(String),
}

/// `RtfPipe.Rtf.ToHtml`.
pub fn to_html(_rtf: &str) -> Result<String, Error> {
    Err(Error::NotPorted)
}

/// Plain text of an RTF document, as `RichTextBox.Text` returns it
/// (`FormatUtil.convertRtfToHtmlSlowMethod`).
pub fn to_text(_rtf: &str) -> Result<String, Error> {
    Err(Error::NotPorted)
}
