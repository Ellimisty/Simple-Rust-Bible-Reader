//! Every chapter along one line, cross-references as arcs above it
//! (after Chris Harrison & Christoph Römhild, 2008).

use crate::bible::{self, Genre, Layout, BOOKS};
use crate::svg::{n, spread, thousands, Svg};
use crate::theme::Theme;
use crate::xref::{undirected, CrossRef};
use std::collections::HashMap;
use std::fmt::Write;

pub struct Options {
    pub min_votes: i32,
    /// Strongest chapter pairs to draw.
    pub max_arcs: usize,
    pub theme: Theme,
}

pub fn render(refs: &[CrossRef], o: &Options) -> String {
    let t = &o.theme;
    let l = Layout::get();
    let mut pairs: HashMap<(usize, usize), u32> = HashMap::new();
    let mut total = 0usize;
    for r in undirected(refs, o.min_votes) {
        let a = l.chapter_index(r.from.book, l.clamp_chapter(r.from));
        let b = l.chapter_index(r.to.book, l.clamp_chapter(r.to));
        if a != b {
            *pairs.entry((a.min(b), a.max(b))).or_default() += 1;
            total += 1;
        }
    }
    let mut arcs: Vec<_> = pairs.into_iter().collect();
    arcs.sort_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
    arcs.truncate(o.max_arcs);
    let max_w = arcs.first().map_or(2, |a| a.1).max(2) as f64;
    let min_w = arcs.last().map_or(1, |a| a.1) as f64;
    // Long arcs first, short arcs drawn over them; within that, weight.
    arcs.sort_by(|x, y| {
        (y.0 .1 - y.0 .0)
            .cmp(&(x.0 .1 - x.0 .0))
            .then(x.1.cmp(&y.1))
    });

    let (w, h) = (2600.0, 1400.0);
    let (mx, base) = (80.0, 1040.0);
    let chapters = l.total_chapters();
    let step = (w - 2.0 * mx) / chapters as f64;
    let x = |i: usize| mx + (i as f64 + 0.5) * step;
    let k = (base - 190.0) / ((w - 2.0 * mx) / 2.0);

    let mut s = Svg::new(w, h, t, "Cross-references of the Bible");
    s.heading(
        t,
        mx,
        74.0,
        "Every Chapter, Connected",
        &format!(
            "The {} strongest chapter-to-chapter links among {} cross-references \u{00b7} 1,189 \
             chapters, Genesis to Revelation \u{00b7} arc colour = distance",
            thousands(arcs.len()),
            thousands(total)
        ),
    );

    s.raw("<g fill=\"none\" stroke-linecap=\"round\">\n");
    let span = (chapters - 1) as f64;
    for ((a, b), c) in &arcs {
        let (x1, x2) = (x(*a), x(*b));
        let rx = (x2 - x1) / 2.0;
        let d = (b - a) as f64 / span;
        let v = ((*c as f64).ln() - min_w.ln()) / (max_w.ln() - min_w.ln()).max(1e-9);
        let _ = writeln!(
            s.buf,
            "<path d=\"M{} {}A{} {} 0 0 1 {} {}\" stroke=\"{}\" stroke-opacity=\"{:.2}\" stroke-width=\"{:.2}\"/>",
            n(x1),
            n(base),
            n(rx),
            n(rx * k),
            n(x2),
            n(base),
            t.ramp_at(d.powf(0.6)),
            if t.dark { 0.10 } else { 0.08 } + 0.5 * v,
            0.5 + 1.1 * v
        );
    }
    s.raw("</g>\n");

    // Chapter bars, length = verses.
    let max_v = l.verses.iter().flatten().copied().max().unwrap_or(1) as f64;
    for b in 1..=BOOKS as u8 {
        let c = t.book(b);
        let _ = write!(s.buf, "<g fill=\"{c}\">");
        for ch in 1..=l.chapters(b) as u16 {
            let i = l.chapter_index(b, ch);
            let hh = 6.0 + 110.0 * l.chapter_verses(b, ch) as f64 / max_v;
            let _ = write!(
                s.buf,
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                n(x(i) - step * 0.42),
                n(base + 4.0),
                format_args!("{:.2}", step * 0.84),
                n(hh)
            );
        }
        s.raw("</g>\n");
    }

    // Book labels, spread so short books stay legible.
    let label_y = base + 140.0;
    let anchors: Vec<f64> = (1..=BOOKS as u8)
        .map(|b| {
            let first = l.chapter_index(b, 1);
            (x(first) + x(first + l.chapters(b) - 1)) / 2.0
        })
        .collect();
    let mut pos = anchors.clone();
    spread(&mut pos, 13.0, mx, w - mx);
    for (i, (&a, &p)) in anchors.iter().zip(&pos).enumerate() {
        let b = i as u8 + 1;
        let _ = writeln!(
            s.buf,
            "<path d=\"M{} {}V{}L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.7\"/>\
             <text transform=\"translate({} {}) rotate(-90)\" font-size=\"11\" fill=\"{}\" \
             text-anchor=\"end\">{}</text>",
            n(a),
            n(base + 124.0),
            n(base + 128.0),
            n(p),
            n(label_y - 6.0),
            t.faint,
            n(p + 4.0),
            n(label_y),
            t.fg,
            bible::abbr(b)
        );
    }
    let gy = label_y + 60.0;
    for g in Genre::ALL {
        let r = g.books();
        let (x1, x2) = (
            x(l.chapter_index(*r.start(), 1)) - step / 2.0,
            x(l.chapter_index(*r.end(), l.chapters(*r.end()) as u16)) + step / 2.0,
        );
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"3\" rx=\"1.5\" fill=\"{}\"/>",
            n(x1 + 1.0),
            n(gy),
            n(x2 - x1 - 2.0),
            t.genre(g)
        );
        let fits = |l: &str| (l.chars().count() as f64) * 9.0 + 8.0 < x2 - x1;
        if let Some(label) = [g.name(), g.short_name()].into_iter().find(|l| fits(l)) {
            s.text(
                (x1 + x2) / 2.0,
                gy + 22.0,
                &label.to_uppercase(),
                &format!(
                    "font-size=\"11\" fill=\"{}\" text-anchor=\"middle\" letter-spacing=\"1.6\"",
                    t.muted
                ),
            );
        }
    }

    // Distance legend.
    let (lx, ly, lw) = (w - mx - 320.0, 52.0, 320.0);
    let mut defs = String::from("<defs><linearGradient id=\"dist\">");
    for i in 0..=10 {
        let f = i as f64 / 10.0;
        let _ = write!(
            defs,
            "<stop offset=\"{f}\" stop-color=\"{}\"/>",
            t.ramp_at(f.powf(0.6))
        );
    }
    defs.push_str("</linearGradient></defs>\n");
    s.raw(&defs);
    let _ = writeln!(
        s.buf,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"8\" rx=\"4\" fill=\"url(#dist)\"/>",
        n(lx),
        n(ly),
        n(lw)
    );
    let small = format!("font-size=\"11\" fill=\"{}\"", t.muted);
    s.text(lx, ly + 26.0, "neighbouring chapters", &small);
    s.text(
        lx + lw,
        ly + 26.0,
        "Genesis \u{2194} Revelation",
        &format!("{small} text-anchor=\"end\""),
    );

    s.footer(
        t,
        "Cross-references: OpenBible.info (CC BY)  \u{00b7}  Bar length = verses per chapter  \
         \u{00b7}  After Chris Harrison & Christoph R\u{00f6}mhild (2008)",
    );
    s.finish()
}
