//! Exportable cross-reference visualizations.
//!
//! Every chart renders to a standalone SVG string (fonts referenced by name,
//! hover titles on marks); with the `png` feature it can be rasterized via
//! `resvg`.

pub mod arc;
pub mod bible;
pub mod chord;
pub mod matrix;
pub mod pair;
pub mod svg;
pub mod theme;
pub mod xref;

pub use theme::{Theme, DARK, LIGHT};

/// Rasterizes an SVG at `scale` × its intrinsic size using system fonts.
#[cfg(feature = "png")]
pub fn to_png(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    use resvg::{tiny_skia, usvg};
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())?;
    let size = tree
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or("invalid scale")?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("image too large")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xref::parse;

    const SAMPLE: &str = "Exod.20.3\tJas.2.10\t12\nJas.1.17\tExod.3.14\t30\n\
                          Gen.1.1\tRev.21.1\t40\nIsa.53.5\tMatt.8.17\t90\n\
                          1Kgs.10.1\t2Chr.9.1\t60\nExod.20.13\tJas.2.11-Jas.2.12\t5\n";

    fn well_formed(svg: &str) {
        sbr_xml::Document::parse(svg).expect("well-formed SVG");
        assert!(svg.starts_with("<svg ") && svg.trim_end().ends_with("</svg>"));
    }

    #[test]
    fn charts_render() {
        let refs = parse(SAMPLE).unwrap();
        for theme in [LIGHT, DARK] {
            let p = pair::render(
                &refs,
                &pair::Options {
                    left: 2,
                    right: 59,
                    min_votes: 1,
                    theme,
                },
            );
            well_formed(&p);
            assert_eq!(
                p.matches("<title>Exod").count() + p.matches("<title>Ex ").count(),
                3
            );
            assert!(p.contains("3 cross-references"));
            well_formed(&matrix::render(
                &refs,
                &matrix::Options {
                    min_votes: 1,
                    detail: 4,
                    theme,
                },
            ));
            well_formed(&arc::render(
                &refs,
                &arc::Options {
                    min_votes: 1,
                    max_arcs: 100,
                    theme,
                },
            ));
            well_formed(&chord::render(
                &refs,
                &chord::Options {
                    min_votes: 1,
                    max_ribbons: 100,
                    theme,
                },
            ));
        }
    }

    #[cfg(feature = "png")]
    #[test]
    fn png_export() {
        let refs = parse(SAMPLE).unwrap();
        let svg = chord::render(
            &refs,
            &chord::Options {
                min_votes: 1,
                max_ribbons: 10,
                theme: LIGHT,
            },
        );
        let png = to_png(&svg, 0.25).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }
}
