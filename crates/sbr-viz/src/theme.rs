//! Colour themes and palettes.

use crate::bible::Genre;

pub const SANS: &str =
    "Inter, 'Segoe UI', 'Helvetica Neue', Helvetica, Arial, 'Liberation Sans', sans-serif";
pub const SERIF: &str = "'Source Serif 4', 'Iowan Old Style', 'Palatino Linotype', Palatino, \
     Georgia, 'Liberation Serif', serif";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub bg: &'static str,
    pub fg: &'static str,
    pub muted: &'static str,
    pub faint: &'static str,
    pub rule: &'static str,
    genres: [&'static str; 10],
    /// Short → long distance ramp for arc diagrams.
    pub ramp: &'static [&'static str],
    /// OT↔OT, OT↔NT, NT↔NT, same book.
    pub quadrants: [&'static str; 4],
}

pub const LIGHT: Theme = Theme {
    name: "light",
    dark: false,
    bg: "#fbfaf6",
    fg: "#1f2328",
    muted: "#7d8590",
    faint: "#c9ccd1",
    rule: "#e6e3dc",
    genres: [
        "#c2452f", "#d9822b", "#b39200", "#5a9a3c", "#16897a", "#2f6fb2", "#5b54c4", "#8c4bb8",
        "#c04a8f", "#9b2c4e",
    ],
    ramp: &[
        "#3a5ba0", "#1f8a9e", "#2e9e6a", "#b59a00", "#e07b28", "#d1403a", "#a1287a",
    ],
    quadrants: ["#2f4fb8", "#7b3fb0", "#c8323c", "#d0287f"],
};

pub const DARK: Theme = Theme {
    name: "dark",
    dark: true,
    bg: "#0b0f17",
    fg: "#e9e7e2",
    muted: "#8b93a1",
    faint: "#3a4250",
    rule: "#1c2330",
    genres: [
        "#ff6b5b", "#ffa94d", "#ffd43b", "#8ce99a", "#38d9a9", "#4dabf7", "#748ffc", "#b197fc",
        "#f783ac", "#ff8787",
    ],
    ramp: &[
        "#3b4cc0", "#3a86ff", "#00c2d1", "#7ae582", "#f9e15b", "#ff9f1c", "#ff4f79", "#e040fb",
    ],
    quadrants: ["#5c7cfa", "#c77dff", "#ff6b6b", "#ff4fd8"],
};

impl Theme {
    pub fn by_name(name: &str) -> Option<Theme> {
        match name {
            "light" | "paper" => Some(LIGHT),
            "dark" | "midnight" => Some(DARK),
            _ => None,
        }
    }

    pub fn genre(&self, g: Genre) -> &'static str {
        self.genres[g as usize]
    }

    pub fn book(&self, book: u8) -> &'static str {
        self.genre(Genre::of(book))
    }

    /// Colour at `t` in `0..=1` along [`Theme::ramp`].
    pub fn ramp_at(&self, t: f64) -> String {
        sample(self.ramp, t)
    }
}

fn rgb(hex: &str) -> [f64; 3] {
    let h = hex.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f64;
    [p(0), p(2), p(4)]
}

fn hex([r, g, b]: [f64; 3]) -> String {
    let c = |x: f64| x.round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b))
}

pub fn mix(a: &str, b: &str, t: f64) -> String {
    let (a, b) = (rgb(a), rgb(b));
    hex([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t))
}

pub fn sample(stops: &[&str], t: f64) -> String {
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
    let i = (t.floor() as usize).min(stops.len() - 2);
    mix(stops[i], stops[i + 1], t - i as f64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn colours() {
        assert_eq!(super::mix("#000000", "#ffffff", 0.5), "#808080");
        assert_eq!(super::sample(&["#000000", "#ff0000"], 1.0), "#ff0000");
        assert_eq!(super::sample(&["#000000", "#ff0000"], 7.0), "#ff0000");
    }
}
