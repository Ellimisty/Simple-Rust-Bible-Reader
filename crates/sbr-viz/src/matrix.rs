//! 66 × 66 book matrix; each cell is a miniature chapter-position heat map,
//! so parallel passages (Kings/Chronicles, the Synoptics) show as diagonals.

use crate::bible::{self, Genre, Layout, BOOKS};
use crate::svg::{esc, n, thousands, Svg};
use crate::theme::Theme;
use crate::xref::CrossRef;
use std::collections::HashMap;
use std::fmt::Write;

pub struct Options {
    pub min_votes: i32,
    /// Sub-bins per cell side; 0 draws plain cells.
    pub detail: usize,
    pub theme: Theme,
}

const CELL: f64 = 15.0;
const GAP: f64 = 4.0;

fn pos(book: u8) -> f64 {
    (book - 1) as f64 * CELL + Genre::of(book) as usize as f64 * GAP
}

pub fn render(refs: &[CrossRef], o: &Options) -> String {
    let t = &o.theme;
    let l = Layout::get();
    let d = o.detail;
    let mut cells = vec![0u32; BOOKS * BOOKS];
    let mut bins: HashMap<(usize, usize, usize), u32> = HashMap::new();
    let mut total = 0usize;
    for r in refs.iter().filter(|r| r.votes >= o.min_votes) {
        let (a, b) = (r.from.book as usize - 1, r.to.book as usize - 1);
        cells[a * BOOKS + b] += 1;
        total += 1;
        if d > 0 {
            let bin = |v: bible::Verse| {
                let f = l.offset_in_book(v) / l.book_verses(v.book) as f64;
                ((f * d as f64) as usize).min(d - 1)
            };
            *bins
                .entry((a * BOOKS + b, bin(r.from), bin(r.to)))
                .or_default() += 1;
        }
    }
    let max_cell = cells.iter().copied().max().unwrap_or(1).max(1) as f64;
    let max_bin = bins.values().copied().max().unwrap_or(1).max(1) as f64;

    let (ox, oy) = (150.0, 200.0);
    let side = pos(BOOKS as u8) + CELL;
    let (w, h) = (ox + side + 300.0, oy + side + 90.0);
    let mut s = Svg::new(w, h, t, "Cross-reference matrix");
    s.heading(
        t,
        40.0,
        64.0,
        "The Cross-Reference Matrix",
        &format!(
            "{} references between the 66 books \u{00b7} rows cite, columns are cited",
            thousands(total)
        ),
    );

    // Genre strips and book labels.
    for g in Genre::ALL {
        let r = g.books();
        let (a, b) = (pos(*r.start()), pos(*r.end()) + CELL);
        let c = t.genre(g);
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"4\" rx=\"2\" fill=\"{c}\"/>\
             <rect x=\"{}\" y=\"{}\" width=\"4\" height=\"{}\" rx=\"2\" fill=\"{c}\"/>",
            n(ox + a),
            n(oy - 52.0),
            n(b - a),
            n(ox - 58.0),
            n(oy + a),
            n(b - a)
        );
    }
    for b in 1..=BOOKS as u8 {
        let p = pos(b) + CELL / 2.0;
        let fill = t.muted;
        s.text(
            ox - 8.0,
            oy + p + 3.5,
            bible::abbr(b),
            &format!("font-size=\"10\" fill=\"{fill}\" text-anchor=\"end\""),
        );
        let _ = writeln!(
            s.buf,
            "<text transform=\"translate({} {}) rotate(-90)\" font-size=\"10\" fill=\"{fill}\">{}</text>",
            n(ox + p + 3.5),
            n(oy - 8.0),
            esc(bible::abbr(b))
        );
    }

    // Cells.
    let quad = |a: usize, b: usize| {
        if a == b {
            t.quadrants[3]
        } else {
            match (bible::is_nt(a as u8 + 1), bible::is_nt(b as u8 + 1)) {
                (false, false) => t.quadrants[0],
                (true, true) => t.quadrants[2],
                _ => t.quadrants[1],
            }
        }
    };
    let _ = writeln!(
        s.buf,
        "<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{w}\" fill=\"{}\" fill-opacity=\"{}\"/>",
        n(ox - 2.0),
        n(oy - 2.0),
        t.rule,
        if t.dark { "0.35" } else { "0.5" },
        w = n(side + 4.0)
    );
    for a in 0..BOOKS {
        for b in 0..BOOKS {
            let c = cells[a * BOOKS + b];
            let (x, y) = (ox + pos(b as u8 + 1), oy + pos(a as u8 + 1));
            let col = quad(a, b);
            let v = if c == 0 {
                0.0
            } else {
                (c as f64).ln_1p() / max_cell.ln_1p()
            };
            let base = if d > 0 {
                0.04 + 0.22 * v
            } else {
                0.05 + 0.95 * v
            };
            let _ = write!(
                s.buf,
                "<rect x=\"{}\" y=\"{}\" width=\"{c_}\" height=\"{c_}\" fill=\"{}\" fill-opacity=\"{:.3}\">\
                 <title>{} \u{2192} {}: {}</title></rect>",
                n(x),
                n(y),
                if c == 0 { t.bg } else { col },
                if c == 0 { 1.0 } else { base },
                esc(&bible::name(a as u8 + 1)),
                esc(&bible::name(b as u8 + 1)),
                thousands(c as usize),
                c_ = n(CELL - 1.0)
            );
        }
    }
    s.raw("\n");
    if d > 0 {
        let bw = (CELL - 1.0) / d as f64;
        let mut keys: Vec<_> = bins.iter().collect();
        keys.sort();
        for (&(cell, i, j), &c) in keys {
            let (a, b) = (cell / BOOKS, cell % BOOKS);
            let v = (c as f64).ln_1p() / max_bin.ln_1p();
            let _ = write!(
                s.buf,
                "<rect x=\"{}\" y=\"{}\" width=\"{bw}\" height=\"{bw}\" fill=\"{}\" fill-opacity=\"{:.2}\"/>",
                n(ox + pos(b as u8 + 1) + j as f64 * bw),
                n(oy + pos(a as u8 + 1) + i as f64 * bw),
                quad(a, b),
                0.12 + 0.88 * v,
                bw = format_args!("{bw:.2}")
            );
        }
        s.raw("\n");
    }
    // Testament divide.
    let div = pos(40) - GAP / 2.0;
    let _ = writeln!(
        s.buf,
        "<path d=\"M{} {}V{}M{} {}H{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"3 3\"/>",
        n(ox + div),
        n(oy - 4.0),
        n(oy + side + 4.0),
        n(ox - 4.0),
        n(oy + div),
        n(ox + side + 4.0),
        t.muted
    );

    // Legend.
    let lx = ox + side + 40.0;
    let mut ly = oy + 10.0;
    let head = format!(
        "font-size=\"11\" fill=\"{}\" letter-spacing=\"1.4\" font-weight=\"600\"",
        t.muted
    );
    s.text(lx, ly, "TESTAMENTS", &head);
    ly += 22.0;
    for (i, label) in [
        "Old \u{2192} Old",
        "Old \u{2194} New",
        "New \u{2192} New",
        "Within a book",
    ]
    .iter()
    .enumerate()
    {
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" rx=\"2\" fill=\"{}\"/>",
            n(lx),
            n(ly - 10.0),
            t.quadrants[i]
        );
        s.text(
            lx + 20.0,
            ly,
            label,
            &format!("font-size=\"12\" fill=\"{}\"", t.fg),
        );
        ly += 22.0;
    }
    ly += 18.0;
    s.text(lx, ly, "DENSITY", &head);
    ly += 14.0;
    for i in 0..10 {
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"18\" height=\"10\" fill=\"{}\" fill-opacity=\"{:.2}\"/>",
            n(lx + i as f64 * 18.0),
            n(ly),
            t.quadrants[0],
            0.12 + 0.88 * i as f64 / 9.0
        );
    }
    s.text(
        lx,
        ly + 26.0,
        "fewer",
        &format!("font-size=\"10\" fill=\"{}\"", t.muted),
    );
    s.text(
        lx + 180.0,
        ly + 26.0,
        "more (log)",
        &format!("font-size=\"10\" fill=\"{}\" text-anchor=\"end\"", t.muted),
    );
    ly += 62.0;
    s.text(lx, ly, "GENRES", &head);
    ly += 22.0;
    for g in Genre::ALL {
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"4\" rx=\"2\" fill=\"{}\"/>",
            n(lx),
            n(ly - 6.0),
            t.genre(g)
        );
        s.text(
            lx + 20.0,
            ly,
            g.name(),
            &format!("font-size=\"12\" fill=\"{}\"", t.fg),
        );
        ly += 20.0;
    }
    if d > 0 {
        ly += 18.0;
        for line in [
            "Each cell is split into a",
            &format!("{d} \u{00d7} {d} grid by position within"),
            "the citing and cited books.",
        ] {
            s.text(
                lx,
                ly,
                line,
                &format!("font-size=\"11\" fill=\"{}\"", t.muted),
            );
            ly += 16.0;
        }
    }

    s.footer(t, "Cross-references: OpenBible.info (CC BY)");
    s.finish()
}
