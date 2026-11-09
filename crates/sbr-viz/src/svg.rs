//! Minimal SVG writer shared by the charts.

use crate::theme::{Theme, SANS, SERIF};
use std::fmt::Write;

pub struct Svg {
    pub buf: String,
    pub w: f64,
    pub h: f64,
}

pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

/// Compact number for coordinates.
pub fn n(x: f64) -> String {
    let s = format!("{x:.1}");
    match s.strip_suffix(".0") {
        Some(i) if i != "-0" => i.to_owned(),
        Some(_) => "0".to_owned(),
        None => s,
    }
}

/// `12345` → `12,345`.
pub fn thousands(x: usize) -> String {
    let s = x.to_string();
    let mut o = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            o.push(',');
        }
        o.push(c);
    }
    o
}

impl Svg {
    pub fn new(w: f64, h: f64, theme: &Theme, title: &str) -> Svg {
        let mut buf = String::new();
        let _ = write!(
            buf,
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
             width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"{sans}\">\n\
             <title>{title}</title>\n\
             <rect width=\"100%\" height=\"100%\" fill=\"{bg}\"/>\n",
            w = n(w),
            h = n(h),
            sans = esc(SANS),
            title = esc(title),
            bg = theme.bg,
        );
        Svg { buf, w, h }
    }

    pub fn raw(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    pub fn text(&mut self, x: f64, y: f64, s: &str, attrs: &str) {
        let _ = writeln!(
            self.buf,
            "<text x=\"{}\" y=\"{}\" {attrs}>{}</text>",
            n(x),
            n(y),
            esc(s)
        );
    }

    /// Title block in the top-left corner.
    pub fn heading(&mut self, theme: &Theme, x: f64, y: f64, title: &str, subtitle: &str) {
        self.text(
            x,
            y,
            title,
            &format!(
                "font-family=\"{}\" font-size=\"34\" fill=\"{}\" letter-spacing=\"0.2\"",
                esc(SERIF),
                theme.fg
            ),
        );
        self.text(
            x,
            y + 28.0,
            subtitle,
            &format!("font-size=\"14\" fill=\"{}\"", theme.muted),
        );
    }

    /// Credit line along the bottom edge.
    pub fn footer(&mut self, theme: &Theme, credit: &str) {
        let (w, h) = (self.w, self.h);
        let _ = writeln!(
            self.buf,
            "<line x1=\"40\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{}\" stroke-width=\"1\"/>",
            theme.rule,
            y = n(h - 44.0),
            x2 = n(w - 40.0)
        );
        self.text(
            40.0,
            h - 22.0,
            credit,
            &format!("font-size=\"11\" fill=\"{}\"", theme.muted),
        );
        self.text(
            w - 40.0,
            h - 22.0,
            "Simple Rust Bible Reader",
            &format!(
                "font-size=\"11\" fill=\"{}\" text-anchor=\"end\" letter-spacing=\"1.2\"",
                theme.muted
            ),
        );
    }

    pub fn finish(mut self) -> String {
        self.buf.push_str("</svg>\n");
        self.buf
    }
}

/// Moves sorted label positions apart to at least `gap`, keeping them in
/// `lo..=hi` and as close to their anchors as possible.
pub fn spread(pos: &mut [f64], gap: f64, lo: f64, hi: f64) {
    if pos.is_empty() {
        return;
    }
    let gap = gap.min((hi - lo) / (pos.len().max(2) - 1) as f64);
    for _ in 0..4 {
        for i in 0..pos.len() {
            let min = if i == 0 { lo } else { pos[i - 1] + gap };
            pos[i] = pos[i].max(min);
        }
        for i in (0..pos.len()).rev() {
            let max = if i + 1 == pos.len() {
                hi
            } else {
                pos[i + 1] - gap
            };
            pos[i] = pos[i].min(max);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(n(1.0), "1");
        assert_eq!(n(1.25), "1.2");
        assert_eq!(n(-0.01), "0");
        assert_eq!(thousands(344799), "344,799");
        assert_eq!(thousands(12), "12");
        let mut p = [0.0, 1.0, 2.0, 50.0];
        spread(&mut p, 10.0, 0.0, 100.0);
        assert_eq!(p, [0.0, 10.0, 20.0, 50.0]);
        let mut p = [95.0, 99.0];
        spread(&mut p, 10.0, 0.0, 100.0);
        assert_eq!(p, [90.0, 100.0]);
    }
}
