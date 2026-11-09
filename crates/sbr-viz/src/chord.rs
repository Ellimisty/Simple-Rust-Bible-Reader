//! Books around a circle, grouped by genre; ribbons join books that cite
//! each other, width ∝ share of each book's cross-references.

use crate::bible::{self, Genre, Layout, BOOKS};
use crate::svg::{esc, n, thousands, Svg};
use crate::theme::Theme;
use crate::xref::{undirected, CrossRef};
use std::f64::consts::{PI, TAU};
use std::fmt::Write;

pub struct Options {
    pub min_votes: i32,
    pub max_ribbons: usize,
    pub theme: Theme,
}

const BOOK_GAP: f64 = 0.0045;
const GENRE_GAP: f64 = 0.03;

/// Angular span `(start, end)` of each book, clockwise from 12 o'clock;
/// arc length ∝ √verses so short books stay visible.
fn spans() -> Vec<(f64, f64)> {
    let l = Layout::get();
    let weights: Vec<f64> = (1..=BOOKS as u8)
        .map(|b| (l.book_verses(b) as f64).sqrt())
        .collect();
    let gaps: f64 = (2..=BOOKS as u8)
        .map(|b| {
            if Genre::of(b) != Genre::of(b - 1) {
                GENRE_GAP
            } else {
                BOOK_GAP
            }
        })
        .sum::<f64>()
        + GENRE_GAP;
    let k = (TAU - gaps) / weights.iter().sum::<f64>();
    let mut a = GENRE_GAP / 2.0;
    let mut out = Vec::new();
    for (i, w) in weights.iter().enumerate() {
        let b = i as u8 + 1;
        if b > 1 {
            a += if Genre::of(b) != Genre::of(b - 1) {
                GENRE_GAP
            } else {
                BOOK_GAP
            };
        }
        out.push((a, a + w * k));
        a += w * k;
    }
    out
}

struct Geo {
    cx: f64,
    cy: f64,
}

impl Geo {
    fn p(&self, r: f64, a: f64) -> (f64, f64) {
        (self.cx + r * a.sin(), self.cy - r * a.cos())
    }
    fn pt(&self, r: f64, a: f64) -> String {
        let (x, y) = self.p(r, a);
        format!("{} {}", n(x), n(y))
    }
    fn arc(&self, r: f64, a0: f64, a1: f64, sweep: u8) -> String {
        let large = u8::from((a1 - a0).abs() > PI);
        format!("A{rr} {rr} 0 {large} {sweep} {}", self.pt(r, a1), rr = n(r))
    }
}

