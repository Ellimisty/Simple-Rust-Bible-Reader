//! Two books on parallel axes, chapters labelled with their titles, every
//! cross-reference between them drawn as a curve (e.g. Exodus ⟷ James).

use crate::bible::{self, Layout};
use crate::svg::{esc, n, spread, thousands, Svg};
use crate::theme::{mix, Theme, SERIF};
use crate::xref::{undirected, CrossRef};
use std::fmt::Write;

pub struct Options {
    pub left: u8,
    pub right: u8,
    pub min_votes: i32,
    pub theme: Theme,
}

struct Link {
    left: f64,
    right: f64,
    votes: i32,
    label: String,
}

fn verse_label(v: bible::Verse) -> String {
    format!("{} {}:{}", bible::abbr(v.book), v.chapter, v.verse)
}

fn links(refs: &[CrossRef], o: &Options) -> Vec<Link> {
    let l = Layout::get();
    let mut out = Vec::new();
    for r in undirected(refs, o.min_votes) {
        let (a, b, b_end) = if r.from.book == o.left && r.to.book == o.right {
            (r.from, r.to, r.to_end)
        } else if r.from.book == o.right && r.to.book == o.left && o.left != o.right {
            (r.to, r.from, r.from)
        } else {
            continue;
        };
        let mut right = l.offset_in_book(b);
        if b_end.book == b.book && b_end > b {
            right = (right + l.offset_in_book(b_end)) / 2.0;
        }
        out.push(Link {
            left: l.offset_in_book(a),
            right,
            votes: r.votes,
            label: format!(
                "{} \u{2192} {}{}  ({} votes)",
                verse_label(a),
                verse_label(b),
                if b_end != b && b_end.book == b.book {
                    format!("\u{2013}{}", b_end.verse)
                } else {
                    String::new()
                },
                r.votes
            ),
        });
    }
    out.sort_by_key(|k| k.votes);
    out
}

pub fn render(refs: &[CrossRef], o: &Options) -> String {
    let t = &o.theme;
    let l = Layout::get();
    let links = links(refs, o);
    let max_ch = l.chapters(o.left).max(l.chapters(o.right));
    let axis_h = (max_ch as f64 * 17.0).clamp(760.0, 3400.0);
    let (w, top, bottom) = (1500.0, 220.0, 110.0);
    let h = top + axis_h + bottom;
    let (xl, xr) = (430.0, w - 430.0);
    let (lname, rname) = (bible::name(o.left), bible::name(o.right));

    let mut s = Svg::new(
        w,
        h,
        t,
        &format!("Cross-references between {lname} and {rname}"),
    );
    let (cl, cr) = (t.book(o.left), t.book(o.right));
    let cr = if cl == cr {
        mix(cr, if t.dark { "#ffffff" } else { "#000000" }, 0.35)
    } else {
        cr.to_owned()
    };
    let _ = writeln!(
        s.buf,
        "<defs><linearGradient id=\"link\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"0\" \
         x2=\"{}\" y2=\"0\"><stop offset=\"0\" stop-color=\"{cl}\"/><stop offset=\"1\" \
         stop-color=\"{cr}\"/></linearGradient></defs>",
        n(xl),
        n(xr)
    );

    // Header
    let count = format!(
        "{} cross-reference{}",
        thousands(links.len()),
        if links.len() == 1 { "" } else { "s" }
    );
    s.text(
        w / 2.0,
        62.0,
        &count,
        &format!(
            "font-family=\"{}\" font-size=\"20\" fill=\"{}\" text-anchor=\"middle\"",
            esc(SERIF),
            t.fg
        ),
    );
    s.text(
        w / 2.0,
        86.0,
        &format!(
            "{} chapters \u{00b7} {} verses   \u{2194}   {} chapters \u{00b7} {} verses",
            l.chapters(o.left),
            thousands(l.book_verses(o.left) as usize),
            l.chapters(o.right),
            thousands(l.book_verses(o.right) as usize)
        ),
        &format!(
            "font-size=\"12\" fill=\"{}\" text-anchor=\"middle\" letter-spacing=\"0.4\"",
            t.muted
        ),
    );
    let big = |s: &mut Svg, x: f64, anchor: &str, name: &str, color: &str| {
        s.text(
            x,
            top - 52.0,
            name,
            &format!(
                "font-family=\"{}\" font-size=\"54\" fill=\"{}\" text-anchor=\"{anchor}\"",
                esc(SERIF),
                t.fg
            ),
        );
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"56\" height=\"3\" rx=\"1.5\" fill=\"{color}\"/>",
            n(if anchor == "end" { x - 56.0 } else { x }),
            n(top - 34.0)
        );
    };
    big(&mut s, xl, "end", &lname, cl);
    big(&mut s, xr, "start", &rname, &cr);

    // Links, weakest first so the strongest sit on top.
    let max_votes = links.iter().map(|k| k.votes).max().unwrap_or(1).max(2) as f64;
    let (sl, sr) = (
        axis_h / l.book_verses(o.left) as f64,
        axis_h / l.book_verses(o.right) as f64,
    );
    let dx = (xr - xl) * 0.42;
    s.raw("<g fill=\"none\" stroke=\"url(#link)\" stroke-linecap=\"round\">\n");
    for k in &links {
        let v = (k.votes.max(1) as f64).ln() / max_votes.ln();
        let (y1, y2) = (top + k.left * sl, top + k.right * sr);
        let _ = writeln!(
            s.buf,
            "<path d=\"M{} {}C{} {} {} {} {} {}\" stroke-width=\"{}\" stroke-opacity=\"{}\">\
             <title>{}</title></path>",
            n(xl + 3.0),
            n(y1),
            n(xl + dx),
            n(y1),
            n(xr - dx),
            n(y2),
            n(xr - 3.0),
            n(y2),
            format_args!("{:.2}", 0.45 + 1.6 * v),
            format_args!("{:.2}", if t.dark { 0.16 } else { 0.14 } + 0.55 * v),
            esc(&k.label)
        );
    }
    s.raw("</g>\n");

    axis(&mut s, t, o.left, xl, top, sl, true, cl);
    axis(&mut s, t, o.right, xr, top, sr, false, &cr);

    s.footer(
        t,
        "Cross-references: OpenBible.info (CC BY)  \u{00b7}  Chapter titles: bibleocity.com \
         (CC BY-NC-SA 4.0)  \u{00b7}  Line weight and opacity scale with community votes",
    );
    s.finish()
}

