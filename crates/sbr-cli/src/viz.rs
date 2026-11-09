//! `sbr viz <chart> --xrefs <file> [options] -o <out.svg|out.png>`

use sbr_viz::{arc, bible, chord, matrix, pair, xref, Theme};
use std::path::{Path, PathBuf};

pub const USAGE: &str =
    "usage: sbr viz <pair|matrix|arc|chord|gallery> --xrefs <cross_references.txt|.zip>
           [--theme light|dark] [--min-votes N] [--scale S] (-o <out.svg|out.png> | --out-dir DIR)
  pair     --left <book> --right <book>   two books on parallel axes, chapter titles
  matrix   [--detail N]                   66x66 book matrix with N x N position bins (default 5)
  arc      [--max-arcs N]                 all chapters on a line, links as arcs (default 40000)
  chord    [--max-ribbons N]              books on a circle grouped by genre (default 700)
  gallery  --out-dir DIR                  every chart in both themes, as .svg and .png
Cross-references: https://a.openbible.info/data/cross-references.zip (CC BY)";

struct Args {
    chart: String,
    xrefs: Option<PathBuf>,
    out: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    theme: Theme,
    min_votes: i32,
    scale: f32,
    left: u8,
    right: u8,
    detail: usize,
    max_arcs: usize,
    max_ribbons: usize,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let mut a = Args {
        chart: args.first().cloned().ok_or("missing chart")?,
        xrefs: None,
        out: None,
        out_dir: None,
        theme: sbr_viz::LIGHT,
        min_votes: 1,
        scale: 2.0,
        left: 2,
        right: 59,
        detail: 5,
        max_arcs: 40_000,
        max_ribbons: 700,
    };
    let mut it = args[1..].iter();
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("{flag} needs a value"));
        let num = |v: &String| {
            v.parse::<f64>()
                .map_err(|_| format!("bad number for {flag}"))
        };
        let book = |v: &String| bible::parse_book(v).ok_or(format!("unknown book {v:?}"));
        match flag.as_str() {
            "--xrefs" => a.xrefs = Some(val()?.into()),
            "-o" | "--out" => a.out = Some(val()?.into()),
            "--out-dir" => a.out_dir = Some(val()?.into()),
            "--theme" => {
                let v = val()?;
                a.theme = Theme::by_name(v).ok_or(format!("unknown theme {v:?}"))?
            }
            "--min-votes" => a.min_votes = num(val()?)? as i32,
            "--scale" => a.scale = num(val()?)? as f32,
            "--left" | "--from" => a.left = book(val()?)?,
            "--right" | "--to" => a.right = book(val()?)?,
            "--detail" => a.detail = num(val()?)? as usize,
            "--max-arcs" => a.max_arcs = num(val()?)? as usize,
            "--max-ribbons" => a.max_ribbons = num(val()?)? as usize,
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(a)
}

fn render(a: &Args, refs: &[xref::CrossRef], chart: &str, theme: Theme) -> Result<String, String> {
    let min_votes = a.min_votes;
    Ok(match chart {
        "pair" => pair::render(
            refs,
            &pair::Options {
                left: a.left,
                right: a.right,
                min_votes,
                theme,
            },
        ),
        "matrix" => matrix::render(
            refs,
            &matrix::Options {
                min_votes,
                detail: a.detail,
                theme,
            },
        ),
        "arc" => arc::render(
            refs,
            &arc::Options {
                min_votes,
                max_arcs: a.max_arcs,
                theme,
            },
        ),
        "chord" => chord::render(
            refs,
            &chord::Options {
                min_votes,
                max_ribbons: a.max_ribbons,
                theme,
            },
        ),
        _ => return Err(format!("unknown chart {chart:?}")),
    })
}

fn write(svg: &str, out: &Path, scale: f32) -> Result<(), String> {
    let png = out
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    let bytes = if png {
        sbr_viz::to_png(svg, scale)?
    } else {
        svg.as_bytes().to_vec()
    };
    std::fs::write(out, bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    println!("{}", out.display());
    Ok(())
}

pub fn run(args: &[String]) -> Result<(), String> {
    let a = parse(args)?;
    let path = a.xrefs.as_deref().ok_or("--xrefs is required")?;
    let refs = xref::load(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if a.chart == "gallery" {
        let dir = a.out_dir.as_deref().ok_or("gallery needs --out-dir")?;
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let pairs = [(2, 59), (23, 40), (1, 66), (19, 58)];
        for theme in [sbr_viz::LIGHT, sbr_viz::DARK] {
            let mut jobs: Vec<(String, String)> = Vec::new();
            for chart in ["matrix", "arc", "chord"] {
                jobs.push((chart.into(), render(&a, &refs, chart, theme)?));
            }
            for (left, right) in pairs {
                let svg = pair::render(
                    &refs,
                    &pair::Options {
                        left,
                        right,
                        min_votes: a.min_votes,
                        theme,
                    },
                );
                let name = format!(
                    "pair-{}-{}",
                    bible::OSIS[left as usize - 1],
                    bible::OSIS[right as usize - 1]
                );
                jobs.push((name.to_lowercase(), svg));
            }
            for (name, svg) in jobs {
                for ext in ["svg", "png"] {
                    write(
                        &svg,
                        &dir.join(format!("{name}-{}.{ext}", theme.name)),
                        a.scale,
                    )?;
                }
            }
        }
        return Ok(());
    }
    let svg = render(&a, &refs, &a.chart, a.theme)?;
    write(&svg, a.out.as_deref().ok_or("-o is required")?, a.scale)
}