pub fn render(refs: &[CrossRef], o: &Options) -> String {
    let t = &o.theme;
    let mut m = vec![0u32; BOOKS * BOOKS];
    let mut flow = vec![0u32; BOOKS];
    let mut total = 0usize;
    for r in undirected(refs, o.min_votes) {
        let (a, b) = (r.from.book as usize - 1, r.to.book as usize - 1);
        if a == b {
            continue;
        }
        m[a.min(b) * BOOKS + a.max(b)] += 1;
        flow[a] += 1;
        flow[b] += 1;
        total += 1;
    }
    let mut pairs: Vec<(usize, usize, u32)> = (0..BOOKS)
        .flat_map(|a| (a + 1..BOOKS).map(move |b| (a, b)))
        .map(|(a, b)| (a, b, m[a * BOOKS + b]))
        .filter(|p| p.2 > 0)
        .collect();
    pairs.sort_by(|x, y| y.2.cmp(&x.2).then((x.0, x.1).cmp(&(y.0, y.1))));
    pairs.truncate(o.max_ribbons);

    let spans = spans();
    let mid = |b: usize| (spans[b].0 + spans[b].1) / 2.0;
    // Slot each ribbon end inside its book so ribbons don't twist.
    let mut slot = vec![(0.0, 0.0); pairs.len() * 2];
    for b in 0..BOOKS {
        let mut ends: Vec<(usize, usize, f64)> = Vec::new();
        for (i, &(x, y, _)) in pairs.iter().enumerate() {
            if x == b {
                ends.push((i * 2, y, 0.0));
            } else if y == b {
                ends.push((i * 2 + 1, x, 0.0));
            }
        }
        for e in &mut ends {
            e.2 = (mid(e.1) - mid(b)).rem_euclid(TAU);
        }
        ends.sort_by(|x, y| y.2.total_cmp(&x.2));
        let (s0, s1) = spans[b];
        let usable = (s1 - s0) * 0.94;
        let mut a = s0 + (s1 - s0) * 0.03;
        for (k, _, _) in ends {
            let w = usable * pairs[k / 2].2 as f64 / flow[b].max(1) as f64;
            slot[k] = (a, a + w);
            a += w;
        }
    }

    let (w, h) = (1600.0, 1640.0);
    let g = Geo {
        cx: w / 2.0,
        cy: 840.0,
    };
    let r = 540.0;
    let mut s = Svg::new(w, h, t, "Cross-reference chord diagram");
    s.heading(
        t,
        40.0,
        64.0,
        "Conversations Between Books",
        &format!(
            "The {} strongest book-to-book links among {} cross-references \u{00b7} ribbon width \
             = share of each book\u{2019}s references \u{00b7} arc length grows with verse count",
            thousands(pairs.len()),
            thousands(total)
        ),
    );

    let max_c = pairs.first().map_or(2, |p| p.2).max(2) as f64;
    let mut order: Vec<usize> = (0..pairs.len()).collect();
    order.sort_by_key(|&i| pairs[i].2);
    s.raw("<g>\n");
    for i in order {
        let (a, b, c) = pairs[i];
        let ((a0, a1), (b0, b1)) = (slot[i * 2], slot[i * 2 + 1]);
        let v = (c as f64).ln() / max_c.ln();
        // Colour by the book where the ribbon is relatively strongest.
        let src = if c as f64 / flow[a] as f64 >= c as f64 / flow[b] as f64 {
            a
        } else {
            b
        };
        let ctr = format!("{} {}", n(g.cx), n(g.cy));
        let _ = writeln!(
            s.buf,
            "<path d=\"M{}{}Q{ctr} {}{}Q{ctr} {}Z\" fill=\"{}\" fill-opacity=\"{:.2}\" \
             stroke=\"{}\" stroke-opacity=\"0.25\" stroke-width=\"0.4\"><title>{} \u{2194} {}: {}</title></path>",
            g.pt(r, a0),
            g.arc(r, a0, a1, 1),
            g.pt(r, b0),
            g.arc(r, b0, b1, 1),
            g.pt(r, a0),
            t.book(src as u8 + 1),
            if t.dark { 0.22 } else { 0.18 } + 0.5 * v,
            t.book(src as u8 + 1),
            esc(&bible::name(a as u8 + 1)),
            esc(&bible::name(b as u8 + 1)),
            thousands(c as usize)
        );
    }
    s.raw("</g>\n");

    // Book ring and labels.
    let (r1, r2) = (r + 4.0, r + 20.0);
    for (i, &(a0, a1)) in spans.iter().enumerate() {
        let b = i as u8 + 1;
        let _ = writeln!(
            s.buf,
            "<path d=\"M{}{}L{}{}Z\" fill=\"{}\"><title>{} \u{00b7} {} references</title></path>",
            g.pt(r1, a0),
            g.arc(r1, a0, a1, 1),
            g.pt(r2, a1),
            g.arc(r2, a1, a0, 0),
            t.book(b),
            esc(&bible::name(b)),
            thousands(flow[i] as usize)
        );
        let a = (a0 + a1) / 2.0;
        let (x, y) = g.p(r2 + 8.0, a);
        let deg = a.to_degrees();
        let (rot, anchor) = if a < PI {
            (deg - 90.0, "start")
        } else {
            (deg + 90.0, "end")
        };
        let _ = writeln!(
            s.buf,
            "<text transform=\"translate({} {}) rotate({})\" dy=\"0.35em\" font-size=\"11\" \
             fill=\"{}\" text-anchor=\"{anchor}\">{}</text>",
            n(x),
            n(y),
            n(rot),
            t.fg,
            esc(bible::abbr(b))
        );
    }

    // Genre bands with curved labels.
    let (rg, rt) = (r + 66.0, r + 80.0);
    let mut defs = String::from("<defs>");
    for (gi, gen) in Genre::ALL.iter().enumerate() {
        let books = gen.books();
        let (a0, a1) = (
            spans[*books.start() as usize - 1].0,
            spans[*books.end() as usize - 1].1,
        );
        let _ = writeln!(
            s.buf,
            "<path d=\"M{}{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"3\" stroke-linecap=\"round\"/>",
            g.pt(rg, a0 + 0.004),
            g.arc(rg, a0 + 0.004, a1 - 0.004, 1),
            t.genre(*gen)
        );
        let m = (a0 + a1) / 2.0;
        let bottom = m > PI / 2.0 && m < 3.0 * PI / 2.0;
        let d = if bottom {
            let rr = rt + 9.0;
            format!("M{}{}", g.pt(rr, a1), g.arc(rr, a1, a0, 0))
        } else {
            format!("M{}{}", g.pt(rt, a0), g.arc(rt, a0, a1, 1))
        };
        let _ = write!(defs, "<path id=\"genre{gi}\" d=\"{d}\"/>");
        let label = gen.name().to_uppercase();
        if (a1 - a0) * rt > label.len() as f64 * 9.0 {
            let _ = writeln!(
                s.buf,
                "<text font-size=\"12\" fill=\"{}\" letter-spacing=\"2\" font-weight=\"600\">\
                 <textPath xlink:href=\"#genre{gi}\" href=\"#genre{gi}\" startOffset=\"50%\" \
                 text-anchor=\"middle\">{}</textPath></text>",
                t.genre(*gen),
                esc(&label)
            );
        }
    }
    defs.push_str("</defs>\n");
    s.raw(&defs);

    s.footer(t, "Cross-references: OpenBible.info (CC BY)");
    s.finish()
}