#[allow(clippy::too_many_arguments)]
fn axis(s: &mut Svg, t: &Theme, book: u8, x: f64, top: f64, scale: f64, left: bool, color: &str) {
    let l = Layout::get();
    let chapters = l.chapters(book);
    let dir = if left { -1.0 } else { 1.0 };
    let anchor = if left { "end" } else { "start" };
    let mut mids = Vec::with_capacity(chapters);
    let mut y = top;
    for c in 1..=chapters as u16 {
        let hgt = l.chapter_verses(book, c) as f64 * scale;
        let _ = writeln!(
            s.buf,
            "<rect x=\"{}\" y=\"{}\" width=\"5\" height=\"{}\" fill=\"{color}\" fill-opacity=\"{}\"/>",
            n(if left { x - 5.0 } else { x }),
            n(y + 0.4),
            n((hgt - 0.8).max(0.6)),
            if c % 2 == 1 { "0.95" } else { "0.55" }
        );
        mids.push(y + hgt / 2.0);
        y += hgt;
    }
    let mut pos = mids.clone();
    spread(&mut pos, 15.0, top + 4.0, y);
    // Consecutive chapters sharing a title get one label and a bracket.
    let titles: Vec<Option<String>> = (1..=chapters as u16)
        .map(|c| bible::chapter_title(book, c))
        .collect();
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for i in 0..chapters {
        match runs.last_mut() {
            Some(r) if titles[i].is_some() && titles[i] == titles[r.0] => r.1 = i,
            _ => runs.push((i, i)),
        }
    }
    let mut run_pos: Vec<f64> = runs.iter().map(|&(a, b)| (pos[a] + pos[b]) / 2.0).collect();
    spread(&mut run_pos, 15.0, top + 4.0, y);
    for (&(a, b), &ry) in runs.iter().zip(&run_pos) {
        let Some(title) = &titles[a] else { continue };
        let mut title = title.clone();
        if title.chars().count() > 46 {
            title = title
                .chars()
                .take(45)
                .collect::<String>()
                .trim_end()
                .to_owned()
                + "\u{2026}";
        }
        if b > a {
            let bx = x + dir * 40.0;
            let _ = writeln!(
                s.buf,
                "<path d=\"M{} {}H{}V{}H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.8\"/>",
                n(bx - dir * 3.0),
                n(pos[a]),
                n(bx),
                n(pos[b]),
                n(bx - dir * 3.0),
                t.faint
            );
        }
        s.text(
            x + dir * 50.0,
            ry + 4.0,
            &title,
            &format!(
                "font-size=\"12.5\" fill=\"{}\" text-anchor=\"{anchor}\"",
                t.fg
            ),
        );
    }
    for (i, (&mid, &py)) in mids.iter().zip(&pos).enumerate() {
        let c = i as u16 + 1;
        if (py - mid).abs() > 1.5 {
            let _ = writeln!(
                s.buf,
                "<path d=\"M{} {}L{} {}L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.7\"/>",
                n(x + dir * 9.0),
                n(mid),
                n(x + dir * 15.0),
                n(mid),
                n(x + dir * 22.0),
                n(py),
                t.faint
            );
        }
        s.text(
            x + dir * 26.0,
            py + 3.5,
            &c.to_string(),
            &format!(
                "font-size=\"10\" fill=\"{}\" text-anchor=\"{anchor}\" font-variant-numeric=\"tabular-nums\"",
                t.muted
            ),
        );
    }
}
